use std::{collections::HashMap, fmt, time::Duration};

use ledger_api_types::{canton_types::LedgerString, value::v2::errors::ValueError};
use thiserror::Error;
use tonic_types::{ErrorDetail, ErrorInfo, StatusExt as _};

mod category_id;
mod error_code_id;

pub use category_id::{CategoryId, Code, UnknownCategoryId};
pub use error_code_id::ErrorCodeId;

// Re-export for convenience, because it's exposed in public API here
pub use tonic::Status;
pub use tonic_types::ResourceInfo;

/// Error during building a Canton client
#[derive(Debug, Error)]
pub enum ClientBuildError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
}

/// Error during interaction with Canton API
#[derive(Debug, Error)]
pub enum CantonError {
    /// This variant is a proper enriched error returned from Canton gRPC API
    #[error("Ledger API returned an error")]
    Decoded(#[from] DecodedCantonError),

    /// This variant is actually a special case of decoded error, when the error is `DAML_FAILURE`
    ///
    /// Added here for convenience in handling Daml-originated errors.
    /// Daml interpretation errors are always considered non-retryable to this variant is always
    /// rejected by any retry policy.
    #[error("Daml interpretation failed")]
    Daml(#[from] DamlFailure),

    /// This variant is a redacted error returned from Ledger API
    ///
    /// [Canton docs] say:
    ///
    /// > Some errors are redacted for security. The API response omits sensitive details, but the
    /// full error message appears in server-side logs. Work with your operator if you need the
    /// complete error context.
    ///
    /// [Canton docs]: https://docs.canton.network/appdev/reference/error-codes
    #[error("Ledger API returned a redacted error (sensitive details were omitted)")]
    Redacted(#[from] RedactedCantonError),

    /// This error variant is returned when the client failed to properly parse an error returned by
    /// Ledger API
    #[error("failed to query Ledger API")]
    Raw(#[source] Status),

    /// This error occurs when the response was received, but it failed to be properly parsed by
    /// the client
    #[error("failed to parse response from Ledger API")]
    ValueError(#[from] ValueError),
}

impl CantonError {
    pub fn value_error(error: impl Into<ValueError>) -> Self {
        Self::ValueError(error.into())
    }
}

impl From<Status> for CantonError {
    fn from(status: Status) -> Self {
        if let Some(error) = DecodedCantonError::from_status(&status) {
            // Here we try to concretize the error as Daml failure
            match DamlFailure::from_decoded(error) {
                // This is actually a Daml interpretation error
                Ok(error) => Self::Daml(error),

                // Fallback to generic decoded error
                Err(error) => Self::Decoded(error),
            }
        } else if let Some(error) = RedactedCantonError::from_status(&status) {
            // If the error cannot be decoded, it may be a redacted error - they don't have
            // error info
            Self::Redacted(error)
        } else {
            // Fallback to raw gRPC status
            Self::Raw(status)
        }
    }
}

/// Redacted error returned from Ledger gRPC API
#[derive(Clone, Debug, Error)]
#[error("{message} (error message was redacted)")]
pub struct RedactedCantonError {
    code: Code,
    message: String,
    request_id: Option<String>,
}

impl RedactedCantonError {
    /// Prefix of redacted error message
    pub const PREFIX: &str =
        "An error occurred. Please contact the operator and inquire about the request";

    /// Construct redacted error from [`tonic::Status`]
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

    /// Original gRPC code from [`tonic`]
    pub fn code(&self) -> Code {
        self.code
    }

    /// Original error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Request ID
    ///
    /// This is either a correlation ID or trace ID.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_ref().map(String::as_str)
    }
}

/// Specific type of Canton error, which represents a failure on Daml interpretation
///
/// Can be constructed from `DecodedCantonError`, if the error code ID is `DAML_FAILURE`.
#[derive(Clone, Debug, Error)]
pub struct DamlFailure {
    code: Code,
    category_id: CategoryId,
    error_id: Option<String>,
    correlation_id: Option<String>,
    trace_id: Option<String>,
    full_message: String,
    message: Option<String>,
    failure_message: Option<String>,
    resources: Vec<ResourceInfo>,
    metadata: HashMap<String, String>,
}

impl DamlFailure {
    /// On error returns original `decoded` error value
    pub fn from_decoded(decoded: DecodedCantonError) -> Result<Self, DecodedCantonError> {
        if matches!(decoded.error_code_id, ErrorCodeId::DamlFailure) {
            let DecodedCantonError {
                code,
                category_id,
                correlation_id,
                trace_id,
                full_message,
                message,
                resources,
                mut metadata,
                ..
            } = decoded;
            let error_id = metadata.remove("error_id");

            let mut failure_message = None;
            if let Some(msg) = &message {
                // Assuming format like:
                //  "User failure: <error_id> (error category <category>): <FailureStatus.message>"
                if let Some((_, rest)) = msg.split_once(':') {
                    if let Some((_, msg)) = rest.split_once(':') {
                        failure_message = Some(msg.to_owned());
                    }
                }
            }

            Ok(Self {
                code,
                category_id,
                error_id,
                correlation_id,
                trace_id,
                full_message,
                message,
                failure_message,
                resources,
                metadata,
            })
        } else {
            Err(decoded)
        }
    }

    /// Original gRPC code from [`tonic`]
    pub fn code(&self) -> Code {
        self.code
    }

    /// Small integer identifying the corresponding error category.
    ///
    /// Note: In the current Daml SDK v3.6, there are only 2 categories returned here:
    ///
    /// - [`CategoryId::InvalidIndependentOfSystemState`]
    /// - [`CategoryId::InvalidGivenCurrentSystemStateOther`]
    ///
    /// See [`DA.Internal.Fail.Types.FailureCategory`][FailureCategory].
    ///
    /// Note that both of these categories are non-retryable.
    ///
    /// [FailureCategory]: https://docs.canton.network/appdev/reference/daml-standard-library/da-fail#data-failurecategory
    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    /// Application-defined error ID
    ///
    /// Note: In Daml SDK v3.6 error ID is required
    /// (see [`DA.Internal.Fail.Types.FailureStatus`][FailureStatus]), but error metadata may be
    /// truncated on the way. That's why this returns `Option`.
    ///
    /// [FailureStatus]: https://docs.canton.network/appdev/reference/daml-standard-library/da-fail#data-failurestatus
    pub fn error_id(&self) -> Option<&str> {
        self.error_id.as_ref().map(String::as_str)
    }

    /// Correlation ID
    ///
    /// This can be correlation ID or trace ID.
    ///
    /// Note: Although this field is documented as mandatory, in the current implementation
    /// Canton 3.6.0 it may be missing if both correlation ID and trace ID are not provided.
    /// For that reason this method returns `Option`.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_ref().map(String::as_str)
    }

    /// Trace ID
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_ref().map(String::as_str)
    }

    /// Full original error message from [`tonic::Status`]
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// DAML_FAILURE(9,ffffffff): User failure: my.app/balance-err (error category 9): Balance too low
    /// ```
    pub fn full_message(&self) -> &str {
        &self.full_message
    }

    /// User-defined part of full message
    ///
    /// If the original message was following `NAME(CN,x): MSG` format, this will return `MSG`.
    /// Otherwise fallback to `None`.
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// User failure: my.app/balance-err (error category 9): Balance too low
    /// ```
    pub fn message(&self) -> Option<&str> {
        self.message.as_ref().map(String::as_str)
    }

    /// "Best-effort" parsed message from Daml application
    ///
    /// If the original message was following
    /// `NAME(CN,x): User failure: ERROR_ID (error category CAT): MSG`
    /// format, then this will contain only `MSG`. `MSG` is equal to application-defined
    /// [`FailureStatus.message`][message] in this case. Otherwise this will be `None`.
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// Balance too low
    /// ```
    ///
    /// [message]: https://docs.canton.network/appdev/reference/daml-standard-library/da-fail#param-message
    pub fn failure_message(&self) -> Option<&str> {
        self.failure_message.as_ref().map(String::as_str)
    }

    /// Return [`Self::failure_message()`] if it is `Some`. If not, check [`Self::message`].
    /// Then fallback to [`Self::full_message`].
    pub fn human_message(&self) -> &str {
        if let Some(failure_message) = &self.failure_message {
            failure_message
        } else if let Some(message) = &self.message {
            message
        } else {
            &self.full_message
        }
    }

    /// Identifies the resources involved in the failure (contract, contract key, package, party,
    /// synchronizer, etc.)
    pub fn resources(&self) -> &[ResourceInfo] {
        &self.resources
    }

    /// Additional metadata provided through [`FailureStatus.meta`][meta]
    ///
    /// [meta]: https://docs.canton.network/appdev/reference/daml-standard-library/da-fail#param-meta
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}

impl fmt::Display for DamlFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.failure_message {
            write!(f, "{message} (category: {:#})", self.category_id)
        } else if let Some(message) = &self.message {
            write!(f, "{message}")
        } else {
            write!(f, "{}", self.full_message)
        }
    }
}

/// Parsed error returned from Ledger gRPC API
///
/// This is a parsed version of [`tonic::Status`], using [gRPC Richer Error Model].
///
/// For more info see [Error Codes] and [Error Code Reference] in Canton docs.
///
/// [gRPC Richer Error Model]: https://google.aip.dev/193
/// [Error Codes]: https://docs.canton.network/appdev/reference/error-codes
/// [Error Code Reference]: https://docs.canton.network/global-synchronizer/reference/error-codes
#[derive(Clone, Debug, Error)]
pub struct DecodedCantonError {
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
            .map(|v| v.parse::<bool>().ok())
            .flatten();

        let correlation_id = status
            .get_details_request_info()
            .map(|request_info| request_info.request_id);

        let retry_delay = status
            .get_details_retry_info()
            .map(|retry_info| retry_info.retry_delay)
            .flatten();

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
        })
    }

    /// Original gRPC code from [`tonic`]
    pub fn code(&self) -> Code {
        self.code
    }

    /// Error code ID.
    pub fn error_code_id(&self) -> &ErrorCodeId {
        &self.error_code_id
    }

    /// Small integer identifying the corresponding error category.
    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    /// Indicates whether Canton knows the command’s outcome conclusively for deduplication.
    ///
    /// - `true` means the rejection is definitive, Canton knows that command was not accepted;
    /// - `false` means the outcome may be uncertain and command processing may have succeeded
    /// despite returned error
    /// - `None` means that the concept doesn't apply to the error type
    ///
    /// Mainly used for command-related RPC methods.
    ///
    /// Note: this is not a decisive factor for retry policy.
    ///
    /// # Examples
    ///
    /// - If request timed out before observing the results, the error will be returned and `false`
    /// will be set. User may check, whether the command actually succeeded or not, before applying
    /// retry policies.
    /// - If `DUPLICATE_COMMAND` is returned, Canton knows for sure that this call failed and `true`
    /// will be set.
    pub fn definite_answer(&self) -> Option<bool> {
        self.definite_answer
    }

    /// Correlation ID
    ///
    /// This can be correlation ID or trace ID.
    ///
    /// Note: Although this field is documented as mandatory, in the current implementation
    /// Canton 3.6.0 it may be missing if both correlation ID and trace ID are not provided.
    /// For that reason this method returns `Option`.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_ref().map(String::as_str)
    }

    /// Trace ID
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_ref().map(String::as_str)
    }

    /// Full original error message from [`tonic::Status`]
    ///
    /// ## Example
    ///
    /// ```plaintext
    /// DUPLICATE_COMMAND(10,ffffffff): A command with the given command id has already been successfully processed
    /// ```
    pub fn full_message(&self) -> &str {
        &self.full_message
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
        self.message.as_ref().map(String::as_str)
    }

    /// Returns [`Self::message()`] if it is `Some`, otherwise fallback to [`Self::full_message`]
    pub fn human_message(&self) -> &str {
        if let Some(message) = &self.message {
            message
        } else {
            &self.full_message
        }
    }

    /// Recommended retry interval when the error is retryable
    pub fn retry_delay(&self) -> Option<Duration> {
        self.retry_delay
    }

    /// Identifies the resources involved in the failure (contract, contract key, package, party,
    /// synchronizer, etc.)
    pub fn resources(&self) -> &[ResourceInfo] {
        &self.resources
    }

    /// Metadata of the error info
    ///
    /// This map doesn't contain extracted fields: `category`, `tid`, `definite_answer`.
    /// These can be accessed from corresponding methods.
    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Completion offset extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the completion offset
    /// of the succeeded command.
    ///
    /// Note: this will also return `None`, if completion offset failed to be parsed as `i64`
    pub fn completion_offset(&self) -> Option<i64> {
        self.metadata
            .get("completion_offset")
            .map(|offset| offset.parse::<i64>().ok())
            .flatten()
    }

    /// Existing submission ID extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the submission ID of the
    /// succeeded command.
    ///
    /// Note: this will also return `None`, if submission ID failed to be parsed as `LedgerString`
    pub fn existing_submission_id(&self) -> Option<LedgerString> {
        self.metadata
            .get("existing_submission_id")
            .cloned()
            .map(LedgerString::new)
            .transpose()
            .ok()
            .flatten()
    }

    /// Application-defined error ID inside `DAML_FAILURE`
    pub fn daml_error_id(&self) -> Option<&str> {
        self.metadata.get("daml_error_id").map(String::as_str)
    }
}

impl fmt::Display for DecodedCantonError {
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

#[cfg(test)]
mod tests {
    use super::CantonError;

    use prost::Message;
    use prost_types::Any;
    use std::collections::HashMap;
    use tonic::{Code, Status};
    use tonic_types::pb::{ErrorInfo, RequestInfo, ResourceInfo, RetryInfo, Status as RpcStatus};

    const REQ_ID: &str = "cor-id-12345679";
    const REDACTED_WITH_REQ: &str = "An error occurred. Please contact the operator and inquire about the request cor-id-12345679 with tid <no-tid>";
    const REDACTED_NO_REQ: &str = "An error occurred. Please contact the operator and inquire about the request <no-correlation-id> with tid <no-tid>";

    fn md(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn pack_any<M: Message>(type_url: &'static str, msg: &M) -> Any {
        Any {
            type_url: type_url.to_owned(),
            value: msg.encode_to_vec(),
        }
    }

    fn err(reason: &str, metadata: &[(&str, &str)]) -> Any {
        pack_any(
            "type.googleapis.com/google.rpc.ErrorInfo",
            &ErrorInfo {
                reason: reason.to_owned(),
                domain: String::new(),
                metadata: md(metadata),
            },
        )
    }

    fn req(id: &str) -> Any {
        pack_any(
            "type.googleapis.com/google.rpc.RequestInfo",
            &RequestInfo {
                request_id: id.to_owned(),
                serving_data: String::new(),
            },
        )
    }

    fn res(typ: &str, name: &str) -> Any {
        pack_any(
            "type.googleapis.com/google.rpc.ResourceInfo",
            &ResourceInfo {
                resource_type: typ.to_owned(),
                resource_name: name.to_owned(),
                owner: String::new(),
                description: String::new(),
            },
        )
    }

    fn retry_1s() -> Any {
        pack_any(
            "type.googleapis.com/google.rpc.RetryInfo",
            &RetryInfo {
                retry_delay: Some(prost_types::Duration {
                    seconds: 1,
                    nanos: 0,
                }),
            },
        )
    }

    fn ledger_status(code: Code, message: &str, details: Vec<Any>) -> Status {
        let rpc = RpcStatus {
            code: code as i32,
            message: message.to_owned(),
            details,
        };
        Status::with_details(code, message, rpc.encode_to_vec().into())
    }

    fn assert_parses_as_canton_decoded(input: Status) {
        use std::assert_matches;

        let output = CantonError::from(input);
        assert_matches!(output, CantonError::Decoded(..));
    }

    fn assert_parses_as_redacted(
        input: Status,
        expected_code: Code,
        expected_message: &str,
        expected_request_id: Option<&str>,
    ) {
        let CantonError::Redacted(error) = CantonError::from(input) else {
            panic!("expected a redacted Canton error");
        };

        assert_eq!(error.code(), expected_code);
        assert_eq!(error.message(), expected_message);
        assert_eq!(error.request_id(), expected_request_id);
    }

    #[test]
    fn parses_missing_field_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::InvalidArgument,
            "MISSING_FIELD(8,cor-id-1): The submitted command is missing a mandatory field: command_id",
            vec![
                err(
                    "MISSING_FIELD",
                    &[
                        ("category", "8"),
                        ("definite_answer", "false"),
                        ("field_name", "command_id"),
                    ],
                ),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_package_not_found_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::NotFound,
            "PACKAGE_NOT_FOUND(11,cor-id-1): Could not find package.",
            vec![
                err(
                    "PACKAGE_NOT_FOUND",
                    &[("category", "11"), ("definite_answer", "false")],
                ),
                req(REQ_ID),
                res(
                    "PACKAGE",
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                ),
            ],
        ));
    }

    #[test]
    fn parses_duplicate_command_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::AlreadyExists,
            "DUPLICATE_COMMAND(10,cor-id-1): A command with the given command id has already been successfully processed",
            vec![
                err(
                    "DUPLICATE_COMMAND",
                    &[("category", "10"), ("definite_answer", "false")],
                ),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_participant_pruned_data_accessed_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::FailedPrecondition,
            "PARTICIPANT_PRUNED_DATA_ACCESSED(9,cor-id-1): Active contracts request at offset 42 precedes pruned offset 17",
            vec![
                err(
                    "PARTICIPANT_PRUNED_DATA_ACCESSED",
                    &[
                        ("category", "9"),
                        ("definite_answer", "false"),
                        ("earliest_offset", "17"),
                    ],
                ),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_request_time_out_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::DeadlineExceeded,
            "REQUEST_TIME_OUT(3,cor-id-1): Timed out while awaiting for a completion corresponding to a command submission.",
            vec![
                err(
                    "REQUEST_TIME_OUT",
                    &[("category", "3"), ("definite_answer", "false")],
                ),
                retry_1s(),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_participant_backpressure_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::Aborted,
            "PARTICIPANT_BACKPRESSURE(2,cor-id-1): The participant is overloaded: Some buffer is full",
            vec![
                err(
                    "PARTICIPANT_BACKPRESSURE",
                    &[
                        ("category", "2"),
                        ("definite_answer", "false"),
                        ("reason", "Some buffer is full"),
                    ],
                ),
                retry_1s(),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_service_not_running_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::Unavailable,
            "SERVICE_NOT_RUNNING(1,cor-id-1): Command Service is not running.",
            vec![
                err(
                    "SERVICE_NOT_RUNNING",
                    &[
                        ("category", "1"),
                        ("definite_answer", "false"),
                        ("service_name", "Command Service"),
                    ],
                ),
                retry_1s(),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_unauthenticated_redacted_error() {
        assert_parses_as_redacted(
            ledger_status(Code::Unauthenticated, REDACTED_WITH_REQ, vec![req(REQ_ID)]),
            Code::Unauthenticated,
            REDACTED_WITH_REQ,
            Some(REQ_ID),
        );
    }

    #[test]
    fn parses_permission_denied_redacted_error() {
        assert_parses_as_redacted(
            ledger_status(Code::PermissionDenied, REDACTED_WITH_REQ, vec![req(REQ_ID)]),
            Code::PermissionDenied,
            REDACTED_WITH_REQ,
            Some(REQ_ID),
        );
    }

    #[test]
    fn parses_internal_redacted_error() {
        assert_parses_as_redacted(
            ledger_status(Code::Internal, REDACTED_NO_REQ, vec![]),
            Code::Internal,
            REDACTED_NO_REQ,
            None,
        );
    }

    #[test]
    fn parses_malformed_request_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::InvalidArgument,
            "MALFORMED_REQUEST(8,cor-id-1): Malformed request",
            vec![
                err(
                    "MALFORMED_REQUEST",
                    &[
                        ("category", "8"),
                        ("definite_answer", "false"),
                        (
                            "message",
                            "view size exceeds the configured maximum request size",
                        ),
                        (
                            "reason",
                            "MaxViewSizeExceeded(view size (bytes) = 35000, max request size configured (bytes) = 32768)",
                        ),
                    ],
                ),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_submission_already_in_flight_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::Aborted,
            "SUBMISSION_ALREADY_IN_FLIGHT(2,cor-id-1): A submission with the given change ID (user ID, command ID, actAs) and submission ID is already in flight",
            vec![
                err(
                    "SUBMISSION_ALREADY_IN_FLIGHT",
                    &[("category", "2"), ("definite_answer", "false")],
                ),
                retry_1s(),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_offset_after_ledger_end_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::OutOfRange,
            "OFFSET_AFTER_LEDGER_END(12,cor-id-1): Absolute offset (12345678) is after ledger end (42)",
            vec![
                err(
                    "OFFSET_AFTER_LEDGER_END",
                    &[("category", "12"), ("definite_answer", "false")],
                ),
                retry_1s(),
                req(REQ_ID),
            ],
        ));
    }

    #[test]
    fn parses_invalid_updates_page_token_error() {
        assert_parses_as_canton_decoded(ledger_status(
            Code::InvalidArgument,
            "INVALID_UPDATES_PAGE_TOKEN(8,cor-id-1): The submitted command contains an invalid page token. Tokens used in GetUpdatesPage requests must be taken from a valid GetUpdatesPageResponse and used with the same EventFormat settings, the same begin and end with the same Canton participant running the same Canton version. Next page token was generated by a different Canton version",
            vec![
                err(
                    "INVALID_UPDATES_PAGE_TOKEN",
                    &[("category", "8"), ("definite_answer", "false")],
                ),
                req(REQ_ID),
            ],
        ));
    }
}
