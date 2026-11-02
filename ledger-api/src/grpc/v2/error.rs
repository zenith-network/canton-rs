//! Ledger API errors

use std::fmt;

use canton_base::{
    error::{CantonError as BaseCantonError, DecodedCantonError, ErrorCodeId},
    retry::{RetryConfig, RetryCtx, RetryableError},
};
use canton_version::semver::Version;
use ledger_api_types::{canton_types::LedgerString, value::v2::errors::ValueError};
use thiserror::Error;
use tokio::time::Sleep;
use tonic::Status;

/// Error during building a Ledger API client
#[derive(Debug, Error)]
pub enum ClientBuildError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    /// Returned when building a client with `https://` schema without enabled TLS feature
    /// (e.g. `"tls-ring"`).
    #[error("HTTPS endpoint requires a TLS feature")]
    TlsNotSupported,
}

/// Extension of a [`DecodedCantonError`], providing Ledger API specific methods.
pub trait LedgerErrorExt {
    /// Completion offset extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the completion offset
    /// of the succeeded command.
    ///
    /// Note: this will also return `None`, if completion offset failed to be parsed as `i64`
    fn completion_offset(&self) -> Option<i64>;

    /// Existing submission ID extracted from `ErrorInfo` metadata
    ///
    /// If the error code ID is `DUPLICATE_COMMAND`, then this may be se to the submission ID of the
    /// succeeded command.
    ///
    /// Note: this will also return `None`, if submission ID failed to be parsed as `LedgerString`
    fn existing_submission_id(&self) -> Option<LedgerString>;

    /// Application-defined error ID inside `DAML_FAILURE`
    fn daml_error_id(&self) -> Option<&str>;
}

impl LedgerErrorExt for DecodedCantonError {
    fn completion_offset(&self) -> Option<i64> {
        self.metadata()
            .get("completion_offset")
            .and_then(|offset| offset.parse::<i64>().ok())
    }

    fn existing_submission_id(&self) -> Option<LedgerString> {
        self.metadata()
            .get("existing_submission_id")
            .cloned()
            .map(LedgerString::new)
            .transpose()
            .ok()
            .flatten()
    }

    fn daml_error_id(&self) -> Option<&str> {
        self.metadata().get("error_id").map(String::as_str)
    }
}

/// Error which is returned when client doesn't support the API version of the server.
#[derive(Clone, Debug, Error)]
pub struct UnsupportedServerVersion {
    server_version: Version,
}

impl UnsupportedServerVersion {
    pub fn new(server_version: Version) -> Self {
        Self { server_version }
    }

    /// Version of the server.
    ///
    /// Note: This is **not** a Ledger API schema version (like `v2`). It's a Canton release
    /// version, e.g. `3.6.0`.
    pub fn server_version(&self) -> &Version {
        &self.server_version
    }
}

impl fmt::Display for UnsupportedServerVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported server version: '{}' (supported: ",
            self.server_version
        )?;
        for (index, version_req) in crate::supported_versions().iter().enumerate() {
            if index > 0 {
                f.write_str(" OR ")?;
            }
            write!(f, "'{version_req}'")?;
        }
        f.write_str(")")
    }
}

/// Specific type of Canton error, which represents a failure on Daml interpretation
///
/// Can be constructed from [`DecodedCantonError`], if the error code ID is `DAML_FAILURE`.
#[derive(Clone, Debug, Error)]
pub struct DamlFailure {
    decoded: DecodedCantonError,
    failure_message: Option<String>,
}

impl DamlFailure {
    /// On error returns original `decoded` error value
    pub fn from_decoded(decoded: DecodedCantonError) -> Result<Self, DecodedCantonError> {
        if matches!(decoded.error_code_id(), ErrorCodeId::DamlFailure) {
            let mut failure_message = None;
            // Assuming format like:
            //  "User failure: <error_id> (error category <category>): <FailureStatus.message>"
            if let Some(msg) = &decoded.message()
                && let Some((_, rest)) = msg.split_once(':')
                && let Some((_, msg)) = rest.split_once(':')
            {
                failure_message = Some(msg.trim().to_owned());
            }

            Ok(Self {
                decoded,
                failure_message,
            })
        } else {
            Err(decoded)
        }
    }

    /// Reference to original decoded Canton error
    pub fn canton_error(&self) -> &DecodedCantonError {
        &self.decoded
    }

    /// Application-defined error ID
    ///
    /// Note: In Daml SDK v3.6 error ID is required
    /// (see [`DA.Internal.Fail.Types.FailureStatus`][FailureStatus]), but error metadata may be
    /// truncated on the way. That's why this returns `Option`.
    ///
    /// [FailureStatus]: https://docs.canton.network/appdev/reference/daml-standard-library/da-fail#data-failurestatus
    pub fn error_id(&self) -> Option<&str> {
        self.decoded.daml_error_id()
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
        self.failure_message.as_deref()
    }

    /// Return [`Self::failure_message()`] if it is `Some`. If not, fallback to underlying
    /// [`DecodedCantonError::human_message()`].
    pub fn human_message(&self) -> &str {
        if let Some(failure_message) = &self.failure_message {
            failure_message
        } else {
            self.decoded.human_message()
        }
    }
}

impl fmt::Display for DamlFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(message) = &self.failure_message {
            write!(f, "{message} (category: {:#})", self.decoded.category_id())
        } else if let Some(message) = &self.decoded.message() {
            write!(f, "{message}")
        } else {
            write!(f, "{}", self.decoded.full_message())
        }
    }
}

// TODO: rename this type to `LedgerApiError`
/// Error during interaction with Ledger API
#[derive(Debug, Error)]
pub enum CantonError {
    /// Base Canton error
    #[error(transparent)]
    CantonError(#[from] BaseCantonError),

    /// This variant is actually a special case of decoded error, when the error is `DAML_FAILURE`
    ///
    /// Added here for convenience in handling Daml-originated errors.
    /// Daml interpretation errors are always considered non-retryable.
    #[error("Daml interpretation failed")]
    Daml(#[from] DamlFailure),

    /// This error occurs when the response was received, but it failed to be properly parsed by
    /// the client
    #[error("failed to parse response from Ledger API")]
    ValueError(#[from] ValueError),

    /// This error is returned when client doesn't support the version of the server.
    #[error("client does not support the version of Canton node")]
    UnsupportedServerVersion(#[from] UnsupportedServerVersion),
}

impl CantonError {
    pub fn from_status(status: Status) -> Self {
        let base_error = BaseCantonError::from_status(status);
        if let BaseCantonError::Decoded(decoded) = base_error {
            // Here we try to concretize the error as Daml failure
            match DamlFailure::from_decoded(decoded) {
                // This is actually a Daml interpretation error
                Ok(error) => Self::Daml(error),

                // Fallback to generic decoded error
                Err(error) => Self::CantonError(BaseCantonError::Decoded(error)),
            }
        } else {
            Self::CantonError(base_error)
        }
    }

    pub fn value_error(error: impl Into<ValueError>) -> Self {
        Self::ValueError(error.into())
    }
}

impl From<Status> for CantonError {
    fn from(status: Status) -> Self {
        Self::from_status(status)
    }
}

impl RetryableError for CantonError {
    fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
        match self {
            Self::CantonError(canton_error) => canton_error.retry_after(config, ctx),

            // Daml interpretation errors (being a special case of decoded errors) are always
            // considered non-retryable
            Self::Daml(_) => None,

            // Always non-retryable
            Self::ValueError(_) | Self::UnsupportedServerVersion(_) => None,
        }
    }

    fn set_ctx(&mut self, ctx: &RetryCtx) {
        match self {
            Self::CantonError(canton_error) => canton_error.set_ctx(ctx),

            // These errors do not record retry context, since they are always non-retryable
            Self::Daml(_) | Self::ValueError(_) | Self::UnsupportedServerVersion(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use canton_base::error::DecodedCantonError;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};
    use tonic::{Code, Status};
    use tonic_types::{ErrorDetail, ErrorInfo, StatusExt as _};

    use super::{DamlFailure, LedgerErrorExt as _};

    const ERROR_ID: &str = "integration.canton-rs/rejected";
    const APPLICATION_MESSAGE: &str = "Controlled rejection: payload is unacceptable";

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

    #[fixture]
    fn duplicate_command_status() -> Status {
        // Canton: ConsistencyErrors.scala, DuplicateCommand.Reject.
        Status::with_error_details_vec(
            Code::AlreadyExists,
            "DUPLICATE_COMMAND(10,0): A command with the given command id has already been successfully processed",
            [error_info(
                "DUPLICATE_COMMAND",
                &[
                    ("category", "10"),
                    ("definite_answer", "false"),
                    ("existing_submission_id", "existing-submission-id"),
                ],
            )],
        )
    }

    fn daml_failure_status(code: Code, category: i32) -> Status {
        // Canton: engine/Error.scala, Interpretation.DamlException adds the prefix;
        // speedy/Pretty.scala, prettyDamlException renders the user failure;
        // CommandExecutionErrors.scala, Interpreter.FailureStatus.Reject adds metadata.
        // Application values match fixtures/daml/Integration.daml in ledger-api-integration.
        Status::with_error_details_vec(
            code,
            format!(
                "DAML_FAILURE({category},0): Interpretation error: Error: User failure: \
                 {ERROR_ID} (error category {category}): {APPLICATION_MESSAGE}",
            ),
            [error_info(
                "DAML_FAILURE",
                &[
                    ("category", &category.to_string()),
                    ("definite_answer", "false"),
                    ("error_id", ERROR_ID),
                    ("fixture", "Sample"),
                    ("detail", "controlled"),
                ],
            )],
        )
    }

    fn decoded_with_metadata(
        status: &Status,
        key: &str,
        value: Option<&str>,
    ) -> DecodedCantonError {
        let mut info = status
            .get_details_error_info()
            .expect("fixture must contain ErrorInfo");
        // Negative cases deliberately omit or corrupt metadata from a Canton fixture.
        if let Some(value) = value {
            info.metadata.insert(key.to_owned(), value.to_owned());
        } else {
            info.metadata.remove(key);
        }
        let status = Status::with_error_details_vec(
            status.code(),
            status.message(),
            [ErrorDetail::ErrorInfo(info)],
        );
        DecodedCantonError::from_status(&status).expect("expected decoded Canton error")
    }

    #[rstest]
    #[case::valid(Some("42"), Some(42))]
    #[case::missing(None, None)]
    #[case::nonnumeric(Some("not-an-offset"), None)]
    #[case::overflow(Some("9223372036854775808"), None)]
    fn completion_offset(#[case] offset: Option<&str>, #[case] expected: Option<i64>) {
        // Canton: CommandDeduplicationError.scala, DuplicateCommandReject reports
        // the completion offset as a decimal Long in its context.
        let status = Status::with_error_details_vec(
            Code::AlreadyExists,
            "DUPLICATE_COMMAND(10,0): Command submission already exists.",
            [error_info(
                "DUPLICATE_COMMAND",
                &[
                    ("category", "10"),
                    ("definite_answer", "true"),
                    ("completion_offset", "42"),
                ],
            )],
        );
        let error = decoded_with_metadata(&status, "completion_offset", offset);

        assert_eq!(error.completion_offset(), expected);
    }

    #[rstest]
    #[case::valid(Some("existing-submission-id"), Some("existing-submission-id"))]
    #[case::missing(None, None)]
    #[case::invalid(Some("invalid@submission"), None)]
    fn existing_submission_id(
        duplicate_command_status: Status,
        #[case] submission_id: Option<&str>,
        #[case] expected: Option<&str>,
    ) {
        let error = decoded_with_metadata(
            &duplicate_command_status,
            "existing_submission_id",
            submission_id,
        );

        assert_eq!(error.existing_submission_id().as_deref(), expected);
    }

    #[rstest]
    #[case::present(Some(ERROR_ID))]
    #[case::missing(None)]
    fn daml_error_id(#[case] error_id: Option<&str>) {
        let status = daml_failure_status(Code::FailedPrecondition, 9);
        let error = decoded_with_metadata(&status, "error_id", error_id);

        assert_eq!(error.daml_error_id(), error_id);
    }

    #[rstest]
    #[case::state_independent(Code::InvalidArgument, 8)]
    #[case::state_dependent(Code::FailedPrecondition, 9)]
    fn daml_failure_preserves_decoded_error(#[case] code: Code, #[case] category: i32) {
        let status = daml_failure_status(code, category);
        let decoded = DecodedCantonError::from_status(&status).unwrap();

        let failure = DamlFailure::from_decoded(decoded.clone()).unwrap();

        assert_eq!(failure.error_id(), Some(ERROR_ID));
        assert_eq!(
            failure.canton_error().full_message(),
            decoded.full_message()
        );
        assert_eq!(failure.canton_error().message(), decoded.message());
        assert_eq!(failure.canton_error().metadata(), decoded.metadata());
    }

    #[test]
    fn daml_failure_tolerates_missing_error_id() {
        let status = daml_failure_status(Code::FailedPrecondition, 9);
        let decoded = decoded_with_metadata(&status, "error_id", None);

        let failure = DamlFailure::from_decoded(decoded).unwrap();

        assert_eq!(failure.error_id(), None);
    }

    #[rstest]
    #[case::without_cause("DAML_FAILURE(9,0)")]
    #[case::without_first_separator("DAML_FAILURE(9,0): Interpretation error")]
    #[case::without_second_separator("DAML_FAILURE(9,0): Interpretation error: Error")]
    fn daml_failure_tolerates_truncated_message(#[case] message: &str) {
        let original = daml_failure_status(Code::FailedPrecondition, 9);
        // Deliberately truncate the source-backed fixture, retaining its ErrorInfo.
        let status = Status::with_error_details_vec(
            original.code(),
            message,
            original.get_error_details_vec(),
        );
        let decoded = DecodedCantonError::from_status(&status).unwrap();

        let failure = DamlFailure::from_decoded(decoded).unwrap();

        assert_eq!(failure.failure_message(), None);
    }

    #[rstest]
    fn daml_failure_returns_non_daml_error_unchanged(duplicate_command_status: Status) {
        let decoded = DecodedCantonError::from_status(&duplicate_command_status).unwrap();

        let returned = DamlFailure::from_decoded(decoded.clone()).unwrap_err();

        assert_eq!(returned.full_message(), decoded.full_message());
        assert_eq!(returned.metadata(), decoded.metadata());
    }

    #[rstest]
    #[case::state_independent(
        Code::InvalidArgument,
        8,
        "Controlled rejection: payload is unacceptable (category: invalid independent of system state)"
    )]
    #[case::state_dependent(
        Code::FailedPrecondition,
        9,
        "Controlled rejection: payload is unacceptable (category: invalid given current system state (other))"
    )]
    fn daml_failure_display_uses_application_message(
        #[case] code: Code,
        #[case] category: i32,
        #[case] expected: &str,
    ) {
        let status = daml_failure_status(code, category);
        let decoded = DecodedCantonError::from_status(&status).unwrap();
        // Isolate formatting from the known application-message extraction bug.
        let failure = DamlFailure {
            decoded,
            failure_message: Some(APPLICATION_MESSAGE.to_owned()),
        };

        assert_eq!(failure.to_string(), expected);
    }

    #[test]
    fn daml_failure_display_uses_decoded_message() {
        let original = daml_failure_status(Code::FailedPrecondition, 9);
        // Truncate the Canton fixture before the application message.
        let status = Status::with_error_details_vec(
            original.code(),
            "DAML_FAILURE(9,0): Interpretation error: Error",
            original.get_error_details_vec(),
        );
        let decoded = DecodedCantonError::from_status(&status).unwrap();
        let failure = DamlFailure::from_decoded(decoded).unwrap();

        assert_eq!(failure.to_string(), "Interpretation error: Error");
    }

    #[test]
    fn daml_failure_display_uses_full_message() {
        let original = daml_failure_status(Code::FailedPrecondition, 9);
        // Truncate the Canton fixture before any message separator.
        let status = Status::with_error_details_vec(
            original.code(),
            "DAML_FAILURE(9,0)",
            original.get_error_details_vec(),
        );
        let decoded = DecodedCantonError::from_status(&status).unwrap();
        let failure = DamlFailure::from_decoded(decoded).unwrap();

        assert_eq!(failure.to_string(), "DAML_FAILURE(9,0)");
    }

    #[test]
    #[ignore = "tracking issue: #38"]
    fn daml_failure_extracts_application_message_from_canton_error() {
        let status = daml_failure_status(Code::FailedPrecondition, 9);
        let decoded = DecodedCantonError::from_status(&status).unwrap();
        let failure = DamlFailure::from_decoded(decoded).unwrap();

        // Known regression: the parser leaves the "User failure" prefix in the message.
        // Keep this assertion failing until the parser handles Canton's full prefix.
        assert_eq!(
            failure.failure_message(),
            Some("Controlled rejection: payload is unacceptable"),
        );
    }
}
