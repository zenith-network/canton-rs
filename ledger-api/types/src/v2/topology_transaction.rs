use std::{fmt, time::SystemTime};

use canton_types::{LedgerString, NonEmpty, ParticipantId, PartyId, SynchronizerId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

#[derive(Clone, Debug)]
pub struct TopologyTransaction {
    /// Assigned by the server. Useful for correlating logs.
    pub update_id: LedgerString,

    /// The absolute offset. It is a valid absolute offset (positive integer).
    pub offset: i64,

    /// A valid synchronizer ID.
    ///
    /// Identifies the synchronizer that synchronized the topology transaction.
    pub synchronizer_id: SynchronizerId,

    /// The time at which the changes in the topology transaction become effective. There is a small delay between a
    /// topology transaction being sequenced and the changes it contains becoming effective. Topology transactions appear
    /// in order relative to a synchronizer based on their effective time rather than their sequencing time.
    pub record_time: SystemTime,

    /// A list of topology events.
    pub events: NonEmpty<TopologyEvent>,
    // pub trace_context: Option<TraceContext>,
    // TODO: implement missing field
}

impl TryFrom<proto::TopologyTransaction> for TopologyTransaction {
    type Error = ValueError;

    fn try_from(value: proto::TopologyTransaction) -> Result<Self, Self::Error> {
        Ok(Self {
            update_id: LedgerString::new(value.update_id)
                .validated_of::<proto::TopologyTransaction>("update_id")
                .no_msg()?,
            offset: value.offset,
            synchronizer_id: SynchronizerId::new(value.synchronizer_id)
                .validated_of::<proto::TopologyTransaction>("synchronizer_id")
                .no_msg()?,
            record_time: value
                .record_time
                .required_of::<proto::TopologyTransaction>("record_time")
                .no_msg()?
                .try_into()
                .unwrap(), // FIXME: change unwrap to error
            events: NonEmpty::try_from(
                value
                    .events
                    .into_iter()
                    .enumerate()
                    .map(|(idx, event)| {
                        event
                            .try_into()
                            .with_msg_owned(format!("failed to convert event[{idx}]"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .map_err(|_| ValueError::raw_message("empty events list"))
            .validated_of::<proto::TopologyTransaction>("events")
            .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
pub enum TopologyEvent {
    ParticipantAuthorizationChanged(ParticipantAuthorizationChanged),
    ParticipantAuthorizationRevoked(ParticipantAuthorizationRevoked),
    ParticipantAuthorizationAdded(ParticipantAuthorizationAdded),
    ParticipantAuthorizationOnboarding(ParticipantAuthorizationOnboarding),
}

impl TryFrom<proto::TopologyEvent> for TopologyEvent {
    type Error = ValueError;

    fn try_from(value: proto::TopologyEvent) -> Result<Self, Self::Error> {
        use proto::topology_event::Event::*;
        let event = value
            .event
            .required_of::<proto::TopologyEvent>("event")
            .no_msg()?;
        match event {
            ParticipantAuthorizationChanged(event) => {
                event.try_into().map(Self::ParticipantAuthorizationChanged)
            }
            ParticipantAuthorizationRevoked(event) => {
                event.try_into().map(Self::ParticipantAuthorizationRevoked)
            }
            ParticipantAuthorizationAdded(event) => {
                event.try_into().map(Self::ParticipantAuthorizationAdded)
            }
            ParticipantAuthorizationOnboarding(event) => event
                .try_into()
                .map(Self::ParticipantAuthorizationOnboarding),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ParticipantAuthorizationChanged {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationChanged> for ParticipantAuthorizationChanged {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationChanged) -> Result<Self, Self::Error> {
        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<proto::ParticipantAuthorizationChanged>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<proto::ParticipantAuthorizationChanged>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<proto::ParticipantAuthorizationChanged>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ParticipantAuthorizationChanged>("participant_permission")
            .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ParticipantAuthorizationRevoked {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
}

impl TryFrom<proto::ParticipantAuthorizationRevoked> for ParticipantAuthorizationRevoked {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationRevoked) -> Result<Self, Self::Error> {
        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<proto::ParticipantAuthorizationRevoked>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<proto::ParticipantAuthorizationRevoked>("participant_id")
                .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ParticipantAuthorizationAdded {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationAdded> for ParticipantAuthorizationAdded {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationAdded) -> Result<Self, Self::Error> {
        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<proto::ParticipantAuthorizationAdded>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<proto::ParticipantAuthorizationAdded>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<proto::ParticipantAuthorizationAdded>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ParticipantAuthorizationAdded>("participant_permission")
            .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ParticipantAuthorizationOnboarding {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationOnboarding> for ParticipantAuthorizationOnboarding {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationOnboarding) -> Result<Self, Self::Error> {
        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<proto::ParticipantAuthorizationOnboarding>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<proto::ParticipantAuthorizationOnboarding>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<proto::ParticipantAuthorizationOnboarding>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<proto::ParticipantAuthorizationOnboarding>("participant_permission")
            .no_msg()?,
        })
    }
}

/// Permission level that the participant has for the party.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ParticipantPermission {
    /// Participant can submit transactions
    Submission,
    /// Participant can only confirm transactions
    Confirmation,
    /// Participant can only observe transactions
    Observation,
}

impl TryFrom<proto::ParticipantPermission> for ParticipantPermission {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantPermission) -> Result<Self, Self::Error> {
        match value {
            proto::ParticipantPermission::Unspecified => Err(ValueError::raw_message(
                "unspecified participant permission",
            )),
            proto::ParticipantPermission::Submission => Ok(Self::Submission),
            proto::ParticipantPermission::Confirmation => Ok(Self::Confirmation),
            proto::ParticipantPermission::Observation => Ok(Self::Observation),
        }
    }
}

impl fmt::Display for ParticipantPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Submission => write!(f, "PARTICIPANT_PERMISSION_SUBMISSION"),
            Self::Confirmation => write!(f, "PARTICIPANT_PERMISSION_CONFIRMATION"),
            Self::Observation => write!(f, "PARTICIPANT_PERMISSION_OBSERVATION"),
        }
    }
}
