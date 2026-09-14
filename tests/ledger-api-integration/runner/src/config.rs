use chrono::Utc;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
};
use tokio::process::Command;

pub(crate) const SDK: &str = "3.6.1-snapshot.20260924.39.0.v85561690";
const SUITE: &str = "ledger-api-integration";

/// Paths and command settings are fixed for the duration of a run.
pub(crate) struct Config {
    workspace: PathBuf,
    pub(crate) logs: PathBuf,
    pub(crate) dar: PathBuf,
    pub(crate) probe: PathBuf,
    cargo_target: PathBuf,
    dpm: OsString,
    color: ColorMode,
}

impl Config {
    pub(crate) fn new() -> io::Result<Self> {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("runner belongs to the integration workspace")
            .to_path_buf();
        let repo = workspace
            .parent()
            .and_then(Path::parent)
            .expect("repository root");
        let artifacts = repo.join("target/ledger-api-integration");
        let stamp = format!(
            "{}-{}",
            Utc::now().format("%Y%m%dT%H%M%S%.6fZ"),
            std::process::id()
        );
        let logs = artifacts.join("runs").join(stamp);
        fs::create_dir_all(logs.parent().expect("run directory parent"))?;
        fs::create_dir(&logs)?;
        let cargo_target = artifacts.join("cargo");

        Ok(Self {
            dar: logs.join("fixtures.dar"),
            probe: cargo_target.join("debug/probe"),
            workspace,
            logs,
            cargo_target,
            dpm: env::var_os("DPM").unwrap_or_else(|| "dpm".into()),
            color: ColorMode::detect(),
        })
    }

    /// Inherit the caller's environment and override only fixture/run settings.
    pub(crate) fn command(&self, program: impl AsRef<OsStr>) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.workspace)
            .env("DAML_PACKAGE", self.workspace.join("fixtures"))
            .env("CARGO_TARGET_DIR", &self.cargo_target)
            .env("CANTON_TEST_DAR", &self.dar);
        command
    }

    pub(crate) fn dpm_command(&self) -> Command {
        self.command(&self.dpm)
    }

    pub(crate) fn fixture_build(&self) -> Command {
        let mut command = self.dpm_command();
        command.args(["build", "--output"]).arg(&self.dar);
        command
    }

    fn cargo_test(&self) -> Command {
        let mut command = self.command("cargo");
        command.args([
            "test",
            "--locked",
            "--color",
            self.color.flag(),
            "-p",
            SUITE,
        ]);
        command
    }

    pub(crate) fn suite_build(&self) -> Command {
        let mut command = self.cargo_test();
        command.arg("--no-run");
        command
    }

    pub(crate) fn suite_tests(&self, filters: &[OsString], endpoint: &str) -> Command {
        let mut command = self.command("cargo");
        self.color.configure_terminal(&mut command);
        command
            .env("CANTON_TEST_ENDPOINT", endpoint)
            .args([
                "test",
                "--locked",
                "--color",
                self.color.flag(),
                "-p",
                SUITE,
            ])
            .args(["--test", "participant"])
            .args(filters)
            .args([
                "--",
                "--color",
                self.color.flag(),
                "--test-threads=1",
                "--nocapture",
            ]);
        command
    }
}

enum ColorMode {
    Always,
    Never,
}

impl ColorMode {
    fn detect() -> Self {
        if io::stdout().is_terminal() {
            Self::Always
        } else {
            Self::Never
        }
    }

    fn flag(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    fn configure_terminal(&self, command: &mut Command) {
        if matches!(self, Self::Never) {
            return;
        }
        // Cargo can replace TERM with "dumb", including before launching us.
        // Libtest still needs a color-capable terminfo entry with --color always.
        let terminal = env::var("TERM")
            .ok()
            .filter(|terminal| !terminal.is_empty() && terminal != "dumb")
            .unwrap_or_else(|| "xterm".into());
        let terminal = serde_json::to_string(&terminal).expect("serialize terminal name");
        command
            .arg("--config")
            .arg(format!("env.TERM.value={terminal}"))
            .args(["--config", "env.TERM.force=true"]);
    }
}
