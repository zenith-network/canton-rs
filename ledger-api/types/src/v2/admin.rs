use canton_types::{LedgerString, PartyId};
use ledger_api_proto::com::daml::ledger::api::v2::admin as proto;
use ledger_api_value::v2::errors::{IntoValueError as _, ValueError};
use protobuf_utils::InvalidProtoField as _;

/// Party details
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyDetails {
    /// The stable unique identifier of a Daml party.
    pub party: PartyId,

    /// true if party is hosted by the participant and the party shares the same identity provider as the user issuing the request.
    pub is_local: bool,

    /// Identity provider ID
    ///
    /// If set to `None`, there could be 3 options:
    ///
    /// 1. the party is managed by the default identity provider.
    /// 2. party is not hosted by the participant.
    /// 3. party is hosted by the participant, but is outside of the user's identity provider.
    pub identity_provider_id: Option<LedgerString>,
    // TODO: implement missing local_metadata field
}

impl From<PartyDetails> for proto::PartyDetails {
    fn from(value: PartyDetails) -> Self {
        Self {
            party: value.party.into(),
            is_local: value.is_local,
            local_metadata: None, // TODO: add conversion when the field is implemented
            identity_provider_id: value
                .identity_provider_id
                .map(Into::into)
                .unwrap_or_default(),
        }
    }
}

impl TryFrom<proto::PartyDetails> for PartyDetails {
    type Error = ValueError;

    fn try_from(value: proto::PartyDetails) -> Result<Self, Self::Error> {
        let party = PartyId::new(value.party)
            .validated_of::<proto::PartyDetails>("party")
            .no_msg()?;
        let identity_provider_id = (!value.identity_provider_id.is_empty())
            .then(|| {
                LedgerString::new(value.identity_provider_id)
                    .validated_of::<proto::PartyDetails>("identity_provider_id")
                    .no_msg()
            })
            .transpose()?;

        Ok(Self {
            party,
            is_local: value.is_local,
            identity_provider_id,
        })
    }
}
