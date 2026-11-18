//! Error types

use core::{error::Error, fmt};

use canton_proto::prost::UnknownEnumValue;
use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField, UnspecifiedEnumValue};

use crate::TopologyMappingCode;

/// Error which is returned on failed conversion from protobuf message type.
#[derive(thiserror::Error)]
#[error(transparent)]
pub struct ProtoDecodeError {
    repr: DecodeErrorRepr,
}

// Avoid "inner" field name appearing in the output
impl fmt::Debug for ProtoDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProtoDecodeError").field(&self.repr).finish()
    }
}

impl ProtoDecodeError {
    /// Create decode error from other error
    pub fn other(error: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self {
            repr: DecodeErrorRepr::Other(error.into()),
        }
    }
}

#[derive(thiserror::Error)]
enum DecodeErrorRepr {
    #[error(transparent)]
    MissingProtoField(#[from] MissingProtoField),

    #[error(transparent)]
    InvalidProtoFieldValue(#[from] InvalidProtoFieldValue),

    #[error(transparent)]
    UnspecifiedEnumValue(#[from] UnspecifiedEnumValue),

    #[error(transparent)]
    UnknownEnumValue(#[from] UnknownEnumValue),

    #[error(transparent)]
    Other(Box<dyn Error + Send + Sync>),
}

// Transparent output, avoid variant names
impl fmt::Debug for DecodeErrorRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProtoField(err) => err.fmt(f),
            Self::InvalidProtoFieldValue(err) => err.fmt(f),
            Self::UnspecifiedEnumValue(err) => err.fmt(f),
            Self::UnknownEnumValue(err) => err.fmt(f),
            Self::Other(err) => err.fmt(f),
        }
    }
}

impl From<MissingProtoField> for ProtoDecodeError {
    fn from(error: MissingProtoField) -> Self {
        Self { repr: error.into() }
    }
}

impl From<InvalidProtoFieldValue> for ProtoDecodeError {
    fn from(error: InvalidProtoFieldValue) -> Self {
        Self { repr: error.into() }
    }
}

impl From<UnspecifiedEnumValue> for ProtoDecodeError {
    fn from(error: UnspecifiedEnumValue) -> Self {
        Self { repr: error.into() }
    }
}

impl From<UnknownEnumValue> for ProtoDecodeError {
    fn from(error: UnknownEnumValue) -> Self {
        Self { repr: error.into() }
    }
}

/// Error which is returned on failed conversion into protobuf message type.
#[derive(thiserror::Error)]
#[error(transparent)]
pub struct ProtoEncodeError {
    repr: EncodeErrorRepr,
}

// Avoid "inner" field name appearing in the output
impl fmt::Debug for ProtoEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProtoDecodeError").field(&self.repr).finish()
    }
}

impl ProtoEncodeError {
    /// Create encode error from other error
    pub fn other(error: impl Into<Box<dyn Error + Send + Sync>>) -> Self {
        Self {
            repr: EncodeErrorRepr::Other(error.into()),
        }
    }
}

impl From<IncompatibleMessageSchema> for ProtoEncodeError {
    fn from(error: IncompatibleMessageSchema) -> Self {
        Self { repr: error.into() }
    }
}

#[derive(thiserror::Error)]
enum EncodeErrorRepr {
    #[error(transparent)]
    IncompatibleMessageSchema(#[from] IncompatibleMessageSchema),

    #[error(transparent)]
    Other(Box<dyn Error + Send + Sync>),
}

// Transparent output, avoid variant names
impl fmt::Debug for EncodeErrorRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IncompatibleMessageSchema(err) => err.fmt(f),
            Self::Other(err) => err.fmt(f),
        }
    }
}

/// This error is returned, when a conversion into a certain version of the message schema failed
#[derive(Debug, thiserror::Error)]
#[error("incompatible message schema: {reason}")]
pub struct IncompatibleMessageSchema {
    reason: String,
}

impl IncompatibleMessageSchema {
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unexpected topology mapping variant: expected '{expected}', got '{got}'")]
pub struct UnexpectedTopologyMapping {
    expected: TopologyMappingCode,
    got: TopologyMappingCode,
}

impl UnexpectedTopologyMapping {
    pub fn new(expected: TopologyMappingCode, got: TopologyMappingCode) -> Self {
        Self { expected, got }
    }
}
