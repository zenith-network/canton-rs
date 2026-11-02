use std::{ops::Range, sync::Arc, time::Duration};

use tokio::time::Sleep;
use tower::{
    retry::{
        backoff::{
            Backoff as _, ExponentialBackoff, ExponentialBackoffMaker, InvalidBackoff,
            MakeBackoff as _,
        },
        budget::{Budget as _, TpsBudget},
    },
    util::rng::{HasherRng, Rng},
};

pub use tower::retry::{backoff, budget};

/// Limitation of the number of retries
#[derive(Clone, Debug)]
pub enum RetryLimitation {
    /// Token budgeting
    ///
    /// Note that the budget most likely should be shared among the clients of the same endpoint
    Tps(Arc<TpsBudget>),

    /// Fixed max number of retries
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
    ///
    /// This should be done on successful request.
    pub fn deposit(&self) {
        if let Self::Tps(budget) = self {
            budget.deposit();
        }
    }
}

impl From<Arc<TpsBudget>> for RetryLimitation {
    fn from(value: Arc<TpsBudget>) -> Self {
        Self::Tps(value)
    }
}

impl From<usize> for RetryLimitation {
    fn from(value: usize) -> Self {
        Self::Fixed(value)
    }
}

/// New-type for [`HasherRng`] which refreshes the state on every clone.
#[derive(Debug, Default)]
pub struct RefreshingRng(HasherRng);

impl Clone for RefreshingRng {
    fn clone(&self) -> Self {
        Self(HasherRng::default())
    }
}

// Implementation fully delegates to underlying type
impl Rng for RefreshingRng {
    fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    fn next_f64(&mut self) -> f64 {
        self.0.next_f64()
    }

    fn next_range(&mut self, range: Range<u64>) -> u64 {
        self.0.next_range(range)
    }
}

/// Generic retry policy
///
/// Note: when cloning this, exponential backoff will be re-created with a fresh random state.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// If limitation is not set, there will be an infinite number of retries
    limitation: Option<RetryLimitation>,
    backoff: ExponentialBackoff<RefreshingRng>,
}

impl RetryPolicy {
    /// Create new retry policy with infinite number of retries and exponential backoff.
    pub fn new(min: Duration, max: Duration, jitter: f64) -> Result<Self, InvalidBackoff> {
        let mut maker = ExponentialBackoffMaker::new(min, max, jitter, RefreshingRng::default())?;
        let backoff = maker.make_backoff();

        Ok(Self {
            limitation: None,
            backoff,
        })
    }

    /// Create new retry policy directly from exponential backoff (infinite number of retries).
    pub fn from_backoff(backoff: ExponentialBackoff<RefreshingRng>) -> Self {
        Self {
            limitation: None,
            backoff,
        }
    }

    /// Set the retry number limitation to given budget manager.
    pub fn with_budget(mut self, budget: Arc<TpsBudget>) -> Self {
        self.limitation = Some(RetryLimitation::Tps(budget));
        self
    }

    /// Set the retry number limit to a fixed number.
    pub fn with_max(mut self, max_retries: usize) -> Self {
        self.limitation = Some(RetryLimitation::Fixed(max_retries));
        self
    }

    /// Reference to retry number limitation
    pub fn limitation(&self) -> Option<&RetryLimitation> {
        self.limitation.as_ref()
    }

    /// Get the delay future is some, based in this policy and attempt number.
    ///
    /// `None` will be returned if there is no budget for retries left.
    pub fn next_backoff(&mut self, attempt: usize) -> Option<Sleep> {
        if let Some(limitation) = &self.limitation {
            limitation
                .withdraw(attempt)
                .then(|| self.backoff.next_backoff())
        } else {
            Some(self.backoff.next_backoff())
        }
    }

    /// Register successful request.
    ///
    /// This affects the retry budget, if used.
    pub fn success(&self) {
        if let Some(limitation) = &self.limitation {
            limitation.deposit();
        }
    }
}

/// Canton-specific retry policy
///
/// This only applies to decoded Canton errors. If the error cannot be decoded, generic policy will
/// be applied.
#[derive(Clone, Debug)]
pub enum CantonRetryPolicy {
    /// Apply the fixed policy to all retryable errors. Retry-ability is determined by category ID
    /// of the decoded error.
    Fixed(RetryPolicy),

    /// Apply the policy defined by the decoded error. The policy is determined by:
    ///
    /// - first check `RetryInfo` in error details
    /// - if it's not set, fallback to default retry policy for this category ID
    /// - if it's not set either, no retry will be performed
    ///
    /// If limitation is set to `None`, infinite number of retries may be performed.
    ///
    /// This is generally more preferable than [`Fixed`](Self::Fixed).
    Auto(Option<RetryLimitation>),
}

impl CantonRetryPolicy {
    /// Create new policy with fixed retry policy.
    pub fn fixed(policy: RetryPolicy) -> Self {
        Self::Fixed(policy)
    }

    /// Create new auto policy with infinite number of retries.
    pub fn auto() -> Self {
        Self::Auto(None)
    }

    /// Create new auto policy with a limitation on number of retries.
    pub fn auto_with_limitation(limitation: RetryLimitation) -> Self {
        Self::Auto(Some(limitation))
    }

    /// Create new auto policy with retry budget
    pub fn auto_with_budget(budget: Arc<TpsBudget>) -> Self {
        Self::Auto(Some(RetryLimitation::Tps(budget)))
    }

    /// Create new auto policy with fixed max number of retries
    pub fn auto_with_max(max_retries: usize) -> Self {
        Self::Auto(Some(RetryLimitation::Fixed(max_retries)))
    }

    /// Reference to retry number limitation
    pub fn limitation(&self) -> Option<&RetryLimitation> {
        match self {
            Self::Fixed(retry_policy) => retry_policy.limitation(),
            Self::Auto(retry_limitation) => retry_limitation.as_ref(),
        }
    }

    /// Register successful request.
    ///
    /// This affects the retry budget, if used.
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
