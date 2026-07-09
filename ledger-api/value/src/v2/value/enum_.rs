use canton_types::{Name, PackageId};
use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;

use crate::v2::{
    Identifier,
    errors::{IntoValueError as _, ValueError, ValueKindError},
    traits,
    value::Value,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Enum {
    pub enum_id: Option<Identifier<PackageId>>,
    pub constructor: Name,
}

// Protobuf conversions

impl From<Enum> for proto::Enum {
    fn from(value: Enum) -> Self {
        Self {
            enum_id: value.enum_id.map(Into::into),
            constructor: value.constructor.into(),
        }
    }
}

impl TryFrom<proto::Enum> for Enum {
    type Error = ValueError;

    fn try_from(value: proto::Enum) -> Result<Self, Self::Error> {
        Ok(Self {
            enum_id: value
                .enum_id
                .map(TryInto::try_into)
                .transpose()
                .with_msg("failed to convert Enum.enum_id")?,
            constructor: Name::new(value.constructor)
                .with_msg("failed to convert Enum.constructor")?,
        })
    }
}

// Ledger API value conversions

impl traits::IntoValue for Enum {
    fn into_value(self) -> Value {
        Value::Enum(self)
    }
}

impl traits::TryFromValue for Enum {
    type Error = ValueKindError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        value.into_enum()
    }
}

impl traits::Value for Enum {}
