use super::*;
use crate::backend::Process;
use std::{path::Path, process::Stdio};

struct Shell {
    directory: PathBuf,
    name: &'static str,
}

impl Backend for Shell {
    fn start(
        &self,
        config: Arc<SandboxConfig>,
        _cancel: CancellationToken,
    ) -> Work<'static, Result<Box<dyn Running>, Error>> {
        let directory = self.directory.clone();
        let name = self.name;
        Box::pin(async move {
            std::fs::create_dir_all(&directory).unwrap();
            let mut command = tokio::process::Command::new("sh");
            command.args(["-c", "trap '' TERM; sleep 30 & child=$!; printf '%s %s\\n' \"$$\" \"$child\" > \"$1\"; wait \"$child\"", "sandbox-test"])
                .arg(directory.join(format!("{name}.pids")));
            Ok(Box::new(Process::spawn(
                command,
                info(name.into(), directory.clone()),
                &directory.join(format!("{name}.log")),
                config.shutdown,
            )?) as Box<dyn Running>)
        })
    }
}

#[test]
#[ignore = "internal signal-test helper; run through signals_stop_every_pool_and_their_descendants"]
fn signal_child_entry() {
    let directory: PathBuf = std::env::var_os("SANDBOX_SIGNAL_TEST_DIR")
        .expect("internal subprocess only")
        .into();
    let first = Pool::with_backend(
        SandboxConfig {
            shutdown: Duration::from_millis(100),
            ..test_config()
        },
        Arc::new(Shell {
            directory: directory.clone(),
            name: "first",
        }),
    );
    let second = Pool::with_backend(
        SandboxConfig {
            shutdown: Duration::from_millis(100),
            ..test_config()
        },
        Arc::new(Shell {
            directory,
            name: "second",
        }),
    );
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let _first = first.shared().await.unwrap();
            let _second = second.fresh().await.unwrap();
            std::future::pending::<()>().await;
        });
}

fn running(pid: u32) -> bool {
    let output = std::process::Command::new("ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .unwrap();
    let state = String::from_utf8_lossy(&output.stdout);
    // Killed orphan grandchildren can briefly remain as zombies until reaped
    // by the OS's parent process. They are not running sandbox processes.
    let state = state.trim();
    !state.is_empty() && !state.starts_with('Z')
}

fn read_pids(directory: &Path) -> Option<Vec<u32>> {
    let mut pids = Vec::new();
    for name in ["first", "second"] {
        let text = std::fs::read_to_string(directory.join(format!("{name}.pids"))).ok()?;
        let pair: Vec<u32> = text
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        if pair.len() != 2 {
            return None;
        }
        pids.extend(pair);
    }
    Some(pids)
}

#[test]
fn signals_stop_every_pool_and_their_descendants() {
    for (signal, code) in [(libc::SIGINT, 130), (libc::SIGTERM, 143)] {
        let directory = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "manager::tests::process::signal_child_entry",
                "--nocapture",
            ])
            .env("SANDBOX_SIGNAL_TEST_DIR", directory.path())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let pids = loop {
            if let Some(pids) = read_pids(directory.path()) {
                break pids;
            }
            if let Some(status) = child.try_wait().unwrap() {
                panic!("helper exited before readiness: {status}");
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("signal helper did not become ready");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        // SAFETY: this PID is our live child, which has not yet been reaped.
        assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
        // A repeated signal must not bypass the first signal's cleanup.
        std::thread::sleep(Duration::from_millis(20));
        unsafe {
            libc::kill(child.id() as i32, signal);
        }
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "interrupted helper did not finish cleanup: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(code));
        for pid in pids {
            until_stopped(pid);
        }
    }
}

fn until_stopped(pid: u32) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while running(pid) {
        assert!(
            std::time::Instant::now() < deadline,
            "managed child {pid} remains running"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
