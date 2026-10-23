//! Implementation support for the attribute; not a separately supported API.
//!
//! The macro supplies a body function and panic expectation. This module owns
//! acquisition, panic capture, and awaited release so early returns and `?` in
//! the body still complete the same lifecycle.

use crate::{
    Sandbox, SandboxConfig,
    pool::{self, Pool},
};
use futures_util::FutureExt;
use std::{
    any::Any,
    fmt::Debug,
    panic::{AssertUnwindSafe, resume_unwind},
};

/// Instance selection requested by a generated test wrapper.
pub enum Mode {
    /// Reuse the named group's ledger state while its instance survives.
    Shared(&'static str),
    /// Start an instance used exclusively by this acquisition.
    Fresh,
}
/// Panic expectation removed from the outer test and checked around its body.
pub enum ExpectedPanic {
    /// The body must return normally; its panics are resumed after release.
    No,
    /// The body must panic, optionally containing the specified message substring.
    Yes(Option<&'static str>),
}

/// Supported test return values: unit or a result whose error implements `Debug`.
pub trait TestOutput {
    /// Describe a returned error when a cleanup failure also needs reporting.
    fn failure(&self) -> Option<String>;
    /// Produce the successful return value after an expected body panic.
    fn success() -> Self;
}

impl TestOutput for () {
    fn failure(&self) -> Option<String> {
        None
    }
    fn success() -> Self {}
}

impl<E: Debug> TestOutput for Result<(), E> {
    fn failure(&self) -> Option<String> {
        self.as_ref().err().map(|error| format!("{error:?}"))
    }
    fn success() -> Self {
        Ok(())
    }
}

/// Execute the generated test against the process-wide pool.
///
/// Explicit config providers are called before acquisition and must agree with
/// the installed settings. The macro supplies Cargo's compile-time temporary
/// directory as the default artifact root when the pool is first initialized.
/// Setup and cleanup failures cannot satisfy an expected
/// body panic. Returned body errors retain priority over additional cleanup
/// failures, which are reported to standard error.
pub async fn run<T, F>(
    config: Option<fn() -> SandboxConfig>,
    target_tmpdir: Option<&str>,
    mode: Mode,
    name: &str,
    expected: ExpectedPanic,
    body: F,
) -> T
where
    T: TestOutput,
    F: for<'a> AsyncFnOnce(&'a Sandbox) -> T,
{
    let pool = pool::global(config.map(|provider| provider()), target_tmpdir)
        .unwrap_or_else(|error| panic!("{name}: setup failed: {error}"));
    run_with_pool(pool, mode, name, expected, body).await
}

/// Apply the body and release rules to a supplied pool, allowing deterministic
/// lifecycle tests without installing a fake process-wide pool.
pub(crate) async fn run_with_pool<T, F>(
    pool: &Pool,
    mode: Mode,
    name: &str,
    expected: ExpectedPanic,
    body: F,
) -> T
where
    T: TestOutput,
    F: for<'a> AsyncFnOnce(&'a Sandbox) -> T,
{
    let lease = match mode {
        Mode::Shared(group) => pool.shared_in_group(group).await,
        Mode::Fresh => pool.fresh().await,
    }
    .unwrap_or_else(|error| panic!("{name}: setup failed: {error}"));
    let outcome = AssertUnwindSafe(async { body(lease.sandbox()).await })
        .catch_unwind()
        .await;
    let cleanup = lease.release().await;
    match outcome {
        Err(panic) => {
            if let Err(error) = cleanup {
                eprintln!("{name}: cleanup failed after test panic: {error}");
                if matches!(expected, ExpectedPanic::Yes(_)) {
                    panic!("{name}: expected-panic body completed, but cleanup failed: {error}");
                }
            }
            match expected {
                ExpectedPanic::No => resume_unwind(panic),
                ExpectedPanic::Yes(None) => T::success(),
                ExpectedPanic::Yes(Some(expected)) => {
                    let message = panic_message(panic.as_ref());
                    assert!(
                        message.contains(expected),
                        "{name}: panic {message:?} did not contain {expected:?}"
                    );
                    T::success()
                }
            }
        }
        Ok(value) => {
            if let Err(error) = cleanup {
                if let Some(primary) = value.failure() {
                    eprintln!(
                        "{name}: original test failure: {primary}; additionally cleanup failed: {error}"
                    );
                    return value;
                }
                panic!("{name}: cleanup failed: {error}");
            }
            assert!(
                matches!(expected, ExpectedPanic::No),
                "{name}: test did not panic as expected"
            );
            value
        }
    }
}

fn panic_message(panic: &(dyn Any + Send)) -> &str {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic>")
}
