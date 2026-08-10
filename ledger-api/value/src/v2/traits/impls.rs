//! This module provides some implementations of these traits for built-in, std and Canton common
//! types

// TODO: need to figure out what's the way to handle tuples properly

use std::collections::BTreeMap;

use canton_types::{ContractId, DottedName, Name, NonEmpty, Numeric, PackageId, PartyId};

use crate::v2::{
    Identifier, IntoRecord, IntoValue, Record, TryFromRecord, TryFromValue, Value,
    errors::{
        Aggregated2ValueError, AggregatedValueError, Tuple2Error, Tuple3Error,
        UnexpectedIdentifier, UnexpectedLabel, UnexpectedRecordSize, ValueKindError,
    },
    value,
};

/// Package ID of `daml-prim-DA-Types`
/// FIXME: need to find a proper way to set this instead of just hard coding
const DAML_PRIM_DA_TYPES_PKG_ID: PackageId =
    PackageId::new_unchecked("5aee9b21b8e9a4c4975b5f4c4198e6e6e8469df49e2010820e792f393db870f4");

/// Get identifier of tupleN type
fn tuple_id<const N: usize>() -> Identifier<PackageId> {
    // Assumptions from Daml
    debug_assert!(N > 0);
    debug_assert!(N <= 20);

    let module_name = DottedName::from_segments(NonEmpty::new(
        vec![Name::new_unchecked("DA".to_string())],
        Name::new_unchecked("Types".to_string()),
    ));

    let entity_name = DottedName::single(Name::new_unchecked(format!("Tuple{N}")));

    Identifier {
        package_id: DAML_PRIM_DA_TYPES_PKG_ID,
        module_name,
        entity_name,
    }
}

// unit type

impl IntoValue for () {
    fn into_value(self) -> value::Value {
        value::Value::Unit
    }
}

impl TryFromValue for () {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_unit()
    }
}

impl Value for () {}

// bool

impl IntoValue for bool {
    fn into_value(self) -> value::Value {
        value::Value::Bool(self)
    }
}

impl TryFromValue for bool {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_bool()
    }
}

impl Value for bool {}

// i64

impl IntoValue for i64 {
    fn into_value(self) -> value::Value {
        value::Value::Int64(self)
    }
}

impl TryFromValue for i64 {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_i64()
    }
}

impl Value for i64 {}

// String

impl IntoValue for String {
    fn into_value(self) -> value::Value {
        value::Value::Text(self)
    }
}

impl TryFromValue for String {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_text()
    }
}

impl Value for String {}

// tuples

// (T1, T2)

impl<T1: IntoValue, T2: IntoValue> IntoRecord for (T1, T2) {
    fn into_record(self) -> value::Record {
        value::Record {
            record_id: Some(tuple_id::<2>()),
            fields: vec![
                value::RecordField {
                    label: Some(Name::new_unchecked("_1".to_string())),
                    value: self.0.into_value(),
                },
                value::RecordField {
                    label: Some(Name::new_unchecked("_2".to_string())),
                    value: self.1.into_value(),
                },
            ],
        }
    }
}

impl<T1: TryFromValue, T2: TryFromValue> TryFromRecord for (T1, T2) {
    type Error = Tuple2Error<T1::Error, T2::Error>;

    fn try_from_record(record: value::Record) -> Result<Self, Self::Error> {
        let identifier: Option<Identifier<PackageId>> = record.record_id;
        if let Some(identifier) = identifier {
            let expected_identifier = tuple_id::<2>();
            if identifier != expected_identifier {
                return Err(UnexpectedIdentifier::new(
                    expected_identifier.to_string(),
                    identifier.to_string(),
                )
                .into());
            }
        }

        let fields = <[value::RecordField; 2]>::try_from(record.fields)
            .map_err(|orig| UnexpectedRecordSize::new(2, orig.len()))?;

        for (idx, field) in fields.iter().enumerate() {
            if let Some(label) = &field.label {
                let expected = format!("_{idx}");
                if label != expected {
                    return Err(UnexpectedLabel::new(expected, label.as_str().to_owned()).into());
                }
            }
        }

        let [field0, field1] = fields;
        let value0 = T1::try_from_value(field0.value).map_err(Tuple2Error::T1Error)?;
        let value1 = T2::try_from_value(field1.value).map_err(Tuple2Error::T2Error)?;

        Ok((value0, value1))
    }
}

impl<T1: Value, T2: Value> Record for (T1, T2) {}

// (T1, T2, T3)

impl<T1: IntoValue, T2: IntoValue, T3: IntoValue> IntoRecord for (T1, T2, T3) {
    fn into_record(self) -> value::Record {
        value::Record {
            record_id: Some(tuple_id::<3>()),
            fields: vec![
                value::RecordField {
                    label: Some(Name::new_unchecked("_1".to_string())),
                    value: self.0.into_value(),
                },
                value::RecordField {
                    label: Some(Name::new_unchecked("_2".to_string())),
                    value: self.1.into_value(),
                },
                value::RecordField {
                    label: Some(Name::new_unchecked("_3".to_string())),
                    value: self.2.into_value(),
                },
            ],
        }
    }
}

impl<T1: TryFromValue, T2: TryFromValue, T3: TryFromValue> TryFromRecord for (T1, T2, T3) {
    type Error = Tuple3Error<T1::Error, T2::Error, T3::Error>;

    fn try_from_record(record: value::Record) -> Result<Self, Self::Error> {
        let identifier: Option<Identifier<PackageId>> = record.record_id;
        if let Some(identifier) = identifier {
            let expected_identifier = tuple_id::<3>();
            if identifier != expected_identifier {
                return Err(Tuple3Error::TupleFromRecordError(
                    UnexpectedIdentifier::new(
                        expected_identifier.to_string(),
                        identifier.to_string(),
                    )
                    .into(),
                ));
            }
        }

        let fields = <[value::RecordField; 3]>::try_from(record.fields)
            .map_err(|orig| UnexpectedRecordSize::new(3, orig.len()))?;

        for (idx, field) in fields.iter().enumerate() {
            if let Some(label) = &field.label {
                let expected = format!("_{idx}");
                if label != expected {
                    return Err(UnexpectedLabel::new(expected, label.as_str().to_owned()).into());
                }
            }
        }

        let [field0, field1, field2] = fields;
        let value0 = T1::try_from_value(field0.value).map_err(Tuple3Error::T1Error)?;
        let value1 = T2::try_from_value(field1.value).map_err(Tuple3Error::T2Error)?;
        let value2 = T3::try_from_value(field2.value).map_err(Tuple3Error::T3Error)?;

        Ok((value0, value1, value2))
    }
}

impl<T1: Value, T2: Value, T3: Value> Record for (T1, T2, T3) {}

// Option<T>

impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> value::Value {
        value::Value::Optional(self.map(T::into_value).map(Box::new))
    }
}

impl<T: TryFromValue> TryFromValue for Option<T> {
    type Error = AggregatedValueError<T::Error>;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value
            .into_optional()?
            .map(|y| T::try_from_value(*y))
            .transpose()
            .map_err(AggregatedValueError::Other)
    }
}

impl<T: Value> Value for Option<T> {}

// Vec<T>

impl<T: IntoValue> IntoValue for Vec<T> {
    fn into_value(self) -> value::Value {
        value::Value::List(self.into_iter().map(T::into_value).collect())
    }
}

impl<T: TryFromValue> TryFromValue for Vec<T> {
    type Error = AggregatedValueError<T::Error>;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value
            .into_list()?
            .into_iter()
            .map(T::try_from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(AggregatedValueError::Other)
    }
}

impl<T: Value> Value for Vec<T> {}

// BTreeMap<K, V>

impl<K: IntoValue, V: IntoValue> IntoValue for BTreeMap<K, V> {
    fn into_value(self) -> value::Value {
        value::Value::GenMap(
            self.into_iter()
                .map(|(k, v)| (k.into_value(), v.into_value()))
                .collect::<BTreeMap<_, _>>(),
        )
    }
}

impl<K: Ord + TryFromValue, V: TryFromValue> TryFromValue for BTreeMap<K, V> {
    type Error = Aggregated2ValueError<K::Error, V::Error>;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value
            .into_gen_map()?
            .into_iter()
            .map(|(k, v)| {
                let k = K::try_from_value(k).map_err(Self::Error::Other1);
                let v = V::try_from_value(v).map_err(Self::Error::Other2);
                k.and_then(|k| v.map(|v| (k, v)))
            })
            .collect()
    }
}

impl<K: Ord + Value, V: Value> Value for BTreeMap<K, V> {}

// Daml primitives

// Party ID

impl IntoValue for PartyId {
    fn into_value(self) -> value::Value {
        value::Value::Party(self)
    }
}

impl TryFromValue for PartyId {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_party()
    }
}

impl Value for PartyId {}

// Contract ID

impl<T> IntoValue for ContractId<T> {
    fn into_value(self) -> value::Value {
        value::Value::ContractId(self.into_any())
    }
}

impl<T> TryFromValue for ContractId<T> {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_contract_id().map(ContractId::from_any)
    }
}

impl<T> Value for ContractId<T> {}

// Numeric

impl IntoValue for Numeric {
    fn into_value(self) -> value::Value {
        value::Value::Numeric(self)
    }
}

impl TryFromValue for Numeric {
    type Error = ValueKindError;

    fn try_from_value(value: value::Value) -> Result<Self, Self::Error> {
        value.into_numeric()
    }
}

impl Value for Numeric {}
