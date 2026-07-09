use std::time::SystemTime;

use canton_types::SynchronizerId;
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OffsetCheckpoint {
    pub offset: i64,
    pub synchronizer_times: Vec<SynchronizerTime>,
}

impl TryFrom<proto::OffsetCheckpoint> for OffsetCheckpoint {
    type Error = ValueError;

    fn try_from(value: proto::OffsetCheckpoint) -> Result<Self, Self::Error> {
        Ok(Self {
            offset: value.offset,
            synchronizer_times: value
                .synchronizer_times
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SynchronizerTime {
    pub synchronizer_id: SynchronizerId,
    pub record_time: SystemTime,
}

impl TryFrom<proto::SynchronizerTime> for SynchronizerTime {
    type Error = ValueError;

    fn try_from(value: proto::SynchronizerTime) -> Result<Self, Self::Error> {
        Ok(Self {
            synchronizer_id: SynchronizerId::new(value.synchronizer_id)
                .validated_of::<proto::SynchronizerTime>("synchronizer_id")
                .no_msg()?,
            record_time: value
                .record_time
                .required_of::<proto::SynchronizerTime>("record_time")
                .no_msg()?
                .try_into()
                .unwrap(), // FIXME: replace unwrap with error
        })
    }
}
