use canton_types::SynchronizerId;

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v2::{CreatedEvent, HasOffset};

#[derive(Clone, Debug)]
// TODO: Since last two variants are not yet implemented, there is no point to check this at the
//       moment. This attribute needs to be removed after IncompleteUnassigned and
//       IncompleteAssigned are implemented.
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
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

impl HasOffset for ContractEntry {
    fn offset(&self) -> i64 {
        match self {
            ContractEntry::ActiveContract(active_contract) => active_contract.offset(),
            ContractEntry::IncompleteUnassigned(incomplete_unassigned) => {
                incomplete_unassigned.offset()
            }
            ContractEntry::IncompleteAssigned(incomplete_assigned) => incomplete_assigned.offset(),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
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

impl HasOffset for ActiveContract {
    fn offset(&self) -> i64 {
        self.created_event.offset()
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct IncompleteUnassigned {
    // TODO: implement
}

impl HasOffset for IncompleteUnassigned {
    fn offset(&self) -> i64 {
        todo!()
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct IncompleteAssigned {
    // TODO: implement
}

impl HasOffset for IncompleteAssigned {
    fn offset(&self) -> i64 {
        todo!()
    }
}
