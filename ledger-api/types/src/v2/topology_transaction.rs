use std::time::SystemTime;

use canton_types::{LedgerString, SynchronizerId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v2::{HasOffset, TopologyEvent};

/// Topology transaction representation in Ledger API.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TopologyTransaction {
    /// Assigned by the server. Useful for correlating logs.
    pub update_id: LedgerString,

    /// The absolute offset.
    ///
    /// It is a valid absolute offset (positive integer).
    pub offset: i64,

    /// A valid synchronizer ID.
    ///
    /// Identifies the synchronizer that synchronized the topology transaction.
    pub synchronizer_id: SynchronizerId,

    /// The time at which the changes in the topology transaction become effective.
    ///
    /// There is a small delay between a topology transaction being sequenced and the changes it
    /// contains becoming effective. Topology transactions appear in order relative to a
    /// synchronizer based on their effective time rather than their sequencing time.
    pub record_time: SystemTime,

    /// A list of topology events.
    pub events: Vec<TopologyEvent>,
    // TODO: implement missing fields:
    //       - trace_context
}

impl HasOffset for TopologyTransaction {
    fn offset(&self) -> i64 {
        self.offset
    }
}

impl TryFrom<proto::TopologyTransaction> for TopologyTransaction {
    type Error = ValueError;

    fn try_from(tx: proto::TopologyTransaction) -> Result<Self, Self::Error> {
        type Proto = proto::TopologyTransaction;

        Ok(Self {
            update_id: LedgerString::new(tx.update_id)
                .validated_of::<Proto>("update_id")
                .no_msg()?,
            offset: tx.offset,
            synchronizer_id: SynchronizerId::new(tx.synchronizer_id)
                .validated_of::<Proto>("synchronizer_id")
                .no_msg()?,
            record_time: tx
                .record_time
                .required_of::<Proto>("record_time")
                .no_msg()?
                .try_into()
                .validated_of::<Proto>("record_time")
                .no_msg()?,
            events: tx
                .events
                .into_iter()
                .enumerate()
                .map(|(idx, event)| {
                    event
                        .try_into()
                        .with_msg_owned(format!("failed to convert events[{idx}]"))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}
