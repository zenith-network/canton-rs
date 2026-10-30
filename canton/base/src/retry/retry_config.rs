use std::{error::Error as _, sync::Arc, time::Duration};

use tokio::time::{self, Sleep};
use tonic::{Code, Response, Status};
use tower::retry::{
    backoff::{
        Backoff as _, ExponentialBackoff, ExponentialBackoffMaker, InvalidBackoff, MakeBackoff as _,
    },
    budget::{Budget as _, TpsBudget},
};

pub use tower::retry::{backoff, budget};

use crate::retry::retry_handler::RetryHandler;

#[derive(Clone, Debug)]
pub struct RetryConfig {
    network: Option<RetryPolicy>,
    canton: Option<CantonRetryPolicy>,
}

impl Default for RetryConfig {
    /// Same as [`Self::no_retry()`]
    fn default() -> Self {
        Self::no_retry()
    }
}

impl RetryConfig {
    /// Same as [`Self::no_retry()`]
    pub fn new() -> Self {
        Self::no_retry()
    }

    /// New config with no retry enabled
    pub fn no_retry() -> Self {
        Self {
            network: None,
            canton: None,
        }
    }

    pub fn with_canton_policy(mut self, policy: Option<CantonRetryPolicy>) -> Self {
        self.canton = policy;
        self
    }

    pub fn with_network_policy(mut self, policy: Option<RetryPolicy>) -> Self {
        self.network = policy;
        self
    }

    pub fn into_handler(self) -> RetryHandler {
        RetryHandler { config: self }
    }

    /// Returns an optional delay future
    pub fn retry_after(&mut self, error: &CantonError, attempt: usize) -> Option<Sleep> {
        match error {
            CantonError::Decoded(error) => self
                .canton
                .as_mut()
                .and_then(|policy| Self::retry_canton_grpc(error, policy, attempt)),

            CantonError::Raw(status) => self
                .network
                .as_mut()
                .and_then(|policy| Self::retry_network(status, policy, attempt)),

            // We consider all redacted errors as non retryable now.
            // Although it's not strictly enforced anywhere, there is no reasonable way to perform
            // an automatic retry in this case now.
            CantonError::Redacted(_) => None,

            // Daml interpretation errors (being a special case of decoded errors) are always
            // considered non-retryable
            CantonError::Daml(_) => None,

            // Always non retryable
            CantonError::ValueError(_) | CantonError::UnsupportedServerVersion(_) => None,
        }
    }

    fn retry_canton_grpc(
        error: &DecodedCantonError,
        policy: &mut CantonRetryPolicy,
        attempt: usize,
    ) -> Option<Sleep> {
        let delay = || {
            error
                .retry_delay()
                .or_else(|| error.category_id().retry())
                .map(|duration| time::sleep(duration))
        };

        match policy {
            CantonRetryPolicy::Fixed(policy) => {
                error
                    .category_id()
                    .retry()
                    .is_some() // checks that the error is considered to be retryable
                    .then(|| policy.next_backoff(attempt))
                    .flatten()
            }
            CantonRetryPolicy::Auto(Some(limitation)) => {
                limitation.withdraw(attempt).then(delay).flatten()
            }
            CantonRetryPolicy::Auto(None) => delay(),
        }
    }

    fn retry_network(status: &Status, policy: &mut RetryPolicy, attempt: usize) -> Option<Sleep> {
        match status.code() {
            // This is how we currently define network errors
            // This heuristics is based on tonic source code
            Code::DeadlineExceeded | Code::ResourceExhausted | Code::Unavailable
                if status.source().is_some() =>
            {
                policy.next_backoff(attempt)
            }

            // The rest are not considered networking errors, so they are not retried
            _ => None,
        }
    }

    /// Register successful request
    pub fn success(&self) {
        if let Some(policy) = &self.canton {
            policy.success();
        }
        if let Some(policy) = &self.network {
            policy.success();
        }
    }
}
