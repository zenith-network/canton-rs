//! Process-wide acquisition and ownership of instance claims.
//!
//! Claims are guarded before requests are sent, so cancellation during startup
//! has the same cleanup guarantee as dropping an acquired lease. The manager
//! performs that cleanup on its own runtime.
use std::sync::{
    Arc, OnceLock,
    atomic::{AtomicU64, Ordering},
};

use tokio::sync::oneshot;

use crate::{Error, Sandbox, SandboxConfig, manager};

static POOL: OnceLock<Result<Pool, Error>> = OnceLock::new();

/// A process has one configuration and one capacity limit, across all groups.
/// Explicit configuration must agree with the configuration already in use.
/// Passing `None` uses the installed settings, building defaults only on first
/// use. An omitted artifact path inherits the installed path, or defaults to
/// `canton-rs-sandbox` under `target_tmpdir` or the system temporary directory.
/// Initialization failures are cached with the process pool.
pub(crate) fn global(
    config: Option<SandboxConfig>,
    target_tmpdir: Option<&str>,
) -> Result<&'static Pool, Error> {
    let pool = POOL
        .get_or_init(|| {
            let config = match config.clone() {
                Some(config) => config,
                None => SandboxConfig::builder().build()?,
            };
            Pool::new(config, target_tmpdir)
        })
        .as_ref()
        .map_err(Clone::clone)?;
    if let Some(mut config) = config {
        if config.artifacts.is_none() {
            config.artifacts = pool.config.artifacts.clone();
        }
        if config != pool.config {
            return Err(Error::Configuration(
                "sandbox settings differ from the process configuration; use the same config provider for all sandbox tests".into(),
            ));
        }
    }
    Ok(pool)
}

/// Client-side handle for allocating unique requests to one manager.
///
/// Production callers share the global pool; separate backends are used only by
/// lifecycle tests. Instance state and capacity accounting live in the manager.
pub(crate) struct Pool {
    config: SandboxConfig,
    pub(crate) core: Arc<manager::Core>,
    next_request: AtomicU64,
}

impl Pool {
    fn new(mut config: SandboxConfig, target_tmpdir: Option<&str>) -> Result<Self, Error> {
        config.resolve_artifacts(target_tmpdir)?;
        Ok(Self {
            core: manager::register(config.clone())?,
            config,
            next_request: AtomicU64::new(1),
        })
    }

    #[cfg(test)]
    pub(crate) async fn shared(&self) -> Result<SandboxLease, Error> {
        self.shared_in_group(crate::DEFAULT_GROUP).await
    }

    pub(crate) async fn shared_in_group(&self, group: &str) -> Result<SandboxLease, Error> {
        if group.is_empty() {
            return Err(Error::Configuration(
                "shared group must not be empty".into(),
            ));
        }
        self.acquire(Some(group.to_owned())).await
    }

    pub(crate) async fn fresh(&self) -> Result<SandboxLease, Error> {
        self.acquire(None).await
    }

    async fn acquire(&self, group: Option<String>) -> Result<SandboxLease, Error> {
        let id = self.next_request.fetch_add(1, Ordering::Relaxed);
        let claim = Claim {
            core: self.core.clone(),
            id,
            armed: true,
        };
        let (reply, receive) = oneshot::channel();
        self.core
            .send(manager::Command::Acquire { id, group, reply })?;
        let sandbox = receive.await.map_err(|_| Error::Closed)??;
        Ok(SandboxLease { sandbox, claim })
    }

    #[cfg(test)]
    pub(crate) async fn shutdown(&self) -> Result<(), Error> {
        self.core.shutdown().await
    }

    #[cfg(test)]
    pub(crate) fn with_backend(
        mut config: SandboxConfig,
        backend: Arc<dyn crate::backend::Backend>,
    ) -> Self {
        config.resolve_artifacts(None).unwrap();
        Self {
            core: manager::register_backend(config.clone(), backend).unwrap(),
            config,
            next_request: AtomicU64::new(1),
        }
    }
}

/// A test's claim on an instance. It is deliberately not cloneable.
///
/// Prefer the attribute for automatic cleanup. Manual acquisitions should await
/// [`Self::release`]. Dropping an unfinished claim blocks until the independent
/// manager stops an otherwise unowned instance, without the idle delay.
/// Dropping one lease leaves other claims on the same shared instance intact.
pub struct SandboxLease {
    sandbox: Arc<Sandbox>,
    claim: Claim,
}

impl SandboxLease {
    /// Borrow the ready instance while this lease keeps its claim active.
    ///
    /// The lease also dereferences to [`Sandbox`] for convenient endpoint access.
    pub fn sandbox(&self) -> &Sandbox {
        &self.sandbox
    }

    /// Release this claim and await any cleanup for which it is responsible.
    ///
    /// Shared claims return immediately when other users remain. The final shared
    /// release waits for reuse, idle retirement, or eviction to free capacity.
    /// Fresh instances always shut down on release.
    ///
    /// Returns process-exit or cleanup failures reported by the manager. If this
    /// future is cancelled, the drop guard still waits for necessary cleanup.
    pub async fn release(mut self) -> Result<(), Error> {
        let (reply, receive) = oneshot::channel();
        self.claim.core.send(manager::Command::Release {
            id: self.claim.id,
            reply,
        })?;
        let outcome = receive.await.map_err(|_| Error::Closed)?;
        self.claim.armed = false;
        outcome
    }
}

impl std::ops::Deref for SandboxLease {
    type Target = Sandbox;
    fn deref(&self) -> &Self::Target {
        &self.sandbox
    }
}

/// Cancellation guard covering both a pending acquisition and its eventual lease.
///
/// Disarmed only after an awaited release reply. Its blocking drop must happen
/// outside the manager thread, which needs to keep running to acknowledge it.
struct Claim {
    core: Arc<manager::Core>,
    id: u64,
    armed: bool,
}

impl Drop for Claim {
    fn drop(&mut self) {
        if self.armed {
            self.core.cancel_blocking(self.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn conflicting_configuration_cannot_create_another_pool() {
        let builder = SandboxConfig::builder()
            .dpm_executable("dpm")
            .max_sandboxes(2)
            .idle_timeout(Duration::ZERO);
        let config = builder.clone().build().unwrap();
        let pool = global(Some(config.clone()), Some("target/tmp")).unwrap();
        assert_eq!(
            pool.config.artifacts,
            Some(
                std::env::current_dir()
                    .unwrap()
                    .join("target/tmp/canton-rs-sandbox")
            )
        );
        assert!(std::ptr::eq(pool, global(None, None).unwrap()));
        // Missing settings inherit the initialized pool even without Cargo metadata.
        assert!(std::ptr::eq(
            pool,
            global(Some(config.clone()), None).unwrap()
        ));
        assert!(std::ptr::eq(
            pool,
            global(None, Some("other-target/tmp")).unwrap()
        ));
        let matching = builder
            .clone()
            .artifacts_dir(pool.config.artifacts.as_ref().unwrap())
            .build()
            .unwrap();
        assert!(std::ptr::eq(pool, global(Some(matching), None).unwrap()));
        assert!(matches!(
            global(
                Some(
                    builder
                        .clone()
                        .artifacts_dir("other-artifacts")
                        .build()
                        .unwrap()
                ),
                None,
            ),
            Err(Error::Configuration(_))
        ));
        assert!(matches!(
            global(
                Some(builder.clone().max_sandboxes(3).build().unwrap()),
                None
            ),
            Err(Error::Configuration(_))
        ));
        assert!(matches!(
            global(Some(builder.with_dar(vec![1, 2, 3]).build().unwrap()), None),
            Err(Error::Configuration(_))
        ));
        assert!(std::ptr::eq(
            pool,
            global(Some(config), Some("target/tmp")).unwrap()
        ));
    }

    #[test]
    fn artifact_directory_defaults_and_explicit_override() {
        let builder = SandboxConfig::builder()
            .dpm_executable("dpm")
            .max_sandboxes(2)
            .idle_timeout(Duration::ZERO);
        let system_default = Pool::new(builder.clone().build().unwrap(), None).unwrap();
        assert_eq!(
            system_default.config.artifacts,
            Some(
                std::path::absolute(std::env::temp_dir())
                    .unwrap()
                    .join("canton-rs-sandbox")
            )
        );
        let explicit = builder.artifacts_dir("custom-artifacts").build().unwrap();
        let pool = Pool::new(explicit.clone(), Some("target/tmp")).unwrap();
        assert_eq!(pool.config, explicit);
    }
}
