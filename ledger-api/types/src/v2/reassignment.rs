use std::time::SystemTime;

use canton_types::{ContractId, LedgerString};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use nonempty::NonEmpty;
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

#[derive(Clone, Debug)]
pub struct Reassignment {
    pub update_id: LedgerString,
    pub command_id: Option<LedgerString>,
    pub workflow_id: Option<LedgerString>,
    pub offset: i64,
    pub events: NonEmpty<ReassignmentEvent>,
    // pub trace_context: Option<TraceContext>,
    pub record_time: SystemTime,
    // pub synchronizer_id: SynchronizerId,
    pub paid_traffic_cost: Option<i64>,
    // TODO: impl missing fields
}

impl TryFrom<proto::Reassignment> for Reassignment {
    type Error = ValueError;

    fn try_from(value: proto::Reassignment) -> Result<Self, Self::Error> {
        let update_id = LedgerString::new(value.update_id)
            .validated_of::<proto::Reassignment>("update_id")
            .no_msg()?;
        let command_id = (!value.command_id.is_empty())
            .then(|| {
                LedgerString::new(value.command_id)
                    .validated_of::<proto::Reassignment>("command_id")
                    .no_msg()
            })
            .transpose()?;
        let workflow_id = (!value.workflow_id.is_empty())
            .then(|| {
                LedgerString::new(value.workflow_id)
                    .validated_of::<proto::Reassignment>("workflow_id")
                    .no_msg()
            })
            .transpose()?;
        let record_time = value
            .record_time
            .required_of::<proto::Reassignment>("record_time")
            .no_msg()?
            .try_into()
            .unwrap(); // FIXME: replace unwrap with error

        let mut events = value
            .events
            .into_iter()
            .enumerate()
            .map(|(idx, event)| {
                event
                    .try_into()
                    .validated_of::<proto::Reassignment>(format!("event[{idx}]"))
                    .no_msg()
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter();
        let head = events
            .next()
            .ok_or_else(|| ValueError::raw_message("expected non-empty list"))
            .validated_of::<proto::Reassignment>("events")
            .no_msg()?;
        let tail = events.collect();
        let events = NonEmpty { head, tail };

        Ok(Self {
            update_id,
            command_id,
            workflow_id,
            offset: value.offset,
            events,
            record_time,
            paid_traffic_cost: value.paid_traffic_cost,
        })
    }
}

#[derive(Clone, Debug)]
pub enum ReassignmentEvent {
    Unassigned(UnassignedEvent),
    Assigned(AssignedEvent),
}

impl TryFrom<proto::ReassignmentEvent> for ReassignmentEvent {
    type Error = ValueError;

    fn try_from(value: proto::ReassignmentEvent) -> Result<Self, Self::Error> {
        use proto::reassignment_event::Event::*;
        let event = value
            .event
            .required_of::<proto::ReassignmentEvent>("event")
            .no_msg()?;
        match event {
            Unassigned(event) => event.try_into().map(Self::Unassigned),
            Assigned(event) => event.try_into().map(Self::Assigned),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UnassignedEvent {
    pub reassignment_id: LedgerString,
    pub contract_id: ContractId,
}

impl TryFrom<proto::UnassignedEvent> for UnassignedEvent {
    type Error = ValueError;

    fn try_from(_: proto::UnassignedEvent) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct AssignedEvent {}

impl TryFrom<proto::AssignedEvent> for AssignedEvent {
    type Error = ValueError;

    fn try_from(_: proto::AssignedEvent) -> Result<Self, Self::Error> {
        todo!()
    }
}
