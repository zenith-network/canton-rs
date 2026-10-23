//! Bounded Canton sandbox fixtures for ordinary asynchronous Rust tests.
//!
//! Use [`macro@sandbox`] with `shared`, `shared, group = "name"`, or `fresh`.
//! The crate manages one bounded pool per test process. A shared group permits
//! unrelated activity within that group; it is not a reset boundary.
//! Fresh acquisitions always start a new instance. Prefer [`macro@sandbox`] or explicitly
//! await [`SandboxLease::release`] so the final test waits for cleanup.
//!
//! SIGINT/SIGTERM stop sandboxes before the process exits. SIGKILL and abort
//! cannot run cleanup. See the crate README for lifecycle and scheduling limits.
//!
//! The [`macro@sandbox`] attribute acquires a ready instance before calling the
//! test body and awaits release afterward, including when the body panics.
//! Place it above `#[tokio::test]` and accept one `&Sandbox` argument:
//!
//! ```no_run
//! use sandbox::{Sandbox, sandbox};
//!
//! #[sandbox(shared, group = "contracts")]
//! #[tokio::test]
//! async fn contracts(sandbox: &Sandbox) {
//!     assert!(sandbox.ledger_api_endpoint().starts_with("http://"));
//! }
//! ```
//!
//! Configure instances with [`SandboxConfig::builder`]. The first acquisition
//! installs the process configuration; explicit configurations used by later
//! tests must agree. For manual acquisition, keep the returned [`SandboxLease`]
//! until all client operations finish and await its release.

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
compile_error!("sandbox currently supports Linux and macOS");

#[doc(hidden)]
pub mod __private;
mod backend;
mod config;
mod error;
mod manager;
mod pool;

pub use config::{SandboxConfig, SandboxConfigBuilder};
pub use error::Error;
pub use pool::SandboxLease;
pub use sandbox_macro::sandbox;

use std::path::{Path, PathBuf};

/// Name of the group used by `#[sandbox(shared)]` and [`Sandbox::shared`].
///
/// Explicitly acquiring group `"0"` accesses the same shared ledger state.
pub const DEFAULT_GROUP: &str = "0";

/// Immutable information about one particular sandbox instance.
///
/// The attribute lends this value to the test body. Manual acquisitions return a
/// [`SandboxLease`] that dereferences to it and keeps the instance in use.
/// Client connections should finish their work before that lease is released.
#[derive(Debug)]
pub struct Sandbox {
    pub(crate) id: String,
    pub(crate) endpoints: Endpoints,
    pub(crate) artifacts: PathBuf,
}

/// Service base URLs assigned to a sandbox instance on local loopback ports.
///
/// Addresses are returned directly rather than published through environment
/// variables. Startup checks the Ledger API and preloaded packages for readiness;
/// the other addresses identify services without separate readiness checks.
#[derive(Clone, Debug)]
pub struct Endpoints {
    /// Participant Ledger API URL, suitable for connecting a tonic client.
    pub ledger_api: String,
    /// Participant administration API URL.
    pub admin_api: String,
    /// Participant JSON API URL.
    pub json_api: String,
    /// Sequencer public API URL.
    pub sequencer_public: String,
    /// Sequencer administration API URL.
    pub sequencer_admin: String,
    /// Mediator administration API URL.
    pub mediator_admin: String,
}

impl Sandbox {
    /// Acquire the process's default shared sandbox group.
    ///
    /// Equivalent to [`Self::shared_in_group`] with [`DEFAULT_GROUP`]. Acquisitions
    /// for the same group share startup and ledger state, including concurrent
    /// writes made by other tests.
    pub async fn shared() -> Result<SandboxLease, Error> {
        Self::shared_in_group(DEFAULT_GROUP).await
    }

    /// Acquire a sandbox whose state is shared only with this named group.
    ///
    /// Reuses the group's active or idle instance, or starts one when capacity is
    /// available. State persists while that instance remains alive; a group name
    /// does not preserve state after idle retirement or process failure.
    ///
    /// Returns [`Error::Configuration`] for an empty group name. Configuration,
    /// startup, and manager failures are also returned to the caller.
    pub async fn shared_in_group(group: &str) -> Result<SandboxLease, Error> {
        pool::global(None, None)?.shared_in_group(group).await
    }

    /// Acquire a fresh sandbox that will stop when this lease is released.
    ///
    /// Every acquisition starts a new instance using the process configuration.
    /// Fresh instances and shared groups use the same capacity limit. Waiting
    /// for capacity precedes the startup timeout.
    pub async fn fresh() -> Result<SandboxLease, Error> {
        pool::global(None, None)?.fresh().await
    }

    /// Identifier for this launched instance, shared by all of its leases.
    ///
    /// A replacement instance receives a new identifier even in the same group.
    pub fn id(&self) -> &str {
        &self.id
    }
    /// All service addresses assigned to this instance.
    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }
    /// Participant Ledger API URL used by SDK clients.
    pub fn ledger_api_endpoint(&self) -> &str {
        &self.endpoints.ledger_api
    }
    /// Directory for this launch attempt's logs and port file.
    ///
    /// Process output is captured in `sandbox.log`; Canton writes `canton.log`.
    /// Artifacts are retained after shutdown to help diagnose test failures.
    pub fn artifacts_dir(&self) -> &Path {
        &self.artifacts
    }
}
