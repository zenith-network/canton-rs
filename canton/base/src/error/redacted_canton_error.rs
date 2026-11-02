use tonic::{Code, Status};
use tonic_types::StatusExt as _;

/// Redacted error returned from Canton node.
///
/// We treat this error as always be non-retryable, so this does not implement
/// [`RetryableError`][crate::retry::RetryableError].
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message} (error message was redacted)")]
pub struct RedactedCantonError {
    code: Code,
    message: String,
    request_id: Option<String>,
}

impl RedactedCantonError {
    /// Prefix of redacted error message.
    pub const PREFIX: &str =
        "An error occurred. Please contact the operator and inquire about the request";

    /// Construct redacted error from [`tonic::Status`].
    ///
    /// Returns `None` if the message doesn't start with a designated prefix [`Self::PREFIX`].
    pub fn from_status(status: &Status) -> Option<Self> {
        // We detect a redacted error by prefix only
        // This is similar to how Scala client is currently working
        if status.message().starts_with(RedactedCantonError::PREFIX) {
            let message = status.message().to_owned();
            let request_id = status
                .get_details_request_info()
                .map(|request_info| request_info.request_id);
            Some(Self {
                code: status.code(),
                message,
                request_id,
            })
        } else {
            None
        }
    }

    /// Original gRPC code from [`tonic`].
    pub fn code(&self) -> Code {
        self.code
    }

    /// Original error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Request ID.
    ///
    /// This is either a correlation ID or trace ID.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use tonic::{Code, Status};

    use crate::error::test_fixtures::{self as fixtures, CORRELATION_ID, TRACE_ID};

    use super::RedactedCantonError;

    #[rstest]
    #[case::no_ids(
        None,
        None,
        "An error occurred. Please contact the operator and inquire about the request <no-correlation-id> with tid <no-tid>",
        None
    )]
    #[case::correlation_only(
        Some(CORRELATION_ID),
        None,
        "An error occurred. Please contact the operator and inquire about the request 1234567890abcdef1234567890abcdef with tid <no-tid>",
        Some(CORRELATION_ID)
    )]
    #[case::trace_only(
        None,
        Some(TRACE_ID),
        "An error occurred. Please contact the operator and inquire about the request <no-correlation-id> with tid fedcba9876543210fedcba9876543210",
        Some(TRACE_ID)
    )]
    #[case::both_ids(
        Some(CORRELATION_ID),
        Some(TRACE_ID),
        "An error occurred. Please contact the operator and inquire about the request 1234567890abcdef1234567890abcdef with tid fedcba9876543210fedcba9876543210",
        Some(CORRELATION_ID)
    )]
    fn from_status_decodes_missing_jwt(
        #[case] correlation_id: Option<&str>,
        #[case] trace_id: Option<&str>,
        #[case] message: &str,
        #[case] request_id: Option<&str>,
    ) {
        let status = fixtures::missing_jwt(correlation_id, trace_id);
        let error =
            RedactedCantonError::from_status(&status).expect("expected redacted Canton error");

        assert_eq!(error.code(), Code::Unauthenticated);
        assert_eq!(error.message(), message);
        assert_eq!(error.request_id(), request_id);
    }

    #[test]
    fn from_status_decodes_serialization_failure_without_request_info() {
        let status = fixtures::status_serialization_failure();
        let error =
            RedactedCantonError::from_status(&status).expect("expected redacted Canton error");

        assert_eq!(error.code(), Code::Internal);
        assert_eq!(
            error.message(),
            "An error occurred. Please contact the operator and inquire about the request 1234567890abcdef1234567890abcdef with tid fedcba9876543210fedcba9876543210",
        );
        assert_eq!(error.request_id(), None);
    }

    #[rstest]
    #[case::rich(fixtures::missing_field())]
    #[case::raw(fixtures::subscription_pool_closed())]
    fn from_status_rejects_status_without_redacted_prefix(#[case] status: Status) {
        assert!(RedactedCantonError::from_status(&status).is_none());
    }
}
