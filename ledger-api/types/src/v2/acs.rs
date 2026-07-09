use canton_types::SynchronizerId;

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v2::CreatedEvent;

#[derive(Clone, Debug)]
pub enum ContractEntry {
    ActiveContract(ActiveContract),
    IncompleteUnassigned(IncompleteUnassigned),
    IncompleteAssigned(IncompleteAssigned),
}

impl TryFrom<proto::get_active_contracts_response::ContractEntry> for ContractEntry {
    type Error = ValueError;

    fn try_from(
        value: proto::get_active_contracts_response::ContractEntry,
    ) -> Result<Self, Self::Error> {
        use proto::get_active_contracts_response as p;

        match value {
            p::ContractEntry::ActiveContract(active_contract) => {
                active_contract.try_into().map(Self::ActiveContract)
            }
            p::ContractEntry::IncompleteUnassigned(_) => {
                Ok(Self::IncompleteUnassigned(IncompleteUnassigned {}))
            }
            p::ContractEntry::IncompleteAssigned(_) => {
                Ok(Self::IncompleteAssigned(IncompleteAssigned {}))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ActiveContract {
    pub created_event: CreatedEvent,
    pub synchronizer_id: SynchronizerId,
    pub reassignment_counter: u64,
}

impl TryFrom<proto::ActiveContract> for ActiveContract {
    type Error = ValueError;

    fn try_from(value: proto::ActiveContract) -> Result<Self, Self::Error> {
        Ok(Self {
            created_event: value
                .created_event
                .required_of::<proto::ActiveContract>("created_event")
                .no_msg()?
                .try_into()
                .validated_of::<proto::ActiveContract>("created_event")
                .no_msg()?,
            synchronizer_id: SynchronizerId::new(value.synchronizer_id)
                .validated_of::<proto::ActiveContract>("synchronizer_id")
                .no_msg()?,
            reassignment_counter: value.reassignment_counter,
        })
    }
}

#[derive(Clone, Debug)]
pub struct IncompleteUnassigned {
    // TODO: implement
}

#[derive(Clone, Debug)]
pub struct IncompleteAssigned {
    // TODO: implement
}
