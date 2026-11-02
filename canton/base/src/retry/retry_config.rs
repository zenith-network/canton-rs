use std::sync::Arc;

use tower::retry::budget::Budget as _;

use crate::retry::{
    RetryHandler,
    policies::{CantonRetryPolicy, RetryLimitation, RetryPolicy},
};

/// Configuration of retry policies
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

    /// Set Canton-specific retry policy
    ///
    /// Note that due to how error parsing happens, Canton-specific policy will take precedence over
    /// generic networking policy.
    pub fn with_canton_policy(mut self, policy: Option<CantonRetryPolicy>) -> Self {
        self.canton = policy;
        self
    }

    /// Set generic networking retry policy
    pub fn with_network_policy(mut self, policy: Option<RetryPolicy>) -> Self {
        self.network = policy;
        self
    }

    /// Create a handler from this config.
    pub fn into_handler(self) -> RetryHandler {
        RetryHandler::from_config(self)
    }

    /// Reference to generic networking retry policy
    pub fn network(&self) -> Option<&RetryPolicy> {
        self.network.as_ref()
    }

    /// Mutable reference to generic networking retry policy
    pub fn network_mut(&mut self) -> Option<&mut RetryPolicy> {
        self.network.as_mut()
    }

    /// Reference to Canton-specific retry policy
    pub fn canton(&self) -> Option<&CantonRetryPolicy> {
        self.canton.as_ref()
    }

    /// Mutable reference to Canton-specific retry policy
    pub fn canton_mut(&mut self) -> Option<&mut CantonRetryPolicy> {
        self.canton.as_mut()
    }

    /// Register successful request
    ///
    /// This updates the underlying retry budget, if used.
    pub fn success(&self) {
        match (&self.canton, &self.network) {
            (None, None) => {}
            (None, Some(network)) => network.success(),
            (Some(canton), None) => canton.success(),

            // If canton and network policies share the same budget, it has to be deposited
            // only once
            (Some(canton), Some(network))
                if let (
                    Some(RetryLimitation::Tps(canton_budget)),
                    Some(RetryLimitation::Tps(network_budget)),
                ) = (canton.limitation(), network.limitation())
                    && Arc::ptr_eq(canton_budget, network_budget) =>
            {
                canton_budget.deposit();
            }

            // Otherwise it's safe to just call success() on both
            (Some(canton), Some(network)) => {
                canton.success();
                network.success();
            }
        }
    }
}
