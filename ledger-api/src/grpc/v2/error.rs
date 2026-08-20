use std::{collections::HashMap, error::Error as _, time::Duration};

use ledger_api_types::{canton_types::LedgerString, value::v2::errors::ValueError};
use thiserror::Error;
use tonic_types::StatusExt as _;

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
    CantonGrpc(#[from] CantonGrpcError),

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
        if status.source().is_some() {
            // this means that the error was sythesized and not directly returned from the server
        }
        if let Some(error) = CantonGrpcError::from_status(&status) {
            Self::CantonGrpc(error)
        } else {
            Self::Raw(status)
        }
    }
}

/// Error returned from Ledger gRPC API.
///
/// This is a parsed version of [`tonic::Status`], using gRPC Richer Error Model.
#[derive(Clone, Debug, Error)]
#[error("{message} (category: {category_id:#}, code: {error_code_id})")]
pub struct CantonGrpcError {
    error_code_id: ErrorCodeId,
    category_id: CategoryId,
    correlation_id: String,
    message: String,
    retry_delay: Option<Duration>,
    resource_info: Option<ResourceInfo>,
    metadata: HashMap<String, String>,
}

impl CantonGrpcError {
    /// Construct error from [`tonic::Status`].
    ///
    /// If the status doesn't match the expected format, returns `None`.
    /// See [Canton docs][https://docs.canton.network/appdev/reference/error-codes].
    pub fn from_status(status: &Status) -> Option<Self> {
        // ErrorInfo is documented as mandatory, therefore if not found, we return None
        let mut error_info = status.get_details_error_info()?;
        let error_code_id = ErrorCodeId::from_string(error_info.reason);
        let category_id = error_info
            .metadata
            .remove("category")?
            .parse::<i32>()
            .ok()?
            .into();

        let request_info = status.get_details_request_info()?;
        let correlation_id = request_info.request_id;

        let retry_delay = status
            .get_details_retry_info()
            .map(|retry_info| retry_info.retry_delay)
            .flatten();

        let (_, message) = status.message().split_once(':')?;

        Some(Self {
            error_code_id,
            category_id,
            correlation_id,
            message: message.trim().to_owned(),
            retry_delay,
            resource_info: status.get_details_resource_info(),
            metadata: error_info.metadata,
        })
    }

    /// Error code ID.
    pub fn error_code_id(&self) -> &ErrorCodeId {
        &self.error_code_id
    }

    /// Small integer identifying the corresponding error category.
    pub fn category_id(&self) -> CategoryId {
        self.category_id
    }

    /// Correlation ID
    pub fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    /// Message targeted at a human reader. Should never be parsed by applications, as the
    /// description might change in future releases to improve clarity.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Recommended retry interval when the error is retryable
    pub fn retry_delay(&self) -> Option<Duration> {
        self.retry_delay
    }

    /// Identifies the resource involved in the failure (contract, contract key, package, party,
    /// synchronizer, etc.)
    pub fn resource_info(&self) -> Option<&ResourceInfo> {
        self.resource_info.as_ref()
    }

    /// If the error code ID is `DUPLICATE_COMMAND`, then this is expected to be se to the
    /// completion offset of the succeeded command.
    ///
    /// Note: this will also return `None`, if completion offset failed to be parsed as `i64`
    pub fn completion_offset(&self) -> Option<i64> {
        self.metadata
            .get("completion_offset")
            .map(|offset| offset.parse::<i64>().ok())
            .flatten()
    }

    /// If the error code ID is `DUPLICATE_COMMAND`, then this is expected to be se to the
    /// submission ID of the succeeded command.
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

    fn assert_parses_as_canton_grpc(input: Status) {
        use std::assert_matches;

        let output = CantonError::from(input);
        assert_matches!(output, CantonError::CantonGrpc(..));
    }

    #[test]
    fn parses_missing_field_error() {
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
            Code::Unauthenticated,
            REDACTED_WITH_REQ,
            vec![req(REQ_ID)],
        ));
    }

    #[test]
    fn parses_permission_denied_redacted_error() {
        assert_parses_as_canton_grpc(ledger_status(
            Code::PermissionDenied,
            REDACTED_WITH_REQ,
            vec![req(REQ_ID)],
        ));
    }

    #[test]
    fn parses_internal_redacted_error() {
        assert_parses_as_canton_grpc(ledger_status(Code::Internal, REDACTED_NO_REQ, vec![]));
    }

    #[test]
    fn parses_malformed_request_error() {
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
        assert_parses_as_canton_grpc(ledger_status(
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
