use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField};

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
