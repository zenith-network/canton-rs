use canton_proto::prost::UnknownEnumValue;
use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField, UnspecifiedEnumValue};

/// Error which is returned on failed conversion from protobuf types.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ValueError {
    inner: ValueErrorInner,
}

#[derive(Debug, thiserror::Error)]
enum ValueErrorInner {
    #[error(transparent)]
    MissingProtoField(#[from] MissingProtoField),

    #[error(transparent)]
    InvalidProtoFieldValue(#[from] InvalidProtoFieldValue),

    #[error(transparent)]
    UnspecifiedEnumValue(#[from] UnspecifiedEnumValue),

    #[error(transparent)]
    UnknownEnumValue(#[from] UnknownEnumValue),
}

impl From<MissingProtoField> for ValueError {
    fn from(value: MissingProtoField) -> Self {
        Self {
            inner: ValueErrorInner::MissingProtoField(value),
        }
    }
}

impl From<InvalidProtoFieldValue> for ValueError {
    fn from(value: InvalidProtoFieldValue) -> Self {
        Self {
            inner: ValueErrorInner::InvalidProtoFieldValue(value),
        }
    }
}

impl From<UnspecifiedEnumValue> for ValueError {
    fn from(value: UnspecifiedEnumValue) -> Self {
        Self {
            inner: ValueErrorInner::UnspecifiedEnumValue(value),
        }
    }
}

impl From<UnknownEnumValue> for ValueError {
    fn from(value: UnknownEnumValue) -> Self {
        Self {
            inner: ValueErrorInner::UnknownEnumValue(value),
        }
    }
}
