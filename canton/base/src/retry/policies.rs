use std::{sync::Arc, time::Duration};

use tokio::time::Sleep;
use tower::retry::{
    backoff::{
        Backoff as _, ExponentialBackoff, ExponentialBackoffMaker, InvalidBackoff, MakeBackoff as _,
    },
    budget::{Budget as _, TpsBudget},
};

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

    /// Set the limitation to given budget
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
                if let Some(lim) = retry_limitation {
                    lim.deposit()
                }
            }
        }
    }
}
