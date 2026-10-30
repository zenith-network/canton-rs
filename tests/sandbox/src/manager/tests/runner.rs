use super::*;
use crate::__private::{Mode, run_with_pool, run_with_pool_should_panic};
use futures_util::FutureExt;
use std::{
    panic::{AssertUnwindSafe, panic_any},
    process::ExitCode,
};

#[tokio::test]
async fn returned_values_are_preserved_after_release() {
    let fake = Fake::new();
    let pool = pool(&fake, 1);
    for expected in [Ok(()), Err("body error")] {
        let result = run_with_pool(
            &pool,
            Mode::Shared("runner"),
            "returned result",
            async move |_sandbox: &Sandbox| expected,
        )
        .await;
        assert_eq!(result, expected);
        assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    }
    let status = run_with_pool(
        &pool,
        Mode::Fresh,
        "exit code",
        async |_sandbox: &Sandbox| ExitCode::from(42),
    )
    .await;
    assert_eq!(status, ExitCode::from(42));
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    assert_eq!(fake.stops.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn runner_waits_for_cleanup() {
    let fake = Fake::gated(1, 0);
    let pool = pool(&fake, 1);
    let running = tokio::spawn(async move {
        run_with_pool(
            &pool,
            Mode::Fresh,
            "awaited cleanup",
            async |sandbox: &Sandbox| {
                assert_eq!(sandbox.id(), "fake-1");
            },
        )
        .await;
    });
    until(|| fake.stops.load(Ordering::SeqCst) == 1).await;
    let finished_before_cleanup = running.is_finished();
    fake.stop_gate.add_permits(1);
    assert!(!finished_before_cleanup);
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unexpected_body_panic_is_resumed_after_release() {
    let fake = Fake::new();
    let pool = pool(&fake, 1);
    let panic = AssertUnwindSafe(run_with_pool(
        &pool,
        Mode::Fresh,
        "body panic",
        async |_sandbox: &Sandbox| {
            tokio::task::yield_now().await;
            panic_any(42_u32);
        },
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    assert_eq!(*panic.downcast::<u32>().unwrap(), 42);
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    assert_eq!(fake.stops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn expected_panics_accept_any_payload_or_a_matching_string() {
    let fake = Fake::new();
    let pool = pool(&fake, 1);
    run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "any payload",
        None,
        async |_sandbox: &Sandbox| panic_any(42_u32),
    )
    .await;
    run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "borrowed string",
        Some("needle"),
        async |_sandbox: &Sandbox| panic!("a needle in a haystack"),
    )
    .await;
    run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "owned string",
        Some("needle"),
        async |_sandbox: &Sandbox| panic_any(String::from("a needle in a haystack")),
    )
    .await;
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    assert_eq!(fake.stops.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn missing_or_mismatched_panics_fail_after_release() {
    let fake = Fake::new();
    let pool = pool(&fake, 1);
    let panic = AssertUnwindSafe(run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "missing panic",
        None,
        async |_sandbox: &Sandbox| {},
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains("missing panic: test did not panic as expected"));

    let panic = AssertUnwindSafe(run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "wrong message",
        Some("needle"),
        async |_sandbox: &Sandbox| panic!("haystack"),
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains("wrong message: panic did not contain expected string"));
    assert!(message.contains("haystack"));
    assert!(message.contains("needle"));

    let panic = AssertUnwindSafe(run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "non-string",
        Some("needle"),
        async |_sandbox: &Sandbox| panic_any(42_u32),
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains("non-string: panic did not contain expected string"));
    assert!(message.contains("found non-string value"));
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    assert_eq!(fake.stops.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn acquisition_failure_cannot_satisfy_expected_panic() {
    let fake = Fake::new();
    fake.fail_start.store(true, Ordering::SeqCst);
    let pool = pool(&fake, 1);
    let panic = AssertUnwindSafe(run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "setup",
        Some("injected startup failure"),
        async |_sandbox: &Sandbox| panic!("body must not run"),
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains("setup: failed to acquire requested sandbox"));
    assert!(message.contains("injected startup failure"));
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn cleanup_failure_takes_precedence_over_returned_errors_and_body_panics() {
    for body_panics in [false, true] {
        let fake = Fake::new();
        fake.fail_stop.store(true, Ordering::SeqCst);
        let pool = pool(&fake, 1);
        let panic = AssertUnwindSafe(run_with_pool(
            &pool,
            Mode::Fresh,
            "cleanup",
            async move |_sandbox: &Sandbox| {
                if body_panics {
                    panic!("body panic");
                }
                Err::<(), _>("body error")
            },
        ))
        .catch_unwind()
        .await
        .unwrap_err();
        let message = panic.downcast::<String>().unwrap();
        assert!(message.contains("cleanup: failed to release acquired sandbox"));
        assert!(message.contains("injected cleanup failure"));
        assert_eq!(fake.live.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn cleanup_failure_cannot_satisfy_expected_panic() {
    let fake = Fake::new();
    fake.fail_stop.store(true, Ordering::SeqCst);
    let pool = pool(&fake, 1);
    let panic = AssertUnwindSafe(run_with_pool_should_panic(
        &pool,
        Mode::Fresh,
        "cleanup",
        Some("injected cleanup failure"),
        async |_sandbox: &Sandbox| panic!("injected cleanup failure"),
    ))
    .catch_unwind()
    .await
    .unwrap_err();
    let message = panic.downcast::<String>().unwrap();
    assert!(message.contains("cleanup: failed to release acquired sandbox"));
    assert!(message.contains("injected cleanup failure"));
    assert_eq!(fake.live.load(Ordering::SeqCst), 0);
}
