//! Construction of immutable settings before any manager or process is started.
//!
//! The builder holds optional overrides. `build` fills environment defaults,
//! resolves explicit paths, and checks invariants. The pool resolves an omitted
//! artifact directory using the test's Cargo directory or the system temporary directory.
use std::{
    ffi::OsString,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use crate::Error;
use canton_version::semver::Version;

/// Process capacity when neither the builder nor `SANDBOX_CAPACITY` overrides it.
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
    pub(crate) canton_version: String,
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
    /// Environment defaults and relative paths are evaluated when
    /// [`SandboxConfigBuilder::build`] is called.
    pub fn builder() -> SandboxConfigBuilder {
        SandboxConfigBuilder::default()
    }

    /// Resolve the artifact default once, before handing settings to the backend.
    pub(crate) fn resolve_artifacts(&mut self, target_tmpdir: Option<&str>) -> Result<(), Error> {
        if self.artifacts.is_none() {
            let mut directory = target_tmpdir
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

/// Builds a validated [`SandboxConfig`].
///
/// Environment defaults are read by [`Self::build`]. Explicit settings take
/// precedence over `SANDBOX_CANTON_VERSION`, `DPM`, `SANDBOX_CAPACITY`, and
/// `SANDBOX_IDLE_TIMEOUT_MS`. The default Canton release is pinned by the
/// `canton-version` crate, independently of the fixture compiler's `DPM_SDK_VERSION`.
///
/// Defaults are two instances, three seconds of shared idle retention, a
/// 120-second startup deadline, and a 15-second graceful shutdown period.
/// Builder setters defer validation until [`Self::build`].
///
/// ```no_run
/// use sandbox::SandboxConfig;
/// use std::time::Duration;
///
/// let config = SandboxConfig::builder()
///     .canton_version("3.5.12")
///     .max_sandboxes(3)
///     .idle_timeout(Duration::from_secs(1))
///     .build()?;
/// # Ok::<(), sandbox::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct SandboxConfigBuilder {
    canton_version: Option<String>,
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
            canton_version: None,
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
    /// Select an exact Canton release, rather than a version requirement.
    ///
    /// Overrides `SANDBOX_CANTON_VERSION` and the default from `canton-version`.
    /// Component availability and installation follow DPM's configuration.
    /// This does not select the Daml compiler used to build fixture DARs.
    pub fn canton_version(mut self, canton_version: impl Into<String>) -> Self {
        self.canton_version = Some(canton_version.into());
        self
    }

    /// Supply already compiled DAR bytes. No Daml compiler runs during acquisition.
    ///
    /// Repeated calls add archives. Parsing and registration happen during
    /// startup, and readiness requires each archive's main package to be registered.
    pub fn with_dar(mut self, bytes: impl Into<Arc<[u8]>>) -> Self {
        self.dars.push(bytes.into());
        self
    }

    /// Choose the DPM executable, overriding `DPM` or the fallback `dpm`.
    ///
    /// A bare executable name uses `PATH`. Relative paths such as `./tools/dpm`
    /// are resolved against the working directory when [`Self::build`] runs.
    pub fn dpm_executable(mut self, executable: impl Into<OsString>) -> Self {
        self.dpm = Some(executable.into());
        self
    }

    /// Choose the parent directory for sandbox logs and staged DARs.
    ///
    /// Overrides the default: `canton-rs-sandbox` under the integration test's
    /// `CARGO_TARGET_TMPDIR`, or under the system temporary directory when unavailable.
    /// Each instance and launch attempt gets its own subdirectory. Relative paths
    /// are resolved by [`Self::build`], and shutdown leaves artifacts in place.
    pub fn artifacts_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.artifacts = Some(directory.into());
        self
    }

    /// Override `SANDBOX_CAPACITY` (otherwise defaults to two).
    ///
    /// Counts starting, ready, idle, and stopping instances across all groups and
    /// fresh acquisitions in this process. Zero is rejected by [`Self::build`].
    pub fn max_sandboxes(mut self, capacity: usize) -> Self {
        self.capacity = Some(capacity);
        self
    }

    /// Override `SANDBOX_IDLE_TIMEOUT_MS` (otherwise defaults to three seconds).
    /// Zero retires shared instances immediately after their last release.
    ///
    /// Idle instances may retire sooner when another acquisition needs capacity.
    /// The final lease's release waits for reuse or completed shutdown.
    pub fn idle_timeout(mut self, duration: Duration) -> Self {
        self.idle = Some(duration);
        self
    }

    /// Set the startup deadline, which defaults to 120 seconds.
    ///
    /// The budget covers DAR preparation, launch retries, and Ledger API readiness.
    /// It starts after a capacity slot is assigned. Failed startups still run
    /// cleanup, whose shutdown budget is separate. Zero is invalid.
    pub fn startup_timeout(mut self, duration: Duration) -> Self {
        self.startup = duration;
        self
    }

    /// Set the graceful shutdown period, which defaults to 15 seconds.
    ///
    /// Cleanup asks the process group to terminate, then force-kills survivors
    /// after this period. Another five seconds is allowed to confirm termination
    /// and reap the child. Zero requests immediate escalation.
    pub fn shutdown_timeout(mut self, duration: Duration) -> Self {
        self.shutdown = duration;
        self
    }

    /// Read environment defaults, resolve relative paths, and check all settings.
    /// No external processes are started. A successful result is ready for use.
    ///
    /// Explicit setters override their corresponding environment defaults.
    /// Capacity becomes a nonzero value, and relative filesystem paths become
    /// absolute; bare executable names remain available for `PATH` lookup.
    ///
    /// Returns [`Error::Configuration`] for malformed environment values, empty
    /// executable names, invalid Canton releases, zero capacity or startup timeout,
    /// unsupported timeout ranges, or failure to resolve the current directory.
    /// Component availability and DAR contents are checked later during startup.
    pub fn build(self) -> Result<SandboxConfig, Error> {
        let capacity = match self.capacity {
            Some(capacity) => NonZeroUsize::new(capacity)
                .ok_or_else(|| Error::Configuration("sandbox capacity must be positive".into()))?,
            None => {
                environment_value::<NonZeroUsize>("SANDBOX_CAPACITY")?.unwrap_or(DEFAULT_CAPACITY)
            }
        };
        let idle = match self.idle {
            Some(idle) => idle,
            None => environment_value::<u64>("SANDBOX_IDLE_TIMEOUT_MS")?
                .map(Duration::from_millis)
                .unwrap_or(DEFAULT_IDLE_TIMEOUT),
        };
        let mut dpm = self
            .dpm
            .unwrap_or_else(|| std::env::var_os("DPM").unwrap_or_else(|| "dpm".into()));
        if dpm.is_empty() {
            return Err(Error::Configuration(
                "DPM executable must not be empty".into(),
            ));
        }
        let canton_version = match self.canton_version {
            Some(version) => Version::parse(&version).map_err(|error| {
                Error::Configuration(format!("invalid Canton release {version:?}: {error}"))
            })?,
            None => environment_value::<Version>("SANDBOX_CANTON_VERSION")?
                .unwrap_or_else(|| canton_version::canton_version().clone()),
        };
        if self.startup.is_zero() {
            return Err(Error::Configuration(
                "startup timeout must be positive".into(),
            ));
        }
        let now = std::time::Instant::now();
        let cleanup = self.shutdown.checked_add(Duration::from_secs(5));
        if now.checked_add(self.startup).is_none()
            || now.checked_add(idle).is_none()
            || cleanup.is_none_or(|duration| now.checked_add(duration).is_none())
        {
            return Err(Error::Configuration(
                "timeout exceeds the supported clock range".into(),
            ));
        }
        let mut artifacts = self.artifacts;
        let executable = Path::new(&dpm);
        let relative_executable = executable.is_relative() && executable.components().count() > 1;
        if artifacts.as_ref().is_some_and(|path| path.is_relative()) || relative_executable {
            let directory =
                std::env::current_dir().map_err(|error| Error::Configuration(error.to_string()))?;
            if let Some(path) = artifacts.as_mut().filter(|path| path.is_relative()) {
                *path = directory.join(&*path);
            }
            if relative_executable {
                dpm = directory.join(executable).into_os_string();
            }
        }
        Ok(SandboxConfig {
            canton_version: canton_version.to_string(),
            dpm,
            dars: self.dars,
            artifacts,
            capacity,
            idle,
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
mod tests {
    use super::*;
    use std::process::Command;

    fn builder() -> SandboxConfigBuilder {
        SandboxConfig::builder()
            .dpm_executable("dpm")
            .max_sandboxes(2)
            .idle_timeout(DEFAULT_IDLE_TIMEOUT)
    }

    #[test]
    fn build_rejects_invalid_settings_without_starting_processes() {
        for builder in [
            builder().canton_version(""),
            builder().canton_version(" "),
            builder().canton_version("3.5"),
            builder().canton_version("^3.5.12"),
            builder().canton_version(">=3.5.12"),
            builder().canton_version("3.5.*"),
            builder().dpm_executable(""),
            builder().max_sandboxes(0),
            builder().startup_timeout(Duration::ZERO),
            builder().startup_timeout(Duration::MAX),
            builder().idle_timeout(Duration::MAX),
            builder().shutdown_timeout(Duration::MAX),
        ] {
            assert!(matches!(builder.build(), Err(Error::Configuration(_))));
        }
        let release = "3.6.0-snapshot.20260817.14786.0.v35512442";
        assert_eq!(
            builder()
                .canton_version(release)
                .build()
                .unwrap()
                .canton_version,
            release
        );
    }

    #[test]
    fn build_resolves_relative_paths_and_preserves_path_lookup() {
        let directory = std::env::current_dir().unwrap();
        let relative = builder()
            .artifacts_dir("relative-artifacts")
            .dpm_executable("./tools/dpm")
            .build()
            .unwrap();
        assert_eq!(
            relative.artifacts,
            Some(directory.join("relative-artifacts"))
        );
        assert_eq!(relative.dpm, directory.join("./tools/dpm").into_os_string());
        let absolute = builder()
            .artifacts_dir(relative.artifacts.as_ref().unwrap())
            .dpm_executable(&relative.dpm)
            .build()
            .unwrap();
        assert_eq!(relative, absolute);
        let default = builder().build().unwrap();
        assert_eq!(default.dpm, "dpm");
        assert!(default.artifacts.is_none());
    }

    fn check_environment(
        case: &str,
        capacity: Option<&str>,
        idle: Option<&str>,
        canton: Option<&std::ffi::OsStr>,
    ) {
        // Give every case its own environment and process pool, without mutating
        // environment variables underneath concurrent Rust tests.
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args(["--ignored", "--exact", "config::tests::environment_child"])
            .env("SANDBOX_ENV_TEST_CASE", case)
            .env("DPM_SDK_VERSION", "not-a-canton-release")
            .env_remove("SANDBOX_CAPACITY")
            .env_remove("SANDBOX_IDLE_TIMEOUT_MS")
            .env_remove("SANDBOX_CANTON_VERSION");
        if let Some(capacity) = capacity {
            command.env("SANDBOX_CAPACITY", capacity);
        }
        if let Some(idle) = idle {
            command.env("SANDBOX_IDLE_TIMEOUT_MS", idle);
        }
        if let Some(canton) = canton {
            command.env("SANDBOX_CANTON_VERSION", canton);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{case}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn environment_defaults_and_zero_idle_timeout() {
        check_environment("defaults", None, None, None);
        check_environment("environment", Some("4"), Some("1250"), None);
        check_environment("zero_idle", Some("4"), Some("0"), None);
        check_environment("canton_environment", None, None, Some("3.5.12".as_ref()));
    }

    #[test]
    fn explicit_settings_override_environment_independently() {
        check_environment(
            "overrides",
            Some("invalid"),
            Some("invalid"),
            Some("invalid".as_ref()),
        );
        check_environment("capacity_override", Some("0"), Some("1250"), None);
        check_environment("idle_override", Some("4"), Some("-1"), None);
        check_environment("canton_override", None, None, Some("invalid".as_ref()));
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
            check_environment("invalid_capacity", Some(value), None, None);
        }
        for value in ["", "-1", "1.5", "1s", "18446744073709551616"] {
            check_environment("invalid_idle", None, Some(value), None);
        }
        for value in ["", " ", "3.5", "^3.5.12", ">=3.5.12", "3.5.*", "latest"] {
            check_environment("invalid_canton", None, None, Some(value.as_ref()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            let value = OsString::from_vec(vec![0xff]);
            check_environment("invalid_canton", None, None, Some(&value));
            check_environment("canton_override", None, None, Some(&value));
        }
    }

    #[test]
    #[ignore = "environment-test helper; run through the parent tests"]
    fn environment_child() {
        let case = std::env::var("SANDBOX_ENV_TEST_CASE").expect("internal subprocess only");
        let mut builder = SandboxConfig::builder();
        if matches!(case.as_str(), "overrides" | "capacity_override") {
            builder = builder.max_sandboxes(7);
        }
        if matches!(case.as_str(), "overrides" | "idle_override") {
            builder = builder.idle_timeout(Duration::from_millis(321));
        }
        if matches!(case.as_str(), "overrides" | "canton_override") {
            builder = builder.canton_version("3.5.12");
        }
        if case.starts_with("invalid_") {
            let name = match case.as_str() {
                "invalid_capacity" => "SANDBOX_CAPACITY",
                "invalid_idle" => "SANDBOX_IDLE_TIMEOUT_MS",
                "invalid_canton" => "SANDBOX_CANTON_VERSION",
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
        let config = builder.clone().build().unwrap();
        let (capacity, idle_ms) = match case.as_str() {
            "defaults" | "canton_environment" | "canton_override" => (2, 3000),
            "environment" => (4, 1250),
            "zero_idle" => (4, 0),
            "overrides" => (7, 321),
            "capacity_override" => (7, 1250),
            "idle_override" => (4, 321),
            _ => panic!("unknown environment test case: {case}"),
        };
        assert_eq!(config.capacity.get(), capacity);
        assert_eq!(config.idle, Duration::from_millis(idle_ms));
        let canton = match case.as_str() {
            "canton_environment" | "canton_override" | "overrides" => "3.5.12",
            _ => canton_version::canton_version_str(),
        };
        assert_eq!(config.canton_version, canton);
        let pool = crate::pool::global(Some(config.clone()), None).unwrap();
        assert!(std::ptr::eq(pool, crate::pool::global(None, None).unwrap()));
        // Equivalent explicit and environment settings must use the same pool.
        let explicit = builder
            .canton_version(canton)
            .max_sandboxes(capacity)
            .idle_timeout(Duration::from_millis(idle_ms))
            .build()
            .unwrap();
        assert!(std::ptr::eq(
            pool,
            crate::pool::global(Some(explicit), None).unwrap()
        ));
    }
}
