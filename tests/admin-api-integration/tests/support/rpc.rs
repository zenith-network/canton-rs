//! RPC deadlines and failure diagnostics.

use std::{fmt::Debug, future::Future};

use super::LIMIT;

/// Bound an operation by the shared deadline, identifying its endpoint on failure.
async fn within<T>(endpoint: &str, future: impl Future<Output = T>) -> T {
    tokio::time::timeout(LIMIT, future)
        .await
        .unwrap_or_else(|_| panic!("{endpoint}: operation exceeded 30-second deadline"))
}

/// Require success within the deadline, retaining the error details on failure.
pub async fn ok<T, E: Debug>(endpoint: &str, future: impl Future<Output = Result<T, E>>) -> T {
    within(endpoint, future)
        .await
        .unwrap_or_else(|error| panic!("{endpoint}: operation failed: {error:#?}"))
}
