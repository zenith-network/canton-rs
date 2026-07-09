use std::convert::Infallible;

use canton_types::{Name, PackageId};
use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v2::{
    Identifier,
    errors::{IntoValueError as _, ValueError},
    traits,
    value::Value,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordField {
    pub label: Option<Name>,
    pub value: Value,
}

// Protobuf conversions

impl From<RecordField> for proto::RecordField {
    fn from(field: RecordField) -> Self {
        Self {
            label: field.label.map(Into::into).unwrap_or_default(),
            value: Some(field.value.into()),
        }
    }
}

impl TryFrom<proto::RecordField> for RecordField {
    type Error = ValueError;

    fn try_from(field: proto::RecordField) -> Result<Self, Self::Error> {
        Ok(Self {
            label: if field.label.is_empty() {
                None
            } else {
                Some(Name::new(field.label).no_msg()?)
            },
            value: field
                .value
                .required_of::<proto::RecordField>("value")
                .no_msg()?
                .try_into()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Record {
    pub record_id: Option<Identifier<PackageId>>,
    pub fields: Vec<RecordField>,
}

// Protobuf conversions

impl From<Record> for proto::Record {
    fn from(record: Record) -> Self {
        Self {
            record_id: record.record_id.map(Into::into),
            fields: record.fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<proto::Record> for Record {
    type Error = ValueError;

    fn try_from(record: proto::Record) -> Result<Self, Self::Error> {
        Ok(Self {
            record_id: record
                .record_id
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::Record>("record_id")
                .no_msg()?,
            fields: record
                .fields
                .into_iter()
                .enumerate()
                .map(|(idx, field)| {
                    field
                        .try_into()
                        .validated_of::<proto::Record>("fields")
                        .with_msg_owned(format!("failed to convert field[{idx}]"))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

// Ledger API value conversions

impl traits::IntoRecord for Record {
    fn into_record(self) -> Record {
        self
    }
}

impl traits::TryFromRecord for Record {
    type Error = Infallible;

    fn try_from_record(record: Record) -> Result<Self, Self::Error> {
        Ok(record)
    }
}

impl traits::Record for Record {}
