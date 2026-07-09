use std::{
    collections::BTreeMap,
    ops::{Deref, DerefMut},
};

use ledger_api_value_proto::com::daml::ledger::api::v2 as proto;
use protobuf_utils::RequiredProtoField as _;

use crate::v2::{
    errors::{AggregatedValueError, IntoValueError as _, ValueError},
    traits,
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TextMap<T>(pub BTreeMap<String, T>);

impl<T> Deref for TextMap<T> {
    type Target = BTreeMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for TextMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Protobuf conversions

impl<T> From<TextMap<T>> for proto::TextMap
where
    T: Into<proto::Value>,
{
    fn from(value: TextMap<T>) -> Self {
        use proto::text_map::Entry;

        proto::TextMap {
            entries: value
                .0
                .into_iter()
                .map(|(key, value)| Entry {
                    key,
                    value: Some(value.into()),
                })
                .collect(),
        }
    }
}

impl<T> TryFrom<proto::TextMap> for TextMap<T>
where
    T: TryFrom<proto::Value, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: proto::TextMap) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .entries
                .into_iter()
                .enumerate()
                .map(|(idx, entry)| {
                    entry
                        .value
                        .required_of::<proto::text_map::Entry>("value")
                        .no_msg()?
                        .try_into()
                        .with_msg_owned(format!("failed to convert entry[{idx}]"))
                        .map(|t| (entry.key, t))
                })
                .collect::<Result<_, _>>()?,
        ))
    }
}

// Ledger API value conversions

impl<T> traits::IntoValue for TextMap<T>
where
    T: traits::IntoValue,
{
    fn into_value(self) -> super::Value {
        super::Value::TextMap(TextMap(
            self.0
                .into_iter()
                .map(|(k, v)| (k, v.into_value()))
                .collect(),
        ))
    }
}

impl<T> traits::TryFromValue for TextMap<T>
where
    T: traits::TryFromValue,
{
    type Error = AggregatedValueError<T::Error>;

    fn try_from_value(value: super::Value) -> Result<Self, Self::Error> {
        value
            .into_text_map()?
            .0
            .into_iter()
            .map(|(k, v)| T::try_from_value(v).map(|t| (k, t)))
            .collect::<Result<_, _>>()
            .map_err(AggregatedValueError::Other)
            .map(Self)
    }
}

impl<T: traits::Value> traits::Value for TextMap<T> {}
