use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::ValueError;

use crate::v2::Empty;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Update<T = Empty, R = Empty, C = Empty, P = Empty> {
    Transaction(T),
    Reassignment(R),
    OffsetCheckpoint(C),
    TopologyTransaction(P),
}

impl<T, R, C, P> TryFrom<proto::get_updates_response::Update> for Update<T, R, C, P>
where
    T: TryFrom<proto::Transaction, Error = ValueError>,
    R: TryFrom<proto::Reassignment, Error = ValueError>,
    C: TryFrom<proto::OffsetCheckpoint, Error = ValueError>,
    P: TryFrom<proto::TopologyTransaction, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: proto::get_updates_response::Update) -> Result<Self, Self::Error> {
        use proto::get_updates_response::Update::*;

        match value {
            Transaction(tx) => tx.try_into().map(Update::Transaction),
            Reassignment(reas) => reas.try_into().map(Update::Reassignment),
            OffsetCheckpoint(oc) => oc.try_into().map(Update::OffsetCheckpoint),
            TopologyTransaction(tt) => tt.try_into().map(Update::TopologyTransaction),
        }
    }
}

impl<T, R, P> TryFrom<proto::get_update_response::Update> for Update<T, R, Empty, P>
where
    T: TryFrom<proto::Transaction, Error = ValueError>,
    R: TryFrom<proto::Reassignment, Error = ValueError>,
    P: TryFrom<proto::TopologyTransaction, Error = ValueError>,
{
    type Error = ValueError;

    fn try_from(value: proto::get_update_response::Update) -> Result<Self, Self::Error> {
        use proto::get_update_response::Update::*;
        match value {
            Transaction(tx) => tx.try_into().map(Update::Transaction),
            Reassignment(reas) => reas.try_into().map(Update::Reassignment),
            TopologyTransaction(tt) => tt.try_into().map(Update::TopologyTransaction),
        }
    }
}
