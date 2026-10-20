use canton_types::PartyId;
use ledger_api_proto::com::daml::ledger::api::v2 as proto;

#[derive(Clone, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct TopologyFormat {
    pub include_participant_authorization_events: Option<ParticipantAuthorizationTopologyFormat>,
}

impl TopologyFormat {
    pub fn new() -> Self {
        Self {
            include_participant_authorization_events: None,
        }
    }

    pub fn include_participant_authorization_events(
        mut self,
        format: ParticipantAuthorizationTopologyFormat,
    ) -> Self {
        self.include_participant_authorization_events = Some(format);
        self
    }
}

impl From<TopologyFormat> for proto::TopologyFormat {
    fn from(value: TopologyFormat) -> Self {
        Self {
            include_participant_authorization_events: value
                .include_participant_authorization_events
                .map(Into::into),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParticipantAuthorizationTopologyFormat {
    pub parties: Vec<PartyId>,
}

impl ParticipantAuthorizationTopologyFormat {
    pub const fn all() -> Self {
        Self {
            parties: Vec::new(),
        }
    }

    pub fn for_parties(parties: Vec<PartyId>) -> Self {
        Self { parties }
    }
}

impl From<ParticipantAuthorizationTopologyFormat>
    for proto::ParticipantAuthorizationTopologyFormat
{
    fn from(value: ParticipantAuthorizationTopologyFormat) -> Self {
        Self {
            parties: value.parties.into_iter().map(Into::into).collect(),
        }
    }
}
