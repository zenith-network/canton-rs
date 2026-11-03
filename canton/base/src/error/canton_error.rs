use tokio::time::Sleep;
use tonic::Status;

use crate::{
    error::{DecodedCantonError, RedactedCantonError},
    retry::{RetryConfig, RetryCtx, RetryableError},
};

/// Base Canton error
#[derive(Clone, Debug, thiserror::Error)]
pub enum CantonError {
    /// This variant is a proper enriched error returned from Canton
    #[error("Canton node returned error")]
    Decoded(#[from] DecodedCantonError),

    /// This variant is a redacted error returned from Canton
    ///
    /// [Canton docs] say:
    ///
    /// > Some errors are redacted for security. The API response omits sensitive details, but the
    /// > full error message appears in server-side logs. Work with your operator if you need the
    /// > complete error context.
    ///
    /// [Canton docs]: https://docs.canton.network/appdev/reference/error-codes
    #[error("Canton node returned redacted error (sensitive details were omitted)")]
    Redacted(#[from] RedactedCantonError),

    /// This error variant is returned when the client failed to properly parse an error returned by
    /// Canton
    #[error("gRPC request failed")]
    Raw(#[source] Status),
}

impl CantonError {
    /// Create base Canton error from `tonic` gRPC status
    pub fn from_status(status: Status) -> Self {
        if let Some(error) = DecodedCantonError::from_status(&status) {
            return Self::Decoded(error);
        }

        // If the error cannot be decoded, it may be a redacted error - they don't have
        // error info
        if let Some(error) = RedactedCantonError::from_status(&status) {
            return Self::Redacted(error);
        }

        // Fallback to raw gRPC status
        Self::Raw(status)
    }
}

impl From<Status> for CantonError {
    fn from(value: Status) -> Self {
        Self::from_status(value)
    }
}

impl RetryableError for CantonError {
    fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
        match self {
            Self::Decoded(decoded_canton_error) => decoded_canton_error.retry_after(config, ctx),
            Self::Raw(status) => status.retry_after(config, ctx),

            // We consider all redacted errors as non retryable now.
            // Although it's not strictly enforced anywhere, there is no reasonable way to perform
            // an automatic retry in this case now.
            Self::Redacted(_) => None,
        }
    }

    fn set_ctx(&mut self, ctx: &RetryCtx) {
        match self {
            CantonError::Decoded(decoded_canton_error) => decoded_canton_error.set_ctx(ctx),
            CantonError::Redacted(_) => {}
            CantonError::Raw(status) => status.set_ctx(ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::error::test_fixtures as fixtures;
    use crate::retry::{CantonRetryPolicy, RetryConfig, RetryHandler};

    use super::CantonError;

    #[tokio::test(start_paused = true)]
    async fn retry_handler_records_attempt_on_decoded_error() {
        let handler = RetryHandler::from_config(
            RetryConfig::new().with_canton_policy(Some(CantonRetryPolicy::auto_with_max(2))),
        );
        let calls = Cell::new(0);

        let result: Result<(), CantonError> = handler
            .call(&(), &(), |(), ()| {
                calls.set(calls.get() + 1);
                async { Err(fixtures::submission_already_in_flight().into()) }
            })
            .await;

        assert_eq!(calls.get(), 3);
        let CantonError::Decoded(error) = result.unwrap_err() else {
            panic!("expected decoded Canton error");
        };
        assert_eq!(error.attempt(), 2);
    }

    #[test]
    fn from_status_selects_decoded() {
        let error = CantonError::from_status(fixtures::missing_field());

        assert!(matches!(error, CantonError::Decoded(_)));
    }

    #[test]
    fn from_status_selects_redacted() {
        let error = CantonError::from_status(fixtures::missing_jwt(None, None));

        assert!(matches!(error, CantonError::Redacted(_)));
    }

    #[test]
    fn from_status_selects_raw() {
        let error = CantonError::from_status(fixtures::subscription_pool_closed());

        assert!(matches!(error, CantonError::Raw(_)));
    }
}
