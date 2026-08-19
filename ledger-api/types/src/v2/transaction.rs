use std::time::SystemTime;

use canton_types::LedgerString;
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

/// Ledger transaction
#[derive(Clone, Debug)]
pub struct Transaction<E> {
    pub update_id: LedgerString,
    pub command_id: Option<LedgerString>,
    pub workflow_id: Option<LedgerString>,
    pub effective_at: SystemTime,
    pub events: Vec<E>,
    pub offset: i64,
    // pub synchronizer_id: SynchronizerId,
    // pub trace_context: Option<TraceContext>,
    pub record_time: SystemTime,
    // pub external_transaction_hash: Option<TxHash>,
    pub paid_traffic_cost: Option<i64>,
    // TODO: implement missing fields
}

impl<E> TryFrom<proto::Transaction> for Transaction<E>
where
    E: TryFrom<proto::Event, Error: Into<ValueError>>,
{
    type Error = ValueError;

    fn try_from(tx: proto::Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            update_id: LedgerString::new(tx.update_id)
                .validated_of::<proto::Transaction>("update_id")
                .no_msg()?,
            command_id: (!tx.command_id.is_empty())
                .then(|| LedgerString::new(tx.command_id))
                .transpose()
                .validated_of::<proto::Transaction>("command_id")
                .no_msg()?,
            workflow_id: (!tx.workflow_id.is_empty())
                .then(|| LedgerString::new(tx.workflow_id))
                .transpose()
                .validated_of::<proto::Transaction>("workflow_id")
                .no_msg()?,
            effective_at: tx
                .effective_at
                .required_of::<proto::Transaction>("effective_at")
                .no_msg()?
                .try_into()
                .unwrap(), // FIXME: change unwrap to error
            events: tx
                .events
                .into_iter()
                .enumerate()
                .map(|(idx, event)| {
                    event
                        .try_into()
                        .map_err(Into::into)
                        .with_msg_owned(format!("failed to convert event[{idx}]"))
                })
                .collect::<Result<_, _>>()?,
            offset: tx.offset,
            record_time: tx
                .record_time
                .required_of::<proto::Transaction>("record_time")
                .no_msg()?
                .try_into()
                .unwrap(), // FIXME: change unwrap to error
            paid_traffic_cost: tx.paid_traffic_cost,
        })
    }
}
