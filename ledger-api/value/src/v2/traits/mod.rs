use crate::v2::{errors::AggregatedValueError, value};

// Not using default conversion traits to avoid conflicts on auto-implementations

pub trait IntoValue {
    fn into_value(self) -> value::Value;
}

pub trait TryFromValue: Sized {
    type Error;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error>;
}

/// Value type
pub trait Value: IntoValue + TryFromValue {}

pub trait IntoRecord {
    fn into_record(self) -> value::Record;
}

pub trait TryFromRecord: Sized {
    type Error;

    fn try_from_record(record: value::Record) -> Result<Self, Self::Error>;
}

/// Record type
pub trait Record: IntoRecord + TryFromRecord {}

// If type can be converted to a record, it can be converted to a value
impl<T: IntoRecord> IntoValue for T {
    fn into_value(self) -> value::Value {
        value::Value::Record(self.into_record())
    }
}

// If type can be received as a record, it can be received as a value
impl<T: TryFromRecord> TryFromValue for T {
    type Error = AggregatedValueError<T::Error>;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        T::try_from_record(value.into_record()?).map_err(AggregatedValueError::Other)
    }
}

// If type is a record, it is a value
impl<T: Record> Value for T {}

mod impls;
