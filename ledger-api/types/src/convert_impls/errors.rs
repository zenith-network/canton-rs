use prost_types::TimestampError;

// Re-export for convenience
pub use ledger_api_value_proto::errors::ValueError as LedgerApiValueError;

use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField};

#[derive(Debug, thiserror::Error)]
pub enum ValueError {
    #[error(transparent)]
    LedgerApiValueError(#[from] LedgerApiValueError),
    #[error(transparent)]
    TimestampError(#[from] TimestampError),
    #[error("received unexpected event: expected '{expected}', got '{got}'")]
    UnexpectedEvent {
        expected: &'static str,
        got: &'static str,
    },
    #[error(transparent)]
    InvalidProtoFieldValue(#[from] InvalidProtoFieldValue),
    #[error(transparent)]
    MissingProtoField(#[from] MissingProtoField),
}

// TODO: maybe we want a type with context message here as well?
