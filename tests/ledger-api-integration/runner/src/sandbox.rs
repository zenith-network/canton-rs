//! Start Canton with reserved ports and verify readiness through the API probe.

use std::{
    io,
    net::TcpListener,
    path::{Path, PathBuf},
    process::ExitStatus,
    time::Duration,
};

use tokio::{fs, process::Command, time::Instant};

use crate::{
    cancellation::Cancellation,
    config::{Config, SDK},
    error::{Failure, Result, after_cleanup},
    process::{LogMode, Process},
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(120);
const PROBE_TIMEOUT: Duration = Duration::from_secs(6);
const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_ATTEMPTS: usize = 3;
const PORT_FLAGS: [&str; 6] = [
    "--ledger-api-port",
    "--admin-api-port",
    "--json-api-port",
    "--sequencer-public-port",
    "--sequencer-admin-port",
    "--mediator-admin-port",
];

/// Own the live participant until the test stage has finished.
pub(crate) struct Sandbox {
    pub(crate) endpoint: String,
    process: Process,
}

impl Sandbox {
    pub(crate) async fn shutdown(self) -> Result<()> {
        self.process.finish().await
    }
}

pub(crate) async fn start(config: &Config, cancellation: &Cancellation) -> Result<Sandbox> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    for number in 1..=MAX_ATTEMPTS {
        cancellation.check()?;
        if Instant::now() >= deadline {
            return Err(Failure::Message(
                "Canton was not ready within 120 seconds".into(),
            ));
        }
        let attempt = Attempt::new(config, number)?;
        println!(
            "Starting Canton {SDK} at {} (attempt {number})",
            attempt.endpoint
        );
        let mut process = Process::spawn_logged(
            attempt.command(config),
            &attempt.log,
            LogMode::Overwrite,
            cancellation,
        )?;
        let outcome = attempt
            .wait_ready(config, cancellation, &mut process, deadline)
            .await;
        if matches!(outcome, Ok(Startup::Ready)) {
            println!("Ledger API is ready and the fixture package is registered.");
            return Ok(Sandbox {
                endpoint: attempt.endpoint,
                process,
            });
        }
        // Finalize failed attempts before either retrying or returning an error.
        let cleanup = process.finish().await;
        let retry = match outcome {
            Ok(Startup::Exited(status)) => {
                attempt
                    .retry_after_exit(status, number < MAX_ATTEMPTS)
                    .await
            }
            Err(error) => Err(error),
            Ok(Startup::Ready) => unreachable!("ready participant returned above"),
        };
        after_cleanup(retry, cleanup)?;
    }
    Err(Failure::Message(
        "Canton could not acquire its ports after three attempts".into(),
    ))
}

struct Ports([u16; 6]);

impl Ports {
    fn reserve() -> io::Result<Self> {
        let mut reservations = Vec::new();
        let mut ports = [0; 6];
        for port in &mut ports {
            let listener = TcpListener::bind(("127.0.0.1", 0))?;
            *port = listener.local_addr()?.port();
            reservations.push(listener);
        }
        // Hold all six simultaneously so no two services receive the same port.
        // Release them together just before Canton is spawned.
        Ok(Self(ports))
    }

    fn ledger_api(&self) -> u16 {
        self.0[0]
    }

    fn configure(&self, command: &mut Command) {
        for (flag, port) in PORT_FLAGS.iter().zip(self.0) {
            command.arg(flag).arg(port.to_string());
        }
    }
}

struct Attempt {
    ports: Ports,
    port_file: PathBuf,
    log: PathBuf,
    canton_log: PathBuf,
    endpoint: String,
}

enum Startup {
    Ready,
    Exited(ExitStatus),
}

impl Attempt {
    fn new(config: &Config, number: usize) -> io::Result<Self> {
        let ports = Ports::reserve()?;
        Ok(Self {
            endpoint: format!("http://127.0.0.1:{}", ports.ledger_api()),
            ports,
            port_file: config.logs.join(format!("ports-{number}.json")),
            log: config.logs.join(format!("sandbox-{number}.log")),
            canton_log: config.logs.join(format!("canton-{number}.log")),
        })
    }

    fn command(&self, config: &Config) -> Command {
        let mut command = config.dpm_command();
        command
            .current_dir(&config.logs)
            .env("CANTON_TEST_ENDPOINT", &self.endpoint)
            .args(["sandbox", "--nuck", "--dar"])
            .arg(&config.dar)
            .arg("--canton-port-file")
            .arg(&self.port_file)
            .args(["--log-file-appender", "flat", "--log-file-name"])
            .arg(&self.canton_log)
            .args(["--log-level-canton", "DEBUG", "--log-level-stdout", "INFO"]);
        self.ports.configure(&mut command);
        command
    }

    /// Only a confirmed bind collision can request another startup attempt.
    async fn retry_after_exit(&self, status: ExitStatus, can_retry: bool) -> Result<()> {
        let bytes = fs::read(&self.log).await?;
        let details = String::from_utf8_lossy(&bytes);
        if can_retry && details.contains("Address already in use") {
            return Ok(());
        }
        // Limit diagnostics without splitting UTF-8 characters.
        let tail_start = details
            .char_indices()
            .rev()
            .nth(11999)
            .map_or(0, |(offset, _)| offset);
        Err(Failure::Message(format!(
            "Canton exited with {status}:\n{}",
            &details[tail_start..]
        )))
    }

    async fn wait_ready(
        &self,
        config: &Config,
        cancellation: &Cancellation,
        sandbox: &mut Process,
        deadline: Instant,
    ) -> Result<Startup> {
        loop {
            cancellation.check()?;
            if Instant::now() >= deadline {
                return Err(Failure::Message(format!(
                    "Canton was not ready within 120 seconds; see {}",
                    self.log.display()
                )));
            }
            if ports_file_ready(&self.port_file, self.ports.ledger_api()).await? {
                let mut command = config.command(&config.probe);
                command.env("CANTON_TEST_ENDPOINT", &self.endpoint);
                let mut probe = Process::spawn_logged(
                    command,
                    &config.logs.join("probe.log"),
                    LogMode::Append,
                    cancellation,
                )?;
                // The probe's RPC has a five-second budget. Also bound the
                // process itself, allowing one second for startup and exit.
                let probe_deadline = deadline.min(Instant::now() + PROBE_TIMEOUT);
                let status = probe.wait(cancellation, Some(probe_deadline)).await;
                let status = after_cleanup(status, probe.finish().await)?;
                if status.is_some_and(|status| status.success()) {
                    return Ok(Startup::Ready);
                }
            }
            // Wait for exit or cancellation as well as the next readiness
            // check, rather than polling child status or blocking a thread.
            let next_check = deadline.min(Instant::now() + POLL_INTERVAL);
            if let Some(status) = sandbox.wait(cancellation, Some(next_check)).await? {
                return Ok(Startup::Exited(status));
            }
        }
    }
}

async fn ports_file_ready(path: &Path, expected: u16) -> Result<bool> {
    let bytes = match fs::read(path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    // The file may still be partially written; wait for complete JSON.
    let recorded = serde_json::from_slice::<serde_json::Value>(&bytes)
        .ok()
        .and_then(|value| value["sandbox"]["ledgerApi"].as_u64());
    match recorded {
        None => Ok(false),
        Some(port) if port == u64::from(expected) => Ok(true),
        Some(port) => Err(Failure::Message(format!(
            "Unexpected Ledger API port in ports file: {port}"
        ))),
    }
}
