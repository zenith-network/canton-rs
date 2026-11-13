use protobuf_utils::{InvalidProtoFieldValue, MissingProtoField, UnspecifiedEnumValue};

/// Error on conversion from/into protobuf.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct ConvertError {
    inner: ErrorInner,
}

impl From<UnspecifiedEnumValue> for ConvertError {
    fn from(value: UnspecifiedEnumValue) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl From<InvalidProtoFieldValue> for ConvertError {
    fn from(value: InvalidProtoFieldValue) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl From<MissingProtoField> for ConvertError {
    fn from(value: MissingProtoField) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

impl From<TimestampOutOfRange> for ConvertError {
    fn from(value: TimestampOutOfRange) -> Self {
        Self {
            inner: value.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ErrorInner {
    #[error(transparent)]
    UnspecifiedEnumValue(#[from] UnspecifiedEnumValue),

    #[error(transparent)]
    InvalidProtoFieldValue(#[from] InvalidProtoFieldValue),

    #[error(transparent)]
    MissingProtoField(#[from] MissingProtoField),

    #[error(transparent)]
    TimestampOutOfRange(#[from] TimestampOutOfRange),
}

#[derive(Debug, thiserror::Error)]
#[error("Unix timestamp {micros} microseconds is out of range")]
pub struct TimestampOutOfRange {
    micros: i64,
}

impl TimestampOutOfRange {
    pub fn new(micros: i64) -> Self {
        Self { micros }
    }
}
