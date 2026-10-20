use std::path::PathBuf;

/// Configuration, startup, process, and cleanup failures reported by the harness.
///
/// Failures are cloneable because several tests can be awaiting the same startup.
/// Startup and process-exit errors include an artifact directory for diagnosis.
#[derive(Clone, Debug, thiserror::Error)]
pub enum Error {
    /// Invalid builder settings or a conflict with the installed process config.
    #[error("invalid sandbox configuration: {0}")]
    Configuration(String),
    /// An instance could not become ready. `message` describes the failure and
    /// `artifacts` points to its startup files and logs.
    #[error("sandbox startup failed: {message}; artifacts: {artifacts}")]
    Startup { message: String, artifacts: PathBuf },
    /// A running instance exited unexpectedly. `message` describes its exit and
    /// `artifacts` identifies the retained launch directory.
    #[error("sandbox exited unexpectedly: {message}; artifacts: {artifacts}")]
    ProcessExit { message: String, artifacts: PathBuf },
    /// Termination or reaping could not be confirmed. The manager closes further
    /// admission because it can no longer safely account for process capacity.
    #[error("sandbox cleanup failed: {0}")]
    Cleanup(String),
    /// The pool has closed or its manager can no longer answer requests.
    #[error("sandbox pool is closed")]
    Closed,
    /// A new manager registration raced with process-wide signal shutdown.
    #[error("sandbox run interrupted")]
    Interrupted,
    /// An operation failed and its cleanup also failed; both causes are retained.
    #[error("{primary}\nAdditionally: {cleanup}")]
    WithCleanup {
        /// Failure that caused cleanup to run.
        primary: Box<Self>,
        /// Additional failure encountered while cleaning up.
        cleanup: Box<Self>,
    },
}

impl Error {
    /// Preserve the original failure while attaching an unsuccessful teardown.
    pub(crate) fn combine(primary: Self, cleanup: Result<(), Self>) -> Self {
        match cleanup {
            Ok(()) => primary,
            Err(cleanup) => Self::WithCleanup {
                primary: Box::new(primary),
                cleanup: Box::new(cleanup),
            },
        }
    }

    /// Whether cleanup is uncertain and the manager must stop admitting work.
    pub(crate) fn cleanup_failed(&self) -> bool {
        matches!(self, Self::Cleanup(_) | Self::WithCleanup { .. })
    }
}
