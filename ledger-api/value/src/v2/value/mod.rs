//! Concrete values of Ledger API

use std::{collections::BTreeMap, convert::Infallible};

use canton_types::{AnyTemplate, ContractId, Numeric, PartyId};
use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;

use super::{
    errors::{IntoValueError as _, ValueError, ValueKindError},
    traits::{self, IntoValue, TryFromValue},
};

mod enum_;
mod record;
mod text_map;
mod value_kind;
mod variant;

pub use enum_::Enum;
pub use record::{Record, RecordField};
pub use text_map::TextMap;
pub use value_kind::ValueKind;
pub use variant::Variant;

/// Aggregated type of all possible Ledger API values
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Value {
    Unit,
    Bool(bool),
    Int64(i64),
    Date(i32),      // TODO: put appropriate type here
    Timestamp(i64), // TODO: put appropriate type here
    Numeric(Numeric),
    Party(PartyId),
    Text(String),
    ContractId(ContractId<AnyTemplate>),
    Optional(Option<Box<Self>>),
    List(Vec<Self>),
    TextMap(TextMap<Self>),
    GenMap(BTreeMap<Self, Self>),
    Record(Record),
    Variant(Box<Variant>),
    Enum(Enum),
}

impl Value {
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Unit => ValueKind::Unit,
            Value::Bool(_) => ValueKind::Bool,
            Value::Int64(_) => ValueKind::Int64,
            Value::Date(_) => ValueKind::Date,
            Value::Timestamp(_) => ValueKind::Timestamp,
            Value::Numeric(_) => ValueKind::Numeric,
            Value::Party(_) => ValueKind::Party,
            Value::Text(_) => ValueKind::Text,
            Value::ContractId(_) => ValueKind::ContractId,
            Value::Optional(_) => ValueKind::Optional,
            Value::List(_) => ValueKind::List,
            Value::TextMap(_) => ValueKind::TextMap,
            Value::GenMap(_) => ValueKind::GenMap,
            Value::Record(_) => ValueKind::Record,
            Value::Variant { .. } => ValueKind::Variant,
            Value::Enum { .. } => ValueKind::Enum,
        }
    }

    pub fn into_unit(&self) -> Result<(), ValueKindError> {
        if let Value::Unit = self {
            Ok(())
        } else {
            Err(ValueKindError {
                expected: ValueKind::Unit,
                got: self.kind(),
            })
        }
    }

    pub fn into_bool(self) -> Result<bool, ValueKindError> {
        if let Value::Bool(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Bool,
                got: self.kind(),
            })
        }
    }

    pub fn into_i64(self) -> Result<i64, ValueKindError> {
        if let Value::Int64(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Int64,
                got: self.kind(),
            })
        }
    }

    pub fn into_numeric(self) -> Result<Numeric, ValueKindError> {
        if let Value::Numeric(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Numeric,
                got: self.kind(),
            })
        }
    }

    pub fn into_party(self) -> Result<PartyId, ValueKindError> {
        if let Value::Party(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Party,
                got: self.kind(),
            })
        }
    }

    pub fn into_text(self) -> Result<String, ValueKindError> {
        if let Value::Text(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Text,
                got: self.kind(),
            })
        }
    }

    pub fn into_contract_id(self) -> Result<ContractId, ValueKindError> {
        if let Value::ContractId(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::ContractId,
                got: self.kind(),
            })
        }
    }

    pub fn into_optional(self) -> Result<Option<Box<Self>>, ValueKindError> {
        if let Value::Optional(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Optional,
                got: self.kind(),
            })
        }
    }

    pub fn into_list(self) -> Result<Vec<Self>, ValueKindError> {
        if let Value::List(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::List,
                got: self.kind(),
            })
        }
    }

    pub fn into_text_map(self) -> Result<TextMap<Self>, ValueKindError> {
        if let Value::TextMap(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::TextMap,
                got: self.kind(),
            })
        }
    }

    pub fn into_gen_map(self) -> Result<BTreeMap<Self, Self>, ValueKindError> {
        if let Value::GenMap(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::GenMap,
                got: self.kind(),
            })
        }
    }

    pub fn into_record(self) -> Result<Record, ValueKindError> {
        if let Value::Record(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Record,
                got: self.kind(),
            })
        }
    }

    pub fn into_variant(self) -> Result<Variant, ValueKindError> {
        if let Value::Variant(value) = self {
            Ok(*value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Variant,
                got: self.kind(),
            })
        }
    }

    pub fn into_enum(self) -> Result<Enum, ValueKindError> {
        if let Value::Enum(value) = self {
            Ok(value)
        } else {
            Err(ValueKindError {
                expected: ValueKind::Enum,
                got: self.kind(),
            })
        }
    }
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl TryFromValue for Value {
    type Error = Infallible;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        Ok(value)
    }
}

impl traits::Value for Value {}

impl From<Value> for proto::Value {
    fn from(value: Value) -> Self {
        use proto::value::Sum;

        match value {
            Value::Unit => Self {
                sum: Some(Sum::Unit(())),
            },
            Value::Bool(v) => Self {
                sum: Some(Sum::Bool(v)),
            },
            Value::Int64(v) => Self {
                sum: Some(Sum::Int64(v)),
            },
            Value::Date(v) => Self {
                sum: Some(Sum::Date(v)),
            },
            Value::Timestamp(v) => Self {
                sum: Some(Sum::Timestamp(v)),
            },
            Value::Numeric(v) => Self {
                sum: Some(Sum::Numeric(v.to_string())),
            },
            Value::Party(v) => Self {
                sum: Some(Sum::Party(v.into())),
            },
            Value::Text(v) => Self {
                sum: Some(Sum::Text(v)),
            },
            Value::ContractId(v) => Self {
                sum: Some(Sum::ContractId(v.into())),
            },
            Value::Optional(v) => Self {
                sum: Some(Sum::Optional(Box::new(proto::Optional {
                    value: v.map(|x| Box::new((*x).into())),
                }))),
            },
            Value::List(v) => Self {
                sum: Some(Sum::List(proto::List {
                    elements: v.into_iter().map(Into::into).collect(),
                })),
            },
            Value::TextMap(v) => Self {
                sum: Some(Sum::TextMap(v.into())),
            },
            Value::GenMap(v) => Self {
                sum: Some(Sum::GenMap(proto::GenMap {
                    entries: v
                        .into_iter()
                        .map(|(key, value)| proto::gen_map::Entry {
                            key: Some(key.into()),
                            value: Some(value.into()),
                        })
                        .collect(),
                })),
            },
            Value::Record(v) => Self {
                sum: Some(Sum::Record(v.into())),
            },
            Value::Variant(v) => Self {
                sum: Some(Sum::Variant(Box::new((*v).into()))),
            },
            Value::Enum(v) => Self {
                sum: Some(Sum::Enum(v.into())),
            },
        }
    }
}

impl TryFrom<proto::Value> for Value {
    type Error = ValueError;

    fn try_from(value: proto::Value) -> Result<Self, Self::Error> {
        use proto::value::Sum;

        Ok(match value.sum.ok_or(ValueError::no_value_found())? {
            Sum::Unit(_) => Self::Unit,
            Sum::Bool(v) => Self::Bool(v),
            Sum::Int64(v) => Self::Int64(v),
            Sum::Date(v) => Self::Date(v),
            Sum::Timestamp(v) => Self::Timestamp(v),
            Sum::Numeric(v) => Self::Numeric(Numeric::parse(v).no_msg()?),
            Sum::Party(v) => Self::Party(PartyId::new(v).no_msg()?),
            Sum::Text(v) => Self::Text(v),
            Sum::ContractId(v) => Self::ContractId(ContractId::new(v).no_msg()?),
            Sum::Optional(v) => Self::Optional(
                v.value
                    .map(|v| Self::try_from(*v))
                    .transpose()?
                    .map(Box::new),
            ),
            Sum::List(v) => Self::List(
                v.elements
                    .into_iter()
                    .enumerate()
                    .map(|(idx, e)| {
                        e.try_into()
                            .with_msg_owned(format!("failed to convert element[{idx}]"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            Sum::TextMap(v) => Self::TextMap(v.try_into()?),
            Sum::GenMap(v) => {
                use proto::gen_map::Entry;

                let mut map = BTreeMap::new();
                for (idx, Entry { key, value }) in v.entries.into_iter().enumerate() {
                    // Skipping empty values
                    if let (Some(key), Some(value)) = (key, value) {
                        map.insert(
                            key.try_into()
                                .with_msg_owned(format!("failed to convert key of entry[{idx}]"))?,
                            value.try_into().with_msg_owned(format!(
                                "failed to convert value of entry[{idx}]"
                            ))?,
                        );
                    }
                }
                Self::GenMap(map)
            }
            Sum::Record(record) => Self::Record(record.try_into()?),
            Sum::Variant(variant) => Self::Variant(Box::new((*variant).try_into()?)),
            Sum::Enum(enum_) => Self::Enum(enum_.try_into()?),
        })
    }
}
