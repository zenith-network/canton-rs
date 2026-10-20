//! RPC deadlines and failure diagnostics.

use std::{fmt::Debug, future::Future};

use super::LIMIT;

/// Read the participant endpoint supplied by the runner.
pub fn endpoint() -> String {
    std::env::var("CANTON_TEST_ENDPOINT")
        .expect("Run cargo run -p runner from tests/ledger-api-integration")
}

/// Bound an RPC or stream operation by the shared deadline.
pub async fn within<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .expect("operation exceeded 30-second deadline")
}

/// Require success within the deadline, retaining the error details on failure.
pub async fn ok<T, E: Debug>(future: impl Future<Output = Result<T, E>>) -> T {
    within(future)
        .await
        .unwrap_or_else(|error| panic!("operation failed: {error:#?}"))
}
