use std::{collections::HashMap, fmt, time::Duration};

use thiserror::Error;
use tonic::{Code, Status};
use tonic_types::{ErrorDetail, ErrorInfo, ResourceInfo, StatusExt as _};

use crate::error::{CategoryId, ErrorCodeId};

#[derive(Clone, Debug, Error)]
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
#[derive(Clone, Debug, Error)]
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
}
