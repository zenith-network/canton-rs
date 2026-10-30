use thiserror::Error;
use tonic::{Code, Status};
use tonic_types::StatusExt as _;

/// Redacted error returned from Canton node.
#[derive(Clone, Debug, Error)]
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
