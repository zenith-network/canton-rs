use ledger_api_proto::com::daml::ledger::api::v2 as proto;

use crate::v2::{EventFormat, TopologyFormat, TransactionFormat, TransactionShape, TxShape};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateFormat<S: TxShape = TransactionShape> {
    pub include_transactions: Option<TransactionFormat<S>>,
    pub include_reassignments: Option<EventFormat>,
    pub include_topology_events: Option<TopologyFormat>,
}

impl<S: TxShape> From<UpdateFormat<S>> for proto::UpdateFormat {
    fn from(value: UpdateFormat<S>) -> Self {
        Self {
            include_transactions: value.include_transactions.map(Into::into),
            include_reassignments: value.include_reassignments.map(Into::into),
            include_topology_events: value.include_topology_events.map(Into::into),
        }
    }
}
