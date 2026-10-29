//! DPM/Canton startup, readiness checks, and process-group termination.
//!
//! Each instance keeps its own ports and artifact directories. The manager owns
//! scheduling; this module owns the complete lifetime of each external process,
//! including cleanup after an unsuccessful launch.
use std::{
    fs::File,
    future::Future,
    net::TcpListener,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use daml_lf::dar::DarFile;
use ledger_api_proto::com::daml::ledger::api::v2::{
    GetLedgerApiVersionRequest, GetPackageStatusRequest, PackageStatus,
    package_service_client::PackageServiceClient, version_service_client::VersionServiceClient,
};
use processkit::{ProcessGroup, ProcessGroupOptions};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt},
    process::{Child, Command},
    time::{Instant, sleep},
};
use tokio_util::sync::CancellationToken;

use crate::{Endpoints, Error, Sandbox, SandboxConfig};

pub(crate) type Work<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The only substitution point for deterministic pool tests.
pub(crate) trait Backend: Send + Sync + 'static {
    /// Launch an instance and wait for readiness, observing startup cancellation.
    ///
    /// On error, clean up any partially started process before returning, or
    /// retain the cleanup failure in the returned error. The manager receives no
    /// process handle on this path and cannot perform that teardown itself.
    fn start(
        &self,
        config: Arc<SandboxConfig>,
        cancel: CancellationToken,
    ) -> Work<'static, Result<Box<dyn Running>, Error>>;
}

/// A ready process owned by one manager lifecycle task until shutdown completes.
pub(crate) trait Running: Send {
    /// Metadata handed to every lease on this particular instance.
    fn sandbox(&self) -> Arc<Sandbox>;
    /// Wait for an unexpected exit. Dropping this future must leave the process
    /// available for `stop`, as cancellation races it against normal shutdown.
    fn exited(&mut self) -> Work<'_, Error>;
    /// Terminate the owned process group and reap the child before returning success.
    fn stop(self: Box<Self>) -> Work<'static, Result<(), Error>>;
}

/// Real backend: stages DARs and retries detected port collisions across up to
/// three launch attempts sharing the configured startup deadline.
pub(crate) struct Canton;

impl Backend for Canton {
    fn start(
        &self,
        config: Arc<SandboxConfig>,
        cancel: CancellationToken,
    ) -> Work<'static, Result<Box<dyn Running>, Error>> {
        Box::pin(async move {
            let deadline = Instant::now() + config.startup;
            let directory = config
                .artifacts
                .as_ref()
                .expect("artifact directory resolved during pool initialization")
                .join(format!("{}-{}", std::process::id(), uuid::Uuid::new_v4()));
            let make_error = |message: String| Error::Startup {
                message,
                artifacts: directory.clone(),
            };
            let prepare = async {
                tokio::fs::create_dir_all(&directory)
                    .await
                    .map_err(|e| make_error(e.to_string()))?;
                let mut dars = Vec::new();
                for (index, bytes) in config.dars.iter().enumerate() {
                    let path = directory.join(format!("fixture-{index}.dar"));
                    tokio::fs::write(&path, bytes)
                        .await
                        .map_err(|e| make_error(e.to_string()))?;
                    let input = path.clone();
                    let package = tokio::task::spawn_blocking(move || {
                        DarFile::read_from(&input)
                            .and_then(|mut dar| dar.main_dalf())
                            .map(|dalf| dalf.hash().to_string())
                            .map_err(|e| format!("{}: {e}", input.display()))
                    })
                    .await
                    .map_err(|e| make_error(e.to_string()))?
                    .map_err(make_error)?;
                    dars.push((path, package));
                }
                Ok::<_, Error>(dars)
            };
            let dars = tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(Error::Closed),
                result = tokio::time::timeout_at(deadline, prepare) => result.map_err(|_| make_error("startup deadline expired preparing DARs".into()))??,
            };

            for attempt in 1..=3 {
                if cancel.is_cancelled() {
                    return Err(Error::Closed);
                }
                if Instant::now() >= deadline {
                    return Err(make_error("startup deadline expired".into()));
                }
                let ports = Ports::reserve().map_err(|e| make_error(e.to_string()))?;
                let attempt_directory = directory.join(format!("attempt-{attempt}"));
                tokio::fs::create_dir(&attempt_directory)
                    .await
                    .map_err(|e| make_error(e.to_string()))?;
                let log = attempt_directory.join("sandbox.log");
                let port_file = attempt_directory.join("ports.json");
                let info = Arc::new(Sandbox {
                    id: directory
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    endpoints: ports.endpoints(),
                    artifacts: attempt_directory.clone(),
                });
                let mut command = Command::new(&config.dpm);
                command
                    .current_dir(&attempt_directory)
                    .env_remove("DAML_PACKAGE")
                    .env_remove("DAML_PROJECT")
                    .args(["component", "run", "canton-open-source"])
                    .arg(&config.canton_version)
                    .arg("--")
                    .args(["sandbox", "--nuck", "--canton-port-file"])
                    .arg(&port_file)
                    .args(["--log-file-appender", "flat", "--log-file-name"])
                    .arg(attempt_directory.join("canton.log"))
                    .args(["--log-level-canton", "DEBUG", "--log-level-stdout", "INFO"]);
                for (path, _) in &dars {
                    command.arg("--dar").arg(path);
                }
                ports.configure(&mut command);
                // Keep all reservations alive until the last possible moment.
                drop(ports);
                let mut process = Process::spawn(command, info, &log, config.shutdown)
                    .map_err(|e| make_error(e.to_string()))?;
                let ready = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => Err(Error::Closed),
                    result = process.ready(&port_file, &dars, deadline) => result,
                };
                if ready.is_ok() {
                    return Ok(Box::new(process) as Box<dyn Running>);
                }
                let primary = ready.unwrap_err();
                let cleanup = Box::new(process).stop().await;
                let details = log_tail(&log).await;
                let collision = matches!(&primary, Error::ProcessExit { .. })
                    && details.contains("Address already in use");
                if cleanup.is_ok() && collision && attempt < 3 {
                    continue;
                }
                let primary = match primary {
                    Error::Closed => Error::Closed,
                    other => make_error(format!("{other}\n{details}")),
                };
                return Err(Error::combine(primary, cleanup));
            }
            unreachable!("the last attempt always returns")
        })
    }
}

/// One launched child, its containment group, and the files needed to diagnose it.
pub(crate) struct Process {
    group: ProcessGroup,
    child: Child,
    info: Arc<Sandbox>,
    shutdown: Duration,
}

impl Process {
    /// Launch a child in a managed process group with stdout and stderr captured
    /// together in the supplied log file. The caller is responsible for readiness.
    pub(crate) fn spawn(
        mut command: Command,
        info: Arc<Sandbox>,
        log: &Path,
        shutdown: Duration,
    ) -> Result<Self, Error> {
        let output = File::create(log).map_err(|e| Error::Startup {
            message: e.to_string(),
            artifacts: info.artifacts.clone(),
        })?;
        command
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|e| Error::Startup {
                message: e.to_string(),
                artifacts: info.artifacts.clone(),
            })?)
            .stderr(output)
            .process_group(0);
        let group = ProcessGroup::with_options(
            ProcessGroupOptions::default()
                .shutdown_timeout(shutdown)
                .escalate_to_kill(true),
        )
        .map_err(|e| Error::Startup {
            message: e.to_string(),
            artifacts: info.artifacts.clone(),
        })?;
        let child = group.spawn(command).map_err(|e| Error::Startup {
            message: e.to_string(),
            artifacts: info.artifacts.clone(),
        })?;
        Ok(Self {
            group,
            child,
            info,
            shutdown,
        })
    }

    /// Wait for a matching port file, a successful Ledger API version request,
    /// and registration of every supplied DAR's main package. Exit is checked
    /// alongside readiness so a dead child fails setup without spending the budget.
    async fn ready(
        &mut self,
        port_file: &Path,
        dars: &[(PathBuf, String)],
        deadline: Instant,
    ) -> Result<(), Error> {
        loop {
            if Instant::now() >= deadline {
                return Err(Error::Startup {
                    message: "startup deadline expired".into(),
                    artifacts: self.info.artifacts.clone(),
                });
            }
            let ready = async {
                if !ports_ready(port_file, &self.info.endpoints.ledger_api).await? {
                    return Ok(false);
                }
                let check = async {
                    let channel = tonic::transport::Endpoint::from_shared(
                        self.info.endpoints.ledger_api.clone(),
                    )?
                    .connect()
                    .await?;
                    VersionServiceClient::new(channel.clone())
                        .get_ledger_api_version(GetLedgerApiVersionRequest {})
                        .await?;
                    let mut packages = PackageServiceClient::new(channel);
                    for (_, package_id) in dars {
                        let status = packages
                            .get_package_status(GetPackageStatusRequest {
                                package_id: package_id.clone(),
                            })
                            .await?
                            .into_inner();
                        if status.package_status != PackageStatus::Registered as i32 {
                            return Ok(false);
                        }
                    }
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(true)
                };
                Ok(tokio::time::timeout_at(
                    deadline.min(Instant::now() + Duration::from_secs(5)),
                    check,
                )
                .await
                .is_ok_and(|result| result.unwrap_or(false)))
            };
            tokio::select! {
                biased;
                status = self.child.wait() => return Err(self.exit_error(format!("{status:?}"))),
                result = ready => match result {
                    Ok(true) => return Ok(()),
                    Ok(false) => (),
                    Err(message) => return Err(Error::Startup { message, artifacts: self.info.artifacts.clone() }),
                },
            }
            tokio::select! {
                status = self.child.wait() => return Err(self.exit_error(format!("{status:?}"))),
                _ = sleep(Duration::from_millis(250)) => (),
            }
        }
    }

    fn exit_error(&self, message: String) -> Error {
        Error::ProcessExit {
            message,
            artifacts: self.info.artifacts.clone(),
        }
    }
}

impl Running for Process {
    fn sandbox(&self) -> Arc<Sandbox> {
        self.info.clone()
    }

    fn exited(&mut self) -> Work<'_, Error> {
        Box::pin(async {
            let status = self.child.wait().await;
            self.exit_error(format!("{status:?}"))
        })
    }

    fn stop(mut self: Box<Self>) -> Work<'static, Result<(), Error>> {
        Box::pin(async move {
            let cleanup = async {
                let (group, child) = tokio::join!(self.group.shutdown_ref(), self.child.wait());
                let group = group
                    .map_err(|e| Error::Cleanup(format!("{}: {e}", self.info.artifacts.display())));
                let child = child
                    .map(|_| ())
                    .map_err(|e| Error::Cleanup(format!("reaping child: {e}")));
                match (group, child) {
                    (Ok(()), result) => result,
                    (Err(primary), cleanup) => Err(Error::combine(primary, cleanup)),
                }
            };
            match tokio::time::timeout(self.shutdown + Duration::from_secs(5), cleanup).await {
                Ok(result) => result,
                Err(_) => {
                    // Drop still kills the containment group; keep the pool closed
                    // because we could not confirm teardown within the deadline.
                    let _ = self.group.kill_all();
                    Err(Error::Cleanup(format!(
                        "termination/reaping deadline expired; artifacts: {}",
                        self.info.artifacts.display()
                    )))
                }
            }
        })
    }
}

/// Six reserved loopback ports held until immediately before spawning Canton.
///
/// Releasing the listeners leaves a small bind race; startup retries detected
/// address collisions rather than treating a reservation as a permanent guarantee.
struct Ports {
    ports: [u16; 6],
    _listeners: Vec<TcpListener>,
}

impl Ports {
    fn reserve() -> std::io::Result<Self> {
        let mut listeners = Vec::new();
        let mut ports = [0; 6];
        for port in &mut ports {
            let listener = TcpListener::bind(("127.0.0.1", 0))?;
            *port = listener.local_addr()?.port();
            listeners.push(listener);
        }
        Ok(Self {
            ports,
            _listeners: listeners,
        })
    }

    fn configure(&self, command: &mut Command) {
        for (flag, port) in [
            "--ledger-api-port",
            "--admin-api-port",
            "--json-api-port",
            "--sequencer-public-port",
            "--sequencer-admin-port",
            "--mediator-admin-port",
        ]
        .iter()
        .zip(self.ports)
        {
            command.arg(flag).arg(port.to_string());
        }
    }

    fn endpoints(&self) -> Endpoints {
        let endpoint = |index| format!("http://127.0.0.1:{}", self.ports[index]);
        Endpoints {
            ledger_api: endpoint(0),
            admin_api: endpoint(1),
            json_api: endpoint(2),
            sequencer_public: endpoint(3),
            sequencer_admin: endpoint(4),
            mediator_admin: endpoint(5),
        }
    }
}

/// Treat missing or incomplete port files as pending startup. A reported port
/// that differs from the assigned Ledger API endpoint is a fatal mismatch.
async fn ports_ready(path: &Path, endpoint: &str) -> Result<bool, String> {
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    let port = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| value["sandbox"]["ledgerApi"].as_u64());
    match port {
        None => Ok(false),
        Some(port) if endpoint == format!("http://127.0.0.1:{port}") => Ok(true),
        Some(port) => Err(format!(
            "ports file reports unexpected Ledger API port {port}"
        )),
    }
}

/// Read a bounded log suffix for a startup error, retaining the path even when
/// the log cannot be read.
async fn log_tail(path: &Path) -> String {
    const LIMIT: u64 = 12_000;
    let read = async {
        let mut file = tokio::fs::File::open(path).await?;
        let length = file.metadata().await?.len();
        file.seek(std::io::SeekFrom::Start(length.saturating_sub(LIMIT)))
            .await?;
        let mut bytes = Vec::new();
        file.take(LIMIT).read_to_end(&mut bytes).await?;
        Ok::<_, std::io::Error>(bytes)
    };
    match read.await {
        Ok(bytes) => format!(
            "log: {}\n{}",
            path.display(),
            String::from_utf8_lossy(&bytes)
        ),
        Err(error) => format!("log: {} (unavailable: {error})", path.display()),
    }
}
