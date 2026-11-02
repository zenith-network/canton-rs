use std::{collections::HashMap, fmt, time::Duration};

use tokio::time::{self, Sleep};
use tonic::{Code, Status};
use tonic_types::{ErrorDetail, ErrorInfo, ResourceInfo, StatusExt as _};

use crate::{
    error::{CategoryId, ErrorCodeId},
    retry::{CantonRetryPolicy, RetryConfig, RetryCtx, RetryableError},
};

#[derive(Clone, Debug, thiserror::Error)]
struct ErrorInner {
    code: Code,
    error_code_id: ErrorCodeId,
    category_id: CategoryId,
    definite_answer: Option<bool>,
    correlation_id: Option<String>,
    trace_id: Option<String>,
    full_message: String,
    message: Option<String>,
    retry_delay: Option<Duration>,
    resources: Vec<ResourceInfo>,
    metadata: HashMap<String, String>,
    attempt: usize,
}

impl fmt::Display for ErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.message {
            write!(
                f,
                "{message} (category: {:#}, code: {})",
                self.category_id, self.error_code_id
            )
        } else {
            write!(f, "{}", self.full_message)
        }
    }
}

/// Parsed error returned from Canton node
///
/// This is a parsed version of [`tonic::Status`], using [gRPC Richer Error Model].
///
/// For more info see [Error Codes] and [Error Code Reference] in Canton docs.
///
/// [gRPC Richer Error Model]: https://google.aip.dev/193
/// [Error Codes]: https://docs.canton.network/appdev/reference/error-codes
/// [Error Code Reference]: https://docs.canton.network/global-synchronizer/reference/error-codes
#[derive(Clone, Debug, thiserror::Error)]
#[error(transparent)]
pub struct DecodedCantonError {
    // Allocate error content on heap to keep this type size small, to avoid issues when putting it
    // into enum variants
    inner: Box<ErrorInner>,
}

impl DecodedCantonError {
    /// Construct error from [`tonic::Status`].
    ///
    /// If the status doesn't match the expected format, returns `None`.
    pub fn from_status(status: &Status) -> Option<Self> {
        let code = status.code();

        // ErrorInfo is documented as mandatory, therefore if not found, we return None
        let ErrorInfo {
            reason,
            mut metadata,
            ..
        } = status.get_details_error_info()?;

        let error_code_id = ErrorCodeId::from_string(reason);
        let category_id = metadata.remove("category")?.parse::<i32>().ok()?.into();
        let trace_id = metadata.remove("tid");
        let definite_answer = metadata
            .remove("definite_answer")
            .and_then(|v| v.parse::<bool>().ok());

        let correlation_id = status
            .get_details_request_info()
            .map(|request_info| request_info.request_id);

        let retry_delay = status
            .get_details_retry_info()
            .and_then(|retry_info| retry_info.retry_delay);

        let resources = status
            .get_error_details_vec()
            .into_iter()
            .filter_map(|error_detail| match error_detail {
                ErrorDetail::ResourceInfo(resource_info) => Some(resource_info),
                _ => None,
            })
            .collect::<Vec<_>>();

        let full_message = status.message().to_owned();
        let message = full_message
            .split_once(':')
            .map(|(_, message)| message.trim().to_owned());

        Some(Self {
            inner: Box::new(ErrorInner {
                code,
                error_code_id,
                category_id,
                definite_answer,
                trace_id,
                correlation_id,
                full_message,
                message,
                retry_delay,
                resources,
                metadata,
                attempt: 0,
            }),
        })
    }

    /// Original gRPC code from [`tonic`]
    pub fn code(&self) -> Code {
        self.inner.code
    }

    /// Error code ID.
    pub fn error_code_id(&self) -> &ErrorCodeId {
        &self.inner.error_code_id
    }

    /// Small integer identifying the corresponding error category
    pub fn category_id(&self) -> CategoryId {
        self.inner.category_id
    }

    /// Indicates whether Canton knows the command’s outcome conclusively for deduplication.
    ///
    /// - `true` means the rejection is definitive, Canton knows that command was not accepted;
    /// - `false` means the outcome may be uncertain and command processing may have succeeded
    ///   despite returned error
    /// - `None` means that the concept doesn't apply to the error type
    ///
    /// Mainly used for command-related RPC methods.
    ///
    /// Note: this is not a decisive factor for retry policy.
    ///
    /// # Examples
    ///
    /// - If request timed out before observing the results, the error will be returned and `false`
    ///   will be set. User may check, whether the command actually succeeded or not, before
    ///   applying retry policies.
    /// - If `DUPLICATE_COMMAND` is returned, Canton knows for sure that this call failed and `true`
    ///   will be set.
    pub fn definite_answer(&self) -> Option<bool> {
        self.inner.definite_answer
    }

    /// Correlation ID
    ///
    /// This can be correlation ID or trace ID.
    ///
    /// Note: Although this field is documented as mandatory, in the current implementation
    /// Canton 3.6.0 it may be missing if both correlation ID and trace ID are not provided.
    /// For that reason this method returns `Option`.
    pub fn correlation_id(&self) -> Option<&str> {
        self.inner.correlation_id.as_deref()
    }

    /// Trace ID
    pub fn trace_id(&self) -> Option<&str> {
        self.inner.trace_id.as_deref()
    }

    /// Full original error message from [`tonic::Status`]
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// DUPLICATE_COMMAND(10,ffffffff): A command with the given command id has already been successfully processed
    /// ```
    pub fn full_message(&self) -> &str {
        &self.inner.full_message
    }

    /// "Best-effort" parsed message
    ///
    /// If the original message was following `NAME(CN,x): MSG` form, then this will contain only
    /// `MSG`. Otherwise this will be `None`.
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// A command with the given command id has already been successfully processed
    /// ```
    pub fn message(&self) -> Option<&str> {
        self.inner.message.as_deref()
    }

    /// Returns [`Self::message()`] if it is `Some`, otherwise fallback to [`Self::full_message`]
    pub fn human_message(&self) -> &str {
        if let Some(message) = &self.inner.message {
            message
        } else {
            &self.inner.full_message
        }
    }

    /// Recommended retry interval when the error is retryable
    pub fn retry_delay(&self) -> Option<Duration> {
        self.inner.retry_delay
    }

    /// Identifies the resources involved in the failure (contract, contract key, package, party,
    /// synchronizer, etc.)
    pub fn resources(&self) -> &[ResourceInfo] {
        &self.inner.resources
    }

    /// Metadata of the error info
    ///
    /// This map doesn't contain extracted fields: `category`, `tid`, `definite_answer`.
    /// These can be accessed from corresponding methods.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.inner.metadata
    }

    /// Attempt number, when this error occurred.
    ///
    /// When no retry was configured, this returns `0`.
    pub fn attempt(&self) -> usize {
        self.inner.attempt
    }
}

impl RetryableError for DecodedCantonError {
    fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
        config.canton_mut().and_then(|canton_policy| {
            // TODO: it may make sense to add jitter here
            let delay = || {
                self.retry_delay()
                    .or_else(|| self.category_id().retry())
                    .map(|duration| time::sleep(duration))
            };

            match canton_policy {
                CantonRetryPolicy::Fixed(policy) => {
                    self.category_id()
                        .retry()
                        .is_some() // checks that the error is considered to be retryable
                        .then(|| policy.next_backoff(ctx.attempt()))
                        .flatten()
                }
                CantonRetryPolicy::Auto(Some(limitation)) => {
                    let delay = delay()?;
                    limitation.withdraw(ctx.attempt()).then_some(delay)
                }
                CantonRetryPolicy::Auto(None) => delay(),
            }
        })
    }

    /// Sets attempt number for a decoded error.
    fn set_ctx(&mut self, ctx: &RetryCtx) {
        self.inner.attempt = ctx.attempt();
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use tonic::{Code, Status};
    use tonic_types::{ErrorDetail, StatusExt as _};

    use crate::error::{
        CategoryId, ErrorCodeId,
        test_fixtures::{self as fixtures, CORRELATION_ID, TRACE_ID},
    };
    use crate::retry::{
        CantonRetryPolicy, RetryConfig, RetryCtx, RetryableError,
        budget::{Budget as _, TpsBudget},
    };

    use super::DecodedCantonError;

    #[rstest]
    #[case::non_retryable(fixtures::missing_field(), false)]
    #[case::retryable(fixtures::submission_already_in_flight(), true)]
    #[tokio::test(start_paused = true)]
    async fn auto_policy_withdraws_budget_only_for_retries(
        #[case] status: Status,
        #[case] retryable: bool,
    ) {
        let budget = Arc::new(TpsBudget::new(Duration::from_secs(60), 0, 1.0));
        budget.deposit();
        budget.deposit();
        let mut config = RetryConfig::new()
            .with_canton_policy(Some(CantonRetryPolicy::auto_with_budget(budget.clone())));
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        let delay = error.retry_after(&mut config, &RetryCtx::new());

        assert_eq!(delay.is_some(), retryable);
        assert!(budget.withdraw(), "at least one token must remain");
        assert_eq!(budget.withdraw(), !retryable);
        assert!(!budget.withdraw());
    }

    #[test]
    fn from_status_decodes_missing_field() {
        let status = fixtures::missing_field();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::InvalidArgument);
        assert_eq!(error.error_code_id(), &ErrorCodeId::MissingField);
        assert_eq!(
            error.category_id(),
            CategoryId::InvalidIndependentOfSystemState
        );
        assert_eq!(error.definite_answer(), Some(false));
        assert_eq!(error.correlation_id(), None);
        assert_eq!(error.trace_id(), None);
        assert_eq!(
            error.full_message(),
            "MISSING_FIELD(8,0): The submitted command is missing a mandatory field: transaction_shape",
        );
        assert_eq!(
            error.message(),
            Some("The submitted command is missing a mandatory field: transaction_shape"),
        );
        assert_eq!(
            error.human_message(),
            "The submitted command is missing a mandatory field: transaction_shape",
        );
        assert_eq!(error.retry_delay(), None);
        assert!(error.resources().is_empty());
        assert_eq!(
            error.metadata(),
            &HashMap::from([("field_name".to_owned(), "transaction_shape".to_owned())]),
        );
        assert_eq!(error.attempt(), 0);
    }

    #[test]
    fn from_status_decodes_duplicate_command() {
        let status = fixtures::duplicate_command();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::AlreadyExists);
        assert_eq!(error.error_code_id(), &ErrorCodeId::DuplicateCommand);
        assert_eq!(
            error.category_id(),
            CategoryId::InvalidGivenCurrentSystemStateResourceExists,
        );
        assert_eq!(error.definite_answer(), Some(true));
        assert_eq!(error.correlation_id(), Some(CORRELATION_ID));
        assert_eq!(error.trace_id(), Some(TRACE_ID));
        assert_eq!(
            error.full_message(),
            "DUPLICATE_COMMAND(10,12345678): A command with the given command id has already been successfully processed",
        );
        assert_eq!(
            error.message(),
            Some("A command with the given command id has already been successfully processed"),
        );
        assert_eq!(
            error.human_message(),
            "A command with the given command id has already been successfully processed",
        );
        assert_eq!(error.retry_delay(), None);
        assert!(error.resources().is_empty());
        assert_eq!(
            error.metadata(),
            &HashMap::from([(
                "existing_submission_id".to_owned(),
                "existing-submission-id".to_owned(),
            )]),
        );
        assert_eq!(error.attempt(), 0);
    }

    #[test]
    fn from_status_decodes_unknown_parties_in_resource_order() {
        let status = fixtures::unknown_parties();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::NotFound);
        assert_eq!(error.error_code_id(), &ErrorCodeId::UnknownResource);
        assert_eq!(
            error.category_id(),
            CategoryId::InvalidGivenCurrentSystemStateResourceMissing,
        );
        assert_eq!(error.definite_answer(), Some(false));
        assert_eq!(error.correlation_id(), Some(CORRELATION_ID));
        assert_eq!(error.trace_id(), None);
        assert_eq!(
            error.full_message(),
            concat!(
                "UNKNOWN_RESOURCE(11,12345678): The submitted request refers to a non-existing PARTIES: ",
                "Provided parties have not been found in identity_provider_id=``: [alice,bob].",
            ),
        );
        assert_eq!(
            error.message(),
            Some(concat!(
                "The submitted request refers to a non-existing PARTIES: ",
                "Provided parties have not been found in identity_provider_id=``: [alice,bob].",
            )),
        );
        assert_eq!(
            error.human_message(),
            concat!(
                "The submitted request refers to a non-existing PARTIES: ",
                "Provided parties have not been found in identity_provider_id=``: [alice,bob].",
            ),
        );
        assert_eq!(error.retry_delay(), None);
        let resources: Vec<_> = error
            .resources()
            .iter()
            .map(|resource| {
                (
                    resource.resource_type.as_str(),
                    resource.resource_name.as_str(),
                    resource.owner.as_str(),
                    resource.description.as_str(),
                )
            })
            .collect();
        assert_eq!(
            resources,
            vec![("PARTIES", "alice", "", ""), ("PARTIES", "bob", "", "")],
        );
        assert_eq!(error.metadata(), &HashMap::new());
        assert_eq!(error.attempt(), 0);
    }

    #[test]
    fn from_status_decodes_submission_already_in_flight() {
        let status = fixtures::submission_already_in_flight();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::Aborted);
        assert_eq!(
            error.error_code_id(),
            &ErrorCodeId::SubmissionAlreadyInFlight
        );
        assert_eq!(error.category_id(), CategoryId::ContentionOnSharedResources);
        assert_eq!(error.definite_answer(), Some(false));
        assert_eq!(error.correlation_id(), Some(TRACE_ID));
        assert_eq!(error.trace_id(), Some(TRACE_ID));
        assert_eq!(
            error.full_message(),
            concat!(
                "SUBMISSION_ALREADY_IN_FLIGHT(2,fedcba98): ",
                "A submission with the given change ID (user ID, command ID, actAs) ",
                "and submission ID is already in flight",
            ),
        );
        assert_eq!(
            error.message(),
            Some(concat!(
                "A submission with the given change ID (user ID, command ID, actAs) ",
                "and submission ID is already in flight",
            )),
        );
        assert_eq!(
            error.human_message(),
            concat!(
                "A submission with the given change ID (user ID, command ID, actAs) ",
                "and submission ID is already in flight",
            ),
        );
        assert_eq!(error.retry_delay(), Some(Duration::from_secs(1)));
        assert!(error.resources().is_empty());
        assert_eq!(error.metadata(), &HashMap::new());
        assert_eq!(error.attempt(), 0);
    }

    #[test]
    fn from_status_decodes_descending_order_missing_end_without_definite_answer() {
        let status = fixtures::descending_order_missing_end();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::InvalidArgument);
        assert_eq!(
            error.error_code_id(),
            &ErrorCodeId::Unknown("DESCENDING_ORDER_MISSING_END".to_owned()),
        );
        assert_eq!(
            error.category_id(),
            CategoryId::InvalidIndependentOfSystemState
        );
        assert_eq!(error.definite_answer(), None);
        assert_eq!(error.correlation_id(), None);
        assert_eq!(error.trace_id(), None);
        assert_eq!(
            error.full_message(),
            concat!(
                "DESCENDING_ORDER_MISSING_END(8,0): ",
                "end_inclusive is not provided when descending_order is true",
            ),
        );
        assert_eq!(
            error.message(),
            Some("end_inclusive is not provided when descending_order is true"),
        );
        assert_eq!(
            error.human_message(),
            "end_inclusive is not provided when descending_order is true",
        );
        assert_eq!(error.retry_delay(), None);
        assert!(error.resources().is_empty());
        assert_eq!(error.metadata(), &HashMap::new());
        assert_eq!(error.attempt(), 0);
    }

    #[test]
    fn from_status_decodes_stale_stream_authorization_with_zero_retry_delay() {
        let status = fixtures::stale_stream_authorization();
        let error =
            DecodedCantonError::from_status(&status).expect("expected decoded Canton error");

        assert_eq!(error.code(), Code::Aborted);
        assert_eq!(
            error.error_code_id(),
            &ErrorCodeId::Unknown("STALE_STREAM_AUTHORIZATION".to_owned()),
        );
        assert_eq!(error.category_id(), CategoryId::ContentionOnSharedResources);
        assert_eq!(error.definite_answer(), Some(false));
        assert_eq!(error.correlation_id(), None);
        assert_eq!(error.trace_id(), None);
        assert_eq!(
            error.full_message(),
            "STALE_STREAM_AUTHORIZATION(2,0): Stale stream authorization. Retry quickly.",
        );
        assert_eq!(
            error.message(),
            Some("Stale stream authorization. Retry quickly."),
        );
        assert_eq!(
            error.human_message(),
            "Stale stream authorization. Retry quickly.",
        );
        assert_eq!(error.retry_delay(), Some(Duration::ZERO));
        assert!(error.resources().is_empty());
        assert_eq!(error.metadata(), &HashMap::new());
        assert_eq!(error.attempt(), 0);
    }

    #[rstest]
    #[case::redacted(fixtures::missing_jwt(Some(CORRELATION_ID), Some(TRACE_ID)))]
    #[case::raw(fixtures::subscription_pool_closed())]
    fn from_status_rejects_status_without_error_info(#[case] status: Status) {
        assert!(DecodedCantonError::from_status(&status).is_none());
    }

    #[test]
    #[should_panic]
    fn from_status_rejects_missing_error_info() {
        let original = fixtures::missing_field();
        // Deliberately malformed: remove the mandatory ErrorInfo from a real fixture.
        let status = Status::with_error_details_vec(original.code(), original.message(), []);

        DecodedCantonError::from_status(&status).unwrap();
    }

    #[rstest]
    #[case::missing(None)]
    #[case::nonnumeric(Some("not-a-number"))]
    #[case::overflow(Some("2147483648"))]
    #[should_panic(expected = "expected decoded Canton error")]
    fn from_status_rejects_invalid_category(#[case] category: Option<&str>) {
        let original = fixtures::missing_field();
        let mut info = original
            .get_details_error_info()
            .expect("fixture must contain ErrorInfo");
        // Deliberately malformed: omit or corrupt the mandatory category metadata.
        if let Some(category) = category {
            info.metadata
                .insert("category".to_owned(), category.to_owned());
        } else {
            info.metadata.remove("category");
        }
        let status = Status::with_error_details_vec(
            original.code(),
            original.message(),
            [ErrorDetail::ErrorInfo(info)],
        );

        DecodedCantonError::from_status(&status).expect("expected decoded Canton error");
    }
}
