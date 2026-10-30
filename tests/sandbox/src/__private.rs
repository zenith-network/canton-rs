//! Implementation support for the attribute; not a separately supported API.
//!
//! The macro supplies a body function and panic expectation. This module owns
//! acquisition, panic capture, and awaited release so early returns and `?` in
//! the body still complete the same lifecycle.

use std::{
    any::Any,
    fmt::Debug,
    panic::{AssertUnwindSafe, resume_unwind},
    process::Termination,
};

use futures_util::FutureExt;

use crate::{
    Sandbox, SandboxConfig,
    pool::{self, Pool},
};

/// Instance selection requested by a generated test wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Reuse the named group's ledger state while its instance survives.
    Shared(&'static str),
    /// Start an instance used exclusively by this acquisition.
    Fresh,
}

/// Calls config provider and gets a global sandboxes pool reference.
///
/// Panics if failed to get the pool.
fn get_global_pool(
    config: Option<fn() -> SandboxConfig>,
    target_tmpdir: Option<&str>,
    name: &str,
) -> &'static Pool {
    let config = config.map(|config_provider| config_provider());
    let pool = pool::global(config, target_tmpdir)
        .unwrap_or_else(|error| panic!("{name}: failed to get global sandboxes pool: {error}"));
    pool
}

/// Run a test
///
/// This function returns the return value of the `body`, which must implement [`Termination`],
/// because it is enforced by Rust. If this bound is not satisfied, Rust will report a compilation
/// error when the original function will be passed to this function.
pub async fn run<T, F>(
    config: Option<fn() -> SandboxConfig>,
    target_tmpdir: Option<&str>,
    mode: Mode,
    name: &str,
    body: F,
) -> T
where
    T: Termination,
    F: for<'a> AsyncFnOnce(&'a Sandbox) -> T,
{
    let pool = get_global_pool(config, target_tmpdir, name);
    run_with_pool(pool, mode, name, body).await
}

/// Run a test, which is expected to panic
///
/// This function returns `()` just like the underlying `body`, because for `#[should_panic]` tests
/// if is enforced by Rust. Since we remove the upper-level `#[should_panic]` from the test, Rust
/// won't catch that for us, but it will report compilation error when the original function will be
/// passed to this function.
pub async fn run_should_panic<F>(
    config: Option<fn() -> SandboxConfig>,
    target_tmpdir: Option<&str>,
    mode: Mode,
    name: &str,
    expected: Option<&'static str>,
    body: F,
) where
    F: for<'a> AsyncFnOnce(&'a Sandbox),
{
    let pool = get_global_pool(config, target_tmpdir, name);
    run_with_pool_should_panic(pool, mode, name, expected, body).await
}

/// Run a test, which is expected to panic, against the pool
pub(crate) async fn run_with_pool_should_panic<F>(
    pool: &Pool,
    mode: Mode,
    name: &str,
    expected: Option<&'static str>,
    body: F,
) where
    F: for<'a> AsyncFnOnce(&'a Sandbox),
{
    let outcome = run_inner(pool, mode, name, body).await;

    // Message formats here are mirroring original Rust ones for convenience
    match outcome {
        Err(panic) => {
            if let Some(expected) = expected {
                if let Some(message) = panic_message(panic.as_ref()) {
                    assert!(
                        message.contains(expected),
                        "{name}: panic did not contain expected string\n\
                        panic message: {message:?}\n\
                        expected substring: {expected:?}"
                    );
                } else {
                    panic!(
                        "{name}: panic did not contain expected string\n\
                        found non-string value: `{:?}`\n\
                        expected substring: {expected:?}",
                        panic.as_ref().type_id()
                    );
                }
            }
        }
        Ok(()) => {
            panic!("{name}: test did not panic as expected")
        }
    }
}

/// Run a test against the pool
pub(crate) async fn run_with_pool<T, F>(pool: &Pool, mode: Mode, name: &str, body: F) -> T
where
    T: Termination,
    F: for<'a> AsyncFnOnce(&'a Sandbox) -> T,
{
    let outcome = run_inner(pool, mode, name, body).await;
    outcome.unwrap_or_else(|panic| resume_unwind(panic))
}

/// Acquire the sandbox and run the test body, catching the unwinding panics.
///
/// Panics on any error with sandbox acquisition or release.
///
/// Pool-related errors are taking precedence over actual test errors, shadowing them for user.
/// The idea of this is simple: if anything went wrong with the harness itself, we abort all the
/// tests, the user should not trust their results.
pub(crate) async fn run_inner<F, T>(
    pool: &Pool,
    mode: Mode,
    name: &str,
    body: F,
) -> Result<T, Box<dyn Any + Send>>
where
    F: for<'a> AsyncFnOnce(&'a Sandbox) -> T,
{
    let lease = match mode {
        Mode::Shared(group) => pool.shared_in_group(group).await,
        Mode::Fresh => pool.fresh().await,
    }
    .unwrap_or_else(|error| panic!("{name}: failed to acquire requested sandbox: {error}"));

    let outcome = AssertUnwindSafe(async { body(lease.sandbox()).await })
        .catch_unwind()
        .await;

    lease
        .release()
        .await
        .unwrap_or_else(|error| panic!("{name}: failed to release acquired sandbox: {error}"));

    outcome
}

/// Try to get panic message in string-like format
fn panic_message(panic: &(dyn Any + Send)) -> Option<&str> {
    panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
}
