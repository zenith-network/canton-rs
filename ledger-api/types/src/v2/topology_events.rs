use canton_types::{ParticipantId, PartyId};
use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _, UnspecifiedEnumValue};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum TopologyEvent {
    ParticipantAuthorizationChanged(ParticipantAuthorizationChanged),
    ParticipantAuthorizationRevoked(ParticipantAuthorizationRevoked),
    ParticipantAuthorizationAdded(ParticipantAuthorizationAdded),
    ParticipantAuthorizationOnboarding(ParticipantAuthorizationOnboarding),
}

impl TryFrom<proto::TopologyEvent> for TopologyEvent {
    type Error = ValueError;

    fn try_from(value: proto::TopologyEvent) -> Result<Self, Self::Error> {
        use proto::topology_event::Event;

        let event = value
            .event
            .required_of::<proto::TopologyEvent>("event")
            .no_msg()?;

        match event {
            Event::ParticipantAuthorizationChanged(event) => {
                event.try_into().map(Self::ParticipantAuthorizationChanged)
            }
            Event::ParticipantAuthorizationRevoked(event) => {
                event.try_into().map(Self::ParticipantAuthorizationRevoked)
            }
            Event::ParticipantAuthorizationAdded(event) => {
                event.try_into().map(Self::ParticipantAuthorizationAdded)
            }
            Event::ParticipantAuthorizationOnboarding(event) => event
                .try_into()
                .map(Self::ParticipantAuthorizationOnboarding),
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ParticipantAuthorizationChanged {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationChanged> for ParticipantAuthorizationChanged {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationChanged) -> Result<Self, Self::Error> {
        type Proto = proto::ParticipantAuthorizationChanged;

        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<Proto>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<Proto>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<Proto>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<Proto>("participant_permission")
            .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ParticipantAuthorizationRevoked {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
}

impl TryFrom<proto::ParticipantAuthorizationRevoked> for ParticipantAuthorizationRevoked {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationRevoked) -> Result<Self, Self::Error> {
        type Proto = proto::ParticipantAuthorizationRevoked;

        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<Proto>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<Proto>("participant_id")
                .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ParticipantAuthorizationAdded {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationAdded> for ParticipantAuthorizationAdded {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationAdded) -> Result<Self, Self::Error> {
        type Proto = proto::ParticipantAuthorizationAdded;

        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<Proto>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<Proto>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<Proto>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<Proto>("participant_permission")
            .no_msg()?,
        })
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ParticipantAuthorizationOnboarding {
    pub party_id: PartyId,
    pub participant_id: ParticipantId,
    pub participant_permission: ParticipantPermission,
}

impl TryFrom<proto::ParticipantAuthorizationOnboarding> for ParticipantAuthorizationOnboarding {
    type Error = ValueError;

    fn try_from(value: proto::ParticipantAuthorizationOnboarding) -> Result<Self, Self::Error> {
        type Proto = proto::ParticipantAuthorizationOnboarding;

        Ok(Self {
            party_id: PartyId::new(value.party_id)
                .validated_of::<Proto>("party_id")
                .no_msg()?,
            participant_id: ParticipantId::new(value.participant_id)
                .validated_of::<Proto>("participant_id")
                .no_msg()?,
            participant_permission: proto::ParticipantPermission::try_from(
                value.participant_permission,
            )
            .validated_of::<Proto>("participant_permission")
            .no_msg()?
            .try_into()
            .validated_of::<Proto>("participant_permission")
            .no_msg()?,
        })
    }
}

// FIXME: This actually duplicates the type from `canton-protocol`. I put it here for now to not
// complicate things. However a refactor needed: such types should be defined in some generic crate.
// Then there should be some compatibility crate, which will define, how those types are represented
// on the protocol level and for different APIs. I leave this work out of the question for now,
// cause it needs more thinking. For now we will simply live with this duplicate type.

/// Participant permission for Ledger API
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ParticipantPermission {
    Observation,
    Confirmation,
    Submission,
}

impl TryFrom<proto::ParticipantPermission> for ParticipantPermission {
    type Error = UnspecifiedEnumValue;

    fn try_from(value: proto::ParticipantPermission) -> Result<Self, Self::Error> {
        match value {
            proto::ParticipantPermission::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::ParticipantPermission::Submission => Ok(Self::Submission),
            proto::ParticipantPermission::Confirmation => Ok(Self::Confirmation),
            proto::ParticipantPermission::Observation => Ok(Self::Observation),
        }
    }
}
