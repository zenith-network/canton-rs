use std::error::Error;

use tokio::time::Sleep;
use tonic::{Code, Status};

use crate::retry::{RetryConfig, RetryCtx};

/// Error which can be handled by the basic retry handler.
pub trait RetryableError<Ctx = RetryCtx>: Error {
    /// Returns an optional delay future.
    ///
    /// If `None` is returned, then no retry is advised.
    fn retry_after(&self, config: &mut RetryConfig, ctx: &Ctx) -> Option<Sleep>;

    /// Set the retry context of this error.
    ///
    /// Errors which don't record context can leave the default implementation.
    fn set_ctx(&mut self, ctx: &Ctx) {
        let _ = ctx;
    }
}

impl RetryableError for Status {
    /// For [`Status`] we consider only what we understand as "networking errors":
    ///
    /// - `DeadlineExceeded`
    /// - `ResourceExhausted`
    /// - `Unavailable`
    ///
    /// For other codes there will be no retry regardless the policy.
    fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
        config.network_mut().and_then(|policy| {
            match self.code() {
                // This is how we currently define network errors
                // This heuristics is based on tonic source code
                Code::DeadlineExceeded | Code::ResourceExhausted if self.source().is_some() => {
                    policy.next_backoff(ctx.attempt())
                }

                // For this code the source may be none in some cases (e.g. when proxy is involved)
                Code::Unavailable => policy.next_backoff(ctx.attempt()),

                // The rest are not considered networking errors, so they are not retried
                _ => None,
            }
        })
    }
}
