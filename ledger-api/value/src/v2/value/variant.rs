use canton_types::{Name, PackageId};
use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;
use protobuf_utils::RequiredProtoField as _;

use crate::v2::{
    Identifier,
    errors::{IntoValueError as _, ValueError, ValueKindError},
    traits,
    value::Value,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant {
    pub variant_id: Option<Identifier<PackageId>>,
    pub constructor: Name,
    pub value: Value,
}

// Protobuf conversions

impl From<Variant> for proto::Variant {
    fn from(value: Variant) -> Self {
        Self {
            variant_id: value.variant_id.map(Into::into),
            constructor: value.constructor.into(),
            value: Some(Box::new(value.value.into())),
        }
    }
}

impl TryFrom<proto::Variant> for Variant {
    type Error = ValueError;

    fn try_from(value: proto::Variant) -> Result<Self, Self::Error> {
        Ok(Self {
            variant_id: value.variant_id.map(TryInto::try_into).transpose()?,
            constructor: Name::new(value.constructor).no_msg()?,
            value: Value::try_from(
                *value
                    .value
                    .required_of::<proto::Variant>("value")
                    .no_msg()?,
            )?,
        })
    }
}

// Ledger API value traits

impl traits::IntoValue for Variant {
    fn into_value(self) -> Value {
        Value::Variant(Box::new(self))
    }
}

impl traits::TryFromValue for Variant {
    type Error = ValueKindError;

    fn try_from_value(value: Value) -> Result<Self, Self::Error> {
        value.into_variant()
    }
}

impl traits::Value for Variant {}
