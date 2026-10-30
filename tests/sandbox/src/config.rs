//! Construction of immutable settings before any manager or process is started.
//!
//! The builder holds optional overrides. `build` fills environment defaults,
//! resolves explicit paths, and checks invariants. The pool resolves an omitted
//! artifact directory using the test's Cargo directory or the system temporary directory.
use std::{
    ffi::OsString, num::NonZeroUsize, path::PathBuf, str::FromStr, sync::Arc, time::Duration,
};

use crate::Error;

/// Default capacity of the sandboxes pool.
pub const DEFAULT_CAPACITY: NonZeroUsize = NonZeroUsize::new(2).unwrap();

/// Shared-instance retention when neither the builder nor the environment overrides it.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(3);

/// Validated process-wide settings for sandbox tests.
///
/// Construct with [`Self::builder`]. Return this from a `config = path` provider
/// on the sandbox attribute. Explicit configurations within a process must agree.
/// Building or cloning a config does not start Canton or create another pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SandboxConfig {
    pub(crate) dpm: OsString,
    pub(crate) dars: Vec<Arc<[u8]>>,
    pub(crate) artifacts: Option<PathBuf>,
    /// Capacity includes instances starting up and shutting down.
    pub(crate) capacity: NonZeroUsize,
    /// Keep shared instances alive briefly after their last release for reuse.
    pub(crate) idle: Duration,
    /// Deadline for preparing DARs, launching Canton, and confirming readiness.
    pub(crate) startup: Duration,
    /// Grace period before force-killing the process group. Another five seconds
    /// is allowed to confirm termination and reap the child.
    pub(crate) shutdown: Duration,
}

impl SandboxConfig {
    /// Start configuring a sandbox harness with the standard defaults.
    ///
    /// Environment variables and relative paths are evaluated when
    /// [`SandboxConfigBuilder::build`] is called.
    pub fn builder() -> SandboxConfigBuilder {
        SandboxConfigBuilder::default()
    }

    /// Resolve the artifact default once, before handing settings to the backend.
    pub(crate) fn resolve_artifacts(&mut self, tmpdir: Option<&str>) -> Result<(), Error> {
        if self.artifacts.is_none() {
            let mut directory = tmpdir
                .map_or_else(std::env::temp_dir, PathBuf::from)
                .join("canton-rs-sandbox");
            if directory.is_relative() {
                directory = std::env::current_dir()
                    .map_err(|error| Error::Configuration(error.to_string()))?
                    .join(directory);
            }
            self.artifacts = Some(directory);
        }
        Ok(())
    }
}

/// Builder for [`SandboxConfig`].
///
/// ```no_run
/// use sandbox::SandboxConfig;
/// use std::time::Duration;
///
/// let config = SandboxConfig::builder()
///     .max_sandboxes(3)
///     .idle_timeout(Duration::from_secs(1))
///     .build()?;
/// # Ok::<(), sandbox::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct SandboxConfigBuilder {
    dpm: Option<OsString>,
    dars: Vec<Arc<[u8]>>,
    artifacts: Option<PathBuf>,
    capacity: Option<usize>,
    idle: Option<Duration>,
    startup: Duration,
    shutdown: Duration,
}

impl Default for SandboxConfigBuilder {
    fn default() -> Self {
        Self {
            dpm: None,
            dars: Vec::new(),
            artifacts: None,
            capacity: None,
            idle: None,
            startup: Duration::from_secs(120),
            shutdown: Duration::from_secs(15),
        }
    }
}

impl SandboxConfigBuilder {
    /// Provide DAR file bytes.
    ///
    /// Repeated calls add archives. Parsing and registration happen during
    /// startup, and readiness requires each archive's main package to be registered.
    pub fn with_dar(mut self, bytes: impl Into<Arc<[u8]>>) -> Self {
        self.dars.push(bytes.into());
        self
    }

    /// Set path to DPM executable.
    ///
    /// Environment variable `DPM` takes precedence over this. If none of them set, defaults to
    /// `dpm`.
    pub fn dpm_executable(mut self, executable: impl Into<OsString>) -> Self {
        self.dpm = Some(executable.into());
        self
    }

    /// Set parent directory for sandbox logs and staged DARs.
    ///
    /// If not set, artifacts will be placed under `CARGO_TARGET_TMPDIR`. If it is unset, default
    /// system temp directory (`/tmp`) will be used.
    pub fn artifacts_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.artifacts = Some(directory.into());
        self
    }

    /// Set the capacity of the sandboxes pool.
    ///
    /// Environment variable `SANDBOX_CAPACITY` takes precedence over this. If none of them set,
    /// defaults to [`DEFAULT_CAPACITY`].
    ///
    /// Must be non-zero.
    pub fn max_sandboxes(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// Set the time to wait before destroying an idle sandbox.
    ///
    /// If during this time some other test of the same group will acquire the sandbox, destroy
    /// will be discarded.
    ///
    /// Environment variable `SANDBOX_IDLE_TIMEOUT_MS` takes precedence over this. If none of them
    /// set, defaults to [`DEFAULT_IDLE_TIMEOUT`].
    ///
    /// If set to zero, idle sandboxes will be destroyed immediately after test is completed. This
    /// may be undesired, because launching a new sandbox is costly.
    pub fn idle_timeout(mut self, duration: Duration) -> Self {
        self.idle = Some(duration);
        self
    }

    /// Set the startup deadline (defaults to 120 seconds).
    ///
    /// The startup time covers DAR preparation, launch retries, and Ledger API readiness. It starts
    /// after a capacity slot is assigned. Failed startups still run cleanup, whose shutdown timeout
    /// is separate.
    ///
    /// Must be non-zero.
    pub fn startup_timeout(mut self, duration: Duration) -> Self {
        self.startup = duration;
        self
    }

    /// Set the graceful shutdown period (defaults to 15 seconds).
    ///
    /// Cleanup asks the process group to terminate, then force-kills survivors after this period.
    /// Another five seconds is allowed to confirm termination and reap the child.
    // TODO: describe what happens when set to zero.
    pub fn shutdown_timeout(mut self, duration: Duration) -> Self {
        self.shutdown = duration;
        self
    }

    /// Build sandbox config, returning error on invalid configuration.
    ///
    /// Reads environment variables, which may override provided settings.
    pub fn build(self) -> Result<SandboxConfig, Error> {
        let capacity = match environment_value::<NonZeroUsize>("SANDBOX_CAPACITY")? {
            Some(capacity) => capacity,
            None => NonZeroUsize::new(self.capacity.unwrap_or(DEFAULT_CAPACITY.get()))
                .ok_or_else(|| Error::Configuration("sandbox capacity must be positive".into()))?,
        };

        let idle = environment_value::<u64>("SANDBOX_IDLE_TIMEOUT_MS")?
            .map(Duration::from_millis)
            .or(self.idle)
            .unwrap_or(DEFAULT_IDLE_TIMEOUT);

        let dpm = std::env::var_os("DPM")
            .or(self.dpm)
            .unwrap_or_else(|| "dpm".into());
        if dpm.is_empty() {
            return Err(Error::Configuration(
                "DPM executable must not be empty".into(),
            ));
        }

        if self.startup.is_zero() {
            return Err(Error::Configuration(
                "startup timeout must be positive".into(),
            ));
        }
        Ok(SandboxConfig {
            dpm,
            capacity,
            idle,
            dars: self.dars,
            artifacts: self.artifacts,
            startup: self.startup,
            shutdown: self.shutdown,
        })
    }
}

fn environment_value<T: FromStr>(name: &str) -> Result<Option<T>, Error>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|error| Error::Configuration(format!("{name}={value:?}: {error}"))),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(Error::Configuration(format!("{name}: {error}"))),
    }
}

#[cfg(test)]
pub(crate) fn test_config() -> SandboxConfig {
    // Lifecycle tests need fixed settings, independent of the user's environment.
    SandboxConfig {
        dpm: "dpm".into(),
        dars: Vec::new(),
        artifacts: None,
        capacity: DEFAULT_CAPACITY,
        idle: Duration::ZERO,
        startup: Duration::from_secs(120),
        shutdown: Duration::from_secs(15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn explicit_builder() -> SandboxConfigBuilder {
        SandboxConfig::builder()
            .dpm_executable("code-dpm")
            .max_sandboxes(7)
            .idle_timeout(Duration::from_millis(321))
    }

    #[test]
    fn builder_defaults_and_explicit_settings() {
        check_environment("defaults", &[]);
        check_environment("explicit", &[]);
    }

    #[test]
    fn builder_validates_settings_and_preserves_supplied_values() {
        check_environment("settings", &[]);
    }

    fn check_environment(case: &str, environment: &[(&str, &str)]) {
        // Give every case its own environment, without mutating
        // environment variables underneath concurrent Rust tests.
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", "config::tests::environment_child"])
            .env("SANDBOX_ENV_TEST_CASE", case)
            .env_remove("DPM")
            .env_remove("SANDBOX_CAPACITY")
            .env_remove("SANDBOX_IDLE_TIMEOUT_MS")
            .envs(environment.iter().copied());
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{case}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn environment_overrides_builder_settings_independently() {
        let environment = [
            ("DPM", "env-dpm"),
            ("SANDBOX_CAPACITY", "4"),
            ("SANDBOX_IDLE_TIMEOUT_MS", "1250"),
        ];
        check_environment("overrides", &environment);
        check_environment("invalid_builder", &environment);
        check_environment("dpm_override", &[("DPM", "env-dpm")]);
        check_environment("capacity_override", &[("SANDBOX_CAPACITY", "4")]);
        check_environment("idle_override", &[("SANDBOX_IDLE_TIMEOUT_MS", "1250")]);
        check_environment("zero_idle", &[("SANDBOX_IDLE_TIMEOUT_MS", "0")]);
    }

    #[test]
    fn invalid_environment_fails_setup() {
        for value in [
            "",
            "0",
            "-1",
            "many",
            "340282366920938463463374607431768211455",
        ] {
            check_environment("invalid_capacity", &[("SANDBOX_CAPACITY", value)]);
        }
        for value in ["", "-1", "1.5", "1s", "18446744073709551616"] {
            check_environment("invalid_idle", &[("SANDBOX_IDLE_TIMEOUT_MS", value)]);
        }
        check_environment("invalid_dpm", &[("DPM", "")]);
    }

    #[test]
    #[ignore = "environment-test helper; run through the parent tests"]
    fn environment_child() {
        let case = std::env::var("SANDBOX_ENV_TEST_CASE").expect("internal subprocess only");
        if case == "settings" {
            for builder in [
                explicit_builder().dpm_executable(""),
                explicit_builder().max_sandboxes(0),
                explicit_builder().startup_timeout(Duration::ZERO),
            ] {
                assert!(matches!(builder.build(), Err(Error::Configuration(_))));
            }
            let config = explicit_builder()
                .dpm_executable("./tools/dpm")
                .artifacts_dir("relative-artifacts")
                .startup_timeout(Duration::MAX)
                .idle_timeout(Duration::MAX)
                .shutdown_timeout(Duration::MAX)
                .build()
                .unwrap();
            assert_eq!(config.dpm, "./tools/dpm");
            assert_eq!(config.artifacts, Some(PathBuf::from("relative-artifacts")));
            assert_eq!(config.startup, Duration::MAX);
            assert_eq!(config.idle, Duration::MAX);
            assert_eq!(config.shutdown, Duration::MAX);
            return;
        }
        let builder = match case.as_str() {
            "defaults" => SandboxConfig::builder(),
            "invalid_builder" => explicit_builder().dpm_executable("").max_sandboxes(0),
            _ => explicit_builder(),
        };
        if matches!(
            case.as_str(),
            "invalid_capacity" | "invalid_idle" | "invalid_dpm"
        ) {
            let name = match case.as_str() {
                "invalid_capacity" => "SANDBOX_CAPACITY",
                "invalid_idle" => "SANDBOX_IDLE_TIMEOUT_MS",
                "invalid_dpm" => "DPM",
                _ => unreachable!(),
            };
            assert!(matches!(
                builder.build(),
                Err(Error::Configuration(message)) if message.contains(name)
            ));
            assert!(matches!(
                crate::pool::global(None, None),
                Err(Error::Configuration(message)) if message.contains(name)
            ));
            return;
        }
        let config = builder.build().unwrap();
        let (dpm, capacity, idle_ms) = match case.as_str() {
            "defaults" => ("dpm", 2, 3000),
            "explicit" => ("code-dpm", 7, 321),
            "overrides" | "invalid_builder" => ("env-dpm", 4, 1250),
            "dpm_override" => ("env-dpm", 7, 321),
            "capacity_override" => ("code-dpm", 4, 321),
            "idle_override" => ("code-dpm", 7, 1250),
            "zero_idle" => ("code-dpm", 7, 0),
            _ => panic!("unknown environment test case: {case}"),
        };
        assert_eq!(config.dpm, dpm);
        assert_eq!(config.capacity.get(), capacity);
        assert_eq!(config.idle, Duration::from_millis(idle_ms));
        assert_eq!(config.startup, Duration::from_secs(120));
        assert_eq!(config.shutdown, Duration::from_secs(15));
        assert!(config.dars.is_empty());
        assert!(config.artifacts.is_none());
    }
}
