use ledger_api_proto::{com::daml::ledger::api::v2 as proto, prost::Name as _};
use ledger_api_value::v2::errors::ValueError;

/// Type which has no value.
#[derive(Debug)]
pub enum Empty {}

macro_rules! impl_unexpected_variant {
    ($source:path) => {
        impl TryFrom<$source> for Empty {
            type Error = ValueError;

            fn try_from(_: $source) -> Result<Self, Self::Error> {
                Err(ValueError::raw_message_owned(format!(
                    "received a variant which wasn't expected for by the provided format: {}",
                    <$source>::full_name()
                )))
            }
        }
    };
}

impl_unexpected_variant!(proto::Transaction);
impl_unexpected_variant!(proto::Event);
impl_unexpected_variant!(proto::Reassignment);
impl_unexpected_variant!(proto::TopologyTransaction);
impl_unexpected_variant!(proto::CreatedEvent);
impl_unexpected_variant!(proto::ArchivedEvent);
impl_unexpected_variant!(proto::ExercisedEvent);
impl_unexpected_variant!(proto::OffsetCheckpoint);
