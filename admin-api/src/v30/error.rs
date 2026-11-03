use admin_api_types::error::ValueError;
use canton_base::{
    error::CantonError,
    retry::{RetryConfig, RetryCtx, RetryableError},
    tokio::time::Sleep,
};
use tonic::Status;

/// Error while building Admin API client.
#[derive(Debug, thiserror::Error)]
pub enum ClientBuildError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    /// Returned when building a client with `https://` schema without enabled TLS feature
    /// (e.g. `"tls-ring"`).
    #[error("HTTPS endpoint requires a TLS feature")]
    TlsNotSupported,
}

/// Error during interaction with Admin API
#[derive(Debug, thiserror::Error)]
pub enum AdminApiError {
    /// Base Canton error
    #[error(transparent)]
    CantonError(#[from] CantonError),

    /// This error occurs when the response was received, but it failed to be properly parsed by
    /// the client
    #[error("failed to parse response from Admin API")]
    ValueError(#[from] ValueError),
}

impl AdminApiError {
    /// Create Admin API error from gRPC `tonic` status
    pub fn from_status(status: Status) -> Self {
        Self::CantonError(CantonError::from_status(status))
    }

    /// Create new value error
    pub fn value_error(error: impl Into<ValueError>) -> Self {
        Self::ValueError(error.into())
    }
}

impl From<Status> for AdminApiError {
    fn from(value: Status) -> Self {
        Self::from_status(value)
    }
}

impl RetryableError for AdminApiError {
    fn retry_after(&self, config: &mut RetryConfig, ctx: &RetryCtx) -> Option<Sleep> {
        match self {
            AdminApiError::CantonError(canton_error) => canton_error.retry_after(config, ctx),

            // Always non-retryable
            AdminApiError::ValueError(_) => None,
        }
    }

    fn set_ctx(&mut self, ctx: &RetryCtx) {
        match self {
            AdminApiError::CantonError(canton_error) => canton_error.set_ctx(ctx),
            AdminApiError::ValueError(_) => {}
        }
    }
}
