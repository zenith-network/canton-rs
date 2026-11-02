//! Shared gRPC statuses for error decoding tests.

use std::{collections::HashMap, time::Duration};

use tonic::{Code, Status};
use tonic_types::{ErrorDetail, ErrorInfo, RequestInfo, ResourceInfo, RetryInfo, StatusExt as _};

pub const CORRELATION_ID: &str = "1234567890abcdef1234567890abcdef";
pub const TRACE_ID: &str = "fedcba9876543210fedcba9876543210";

fn error_info(reason: &str, metadata: &[(&str, &str)]) -> ErrorDetail {
    ErrorDetail::ErrorInfo(ErrorInfo::new(
        reason,
        "",
        metadata
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<HashMap<_, _>>(),
    ))
}

/// A request missing the mandatory transaction shape.
pub fn missing_field() -> Status {
    Status::with_error_details_vec(
        Code::InvalidArgument,
        "MISSING_FIELD(8,0): The submitted command is missing a mandatory field: transaction_shape",
        [error_info(
            "MISSING_FIELD",
            &[
                ("category", "8"),
                ("definite_answer", "false"),
                ("field_name", "transaction_shape"),
            ],
        )],
    )
}

/// A definite duplicate-command rejection with separate correlation and trace IDs.
pub fn duplicate_command() -> Status {
    Status::with_error_details_vec(
        Code::AlreadyExists,
        "DUPLICATE_COMMAND(10,12345678): A command with the given command id has already been successfully processed",
        [
            error_info(
                "DUPLICATE_COMMAND",
                &[
                    ("category", "10"),
                    ("definite_answer", "true"),
                    ("tid", TRACE_ID),
                    ("existing_submission_id", "existing-submission-id"),
                ],
            ),
            ErrorDetail::RequestInfo(RequestInfo::new(CORRELATION_ID, "")),
        ],
    )
}

/// Two missing parties in the default identity provider.
pub fn unknown_parties() -> Status {
    Status::with_error_details_vec(
        Code::NotFound,
        concat!(
            "UNKNOWN_RESOURCE(11,12345678): The submitted request refers to a non-existing PARTIES: ",
            "Provided parties have not been found in identity_provider_id=``: [alice,bob].",
        ),
        [
            error_info(
                "UNKNOWN_RESOURCE",
                &[("category", "11"), ("definite_answer", "false")],
            ),
            ErrorDetail::RequestInfo(RequestInfo::new(CORRELATION_ID, "")),
            ErrorDetail::ResourceInfo(ResourceInfo::new("PARTIES", "alice", "", "")),
            ErrorDetail::ResourceInfo(ResourceInfo::new("PARTIES", "bob", "", "")),
        ],
    )
}

/// An in-flight submission with a one-second retry delay and matching IDs.
pub fn submission_already_in_flight() -> Status {
    Status::with_error_details_vec(
        Code::Aborted,
        concat!(
            "SUBMISSION_ALREADY_IN_FLIGHT(2,fedcba98): ",
            "A submission with the given change ID (user ID, command ID, actAs) ",
            "and submission ID is already in flight",
        ),
        [
            error_info(
                "SUBMISSION_ALREADY_IN_FLIGHT",
                &[
                    ("category", "2"),
                    ("definite_answer", "false"),
                    ("tid", TRACE_ID),
                ],
            ),
            ErrorDetail::RetryInfo(RetryInfo::new(Some(Duration::from_secs(1)))),
            ErrorDetail::RequestInfo(RequestInfo::new(TRACE_ID, "")),
        ],
    )
}

/// A descending update stream without an end offset; no definite-answer metadata.
pub fn descending_order_missing_end() -> Status {
    Status::with_error_details_vec(
        Code::InvalidArgument,
        concat!(
            "DESCENDING_ORDER_MISSING_END(8,0): ",
            "end_inclusive is not provided when descending_order is true",
        ),
        [error_info(
            "DESCENDING_ORDER_MISSING_END",
            &[("category", "8")],
        )],
    )
}

/// Stale stream authorization with an explicit zero-second retry delay.
pub fn stale_stream_authorization() -> Status {
    Status::with_error_details_vec(
        Code::Aborted,
        "STALE_STREAM_AUTHORIZATION(2,0): Stale stream authorization. Retry quickly.",
        [
            error_info(
                "STALE_STREAM_AUTHORIZATION",
                &[("category", "2"), ("definite_answer", "false")],
            ),
            ErrorDetail::RetryInfo(RetryInfo::new(Some(Duration::ZERO))),
        ],
    )
}

fn redacted_message(correlation_id: Option<&str>, trace_id: Option<&str>) -> String {
    format!(
        "An error occurred. Please contact the operator and inquire about the request {} with tid {}",
        correlation_id.unwrap_or("<no-correlation-id>"),
        trace_id.unwrap_or("<no-tid>"),
    )
}

/// A redacted authentication failure; RequestInfo prefers the correlation ID.
pub fn missing_jwt(correlation_id: Option<&str>, trace_id: Option<&str>) -> Status {
    Status::with_error_details_vec(
        Code::Unauthenticated,
        redacted_message(correlation_id, trace_id),
        correlation_id
            .or(trace_id)
            .map(|request_id| ErrorDetail::RequestInfo(RequestInfo::new(request_id, ""))),
    )
}

/// An internal status serialization failure with IDs in the message and no details.
pub fn status_serialization_failure() -> Status {
    Status::with_error_details_vec(
        Code::Internal,
        redacted_message(Some(CORRELATION_ID), Some(TRACE_ID)),
        [],
    )
}

/// A closed subscription pool reported without a rich-status envelope.
pub fn subscription_pool_closed() -> Status {
    Status::unavailable("Subscription pool is closed.")
}
