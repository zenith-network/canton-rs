use std::{error::Error as _, sync::Arc, time::Duration};

use tokio::time::{self, Sleep};
use tonic::{Code, Response, Status};
use tower::retry::{
    backoff::{
        Backoff as _, ExponentialBackoff, ExponentialBackoffMaker, InvalidBackoff, MakeBackoff as _,
    },
    budget::{Budget as _, TpsBudget},
};

use crate::grpc::v2::error::{CantonError, CantonGrpcError};

pub use tower::retry::{backoff, budget};

#[derive(Clone, Debug)]
pub enum RetryLimitation {
    /// Token budgeting
    ///
    /// Note that the budget most likely should be shared among the clients of the same endpoint
    Tps(Arc<TpsBudget>),

    /// Max number of retries
    Fixed(usize),
}

impl RetryLimitation {
    /// Returns `true` if retry can be performed
    pub fn withdraw(&self, attempt: usize) -> bool {
        match self {
            Self::Tps(budget) => budget.withdraw(),
            Self::Fixed(max_retries) => attempt < *max_retries,
        }
    }

    /// If limitation is done through [`TpsBudget`], this deposits.
    pub fn deposit(&self) {
        if let Self::Tps(budget) = self {
            budget.deposit();
        }
    }
}

#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// If limitation is not set, there will be an infinite number of retries
    limitation: Option<RetryLimitation>,
    backoff: ExponentialBackoff,
}

impl RetryPolicy {
    pub fn new(min: Duration, max: Duration, jitter: f64) -> Result<Self, InvalidBackoff> {
        let mut maker = ExponentialBackoffMaker::new(min, max, jitter, Default::default())?;
        let backoff = maker.make_backoff();

        Ok(Self {
            limitation: None,
            backoff,
        })
    }

    pub fn from_backoff(backoff: ExponentialBackoff) -> Self {
        Self {
            limitation: None,
            backoff,
        }
    }

    /// Set the limitat to given budget
    pub fn with_budget(mut self, budget: Arc<TpsBudget>) -> Self {
        self.limitation = Some(RetryLimitation::Tps(budget));
        self
    }

    /// Set the limit to fixed number of max retries
    pub fn with_max(mut self, max_retries: usize) -> Self {
        self.limitation = Some(RetryLimitation::Fixed(max_retries));
        self
    }

    pub fn next_backoff(&mut self, attempt: usize) -> Option<Sleep> {
        if let Some(limitation) = &self.limitation {
            limitation
                .withdraw(attempt)
                .then(|| self.backoff.next_backoff())
        } else {
            Some(self.backoff.next_backoff())
        }
    }

    pub fn success(&self) {
        if let Some(limitation) = &self.limitation {
            limitation.deposit();
        }
    }
}

#[derive(Clone, Debug)]
pub enum CantonRetryPolicy {
    /// Perform retry based on given policy
    ///
    /// Only retryable errors (determined by category ID) will be retried.
    Fixed(RetryPolicy),

    /// Determine retries based on the responses from Ledger API (fallback to error category IDs)
    Auto(Option<RetryLimitation>),
}

impl CantonRetryPolicy {
    pub fn auto() -> Self {
        Self::Auto(None)
    }

    pub fn auto_with_budget(budget: Arc<TpsBudget>) -> Self {
        Self::Auto(Some(RetryLimitation::Tps(budget)))
    }

    pub fn auto_with_max(max_retries: usize) -> Self {
        Self::Auto(Some(RetryLimitation::Fixed(max_retries)))
    }

    pub fn success(&self) {
        match self {
            CantonRetryPolicy::Fixed(retry_policy) => {
                retry_policy.success();
            }
            CantonRetryPolicy::Auto(retry_limitation) => {
                retry_limitation.as_ref().map(|lim| lim.deposit());
            }
        }
    }
}

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
            CantonError::CantonGrpc(error) => self
                .canton
                .as_mut()
                .map(|policy| Self::retry_canton_grpc(error, policy, attempt))
                .flatten(),

            CantonError::Raw(status) => self
                .network
                .as_mut()
                .map(|policy| Self::retry_network(status, policy, attempt))
                .flatten(),

            // This one is always non retryable
            CantonError::ValueError(_) => None,
        }
    }

    fn retry_canton_grpc(
        error: &CantonGrpcError,
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

#[derive(Clone, Debug)]
pub struct RetryHandler {
    config: RetryConfig,
}

impl RetryHandler {
    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    pub async fn call<
        S: Clone,
        Req: Clone,
        Resp,
        F: Fn(S, Req) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, Status>>,
    >(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, CantonError> {
        let mut attempt = 0;

        loop {
            let result = func(service.clone(), request.clone())
                .await
                .map_err(CantonError::from);

            match result {
                Ok(response) => {
                    self.config.success();
                    return Ok(response.into_inner());
                }
                Err(error) => {
                    let Some(delay) = self.config.retry_after(&error, attempt) else {
                        return Err(error);
                    };

                    // We log only if the error is not returned, to avoid log duplication
                    #[cfg(feature = "tracing")]
                    tracing::trace!(attempt, ?error, "Client encountered error");

                    attempt += 1;
                    delay.await;
                }
            }
        }
    }

    pub async fn call_with_attempt<
        S: Clone,
        Req: Clone,
        Resp,
        F: Fn(S, Req, usize) -> Fut,
        Fut: Future<Output = Result<Response<Resp>, Status>>,
    >(
        &mut self,
        service: &S,
        request: &Req,
        func: F,
    ) -> Result<Resp, CantonError> {
        let mut attempt = 0;

        loop {
            let result = func(service.clone(), request.clone(), attempt)
                .await
                .map_err(CantonError::from);

            match result {
                Ok(response) => return Ok(response.into_inner()),
                Err(error) => {
                    let Some(delay) = self.config.retry_after(&error, attempt) else {
                        return Err(error);
                    };

                    // We log only if the error is not returned, to avoid log duplication
                    #[cfg(feature = "tracing")]
                    tracing::trace!(attempt, ?error, "Client encountered error");

                    attempt += 1;
                    delay.await;
                }
            }
        }
    }
}
