use canton_crypto::v30::SigningKeysWithThreshold;
use canton_proto::com::digitalasset::canton::protocol::{v30, v31};
use canton_types::{
    NonZeroU31,
    topology::{ParticipantId, PartyId, UniqueIdentifier},
};
use protobuf_utils::{InvalidProtoField as _, InvalidProtoFieldValue, UnspecifiedEnumValue};

use crate::{
    ParticipantPermission, TopologyMappingCode, TopologyMappingVariant,
    error::{
        IncompatibleMessageSchema, ProtoDecodeError, ProtoEncodeError, UnexpectedTopologyMapping,
    },
    mappings::TopologyMapping,
};

/// Mapping that maps a party to a participant (PTP)
///
/// The [`PartyToParticipant`] mapping may also specify a list of signing keys for setting up an
/// external party, in which case the keys and the threshold take precedence over any
/// `PartyToKeyMapping` for the same party.
///
/// Additionally, the list of signing keys may contain the public key of the party's namespace,
/// which allows this mapping to authorize itself without the need of a
/// [`NamespaceDelegation`][crate::mappings::NamespaceDelegation] root certificate (called
/// self-signed).
///
/// ## Authorization
///
/// The required authorization of the mapping is a union of the authorization for individual changes
///
/// - threshold change: party namespace
/// - adding a signing key: party namespace + all the new signing key
/// - removing a signing key: party namespace
/// - changing the signing key threshold: party namespace
/// - upgrading a participant permission or adding a new participant: namespaces from party and the
///   participant namespace
/// - downgrading a participant permission or removing a participant: party namespace OR the
///   participant namespace
/// - setting a participant's onboarding flag from false to true: party namespace
/// - setting a participant's onboarding flag from true to false: participant namespace
/// - the removal of a PTP must be authorized just by the party
///
/// ## Revocation
///
/// Revoking a self-signed PTP does not prevent later re-creation of a PTP with the same party ID.
/// To prevent further usage of the key associated with the party's namespace, revoke a
/// [`NamespaceDelegation`][crate::mappings::NamespaceDelegation] root certificate for that
/// namespace.
///
/// ## Uniqueness key
///
/// ```plaintext
/// party
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartyToParticipant {
    party: PartyId,
    threshold: NonZeroU31,
    participants: Vec<HostingParticipant>,
    party_signing_keys: Option<SigningKeysWithThreshold>,
}

// This is currently the value defined in Canton
const MAX_PARTY_SIGNING_KEYS: usize = 20;

impl PartyToParticipant {
    /// Code of the topology mapping
    pub const CODE: TopologyMappingCode = TopologyMappingCode::PartyToParticipant;

    /// Create new party to participant mapping
    pub fn new(
        party: PartyId,
        threshold: NonZeroU31,
        participants: Vec<HostingParticipant>,
        party_signing_keys: Option<SigningKeysWithThreshold>,
    ) -> Result<Self, PtpError> {
        if party_signing_keys
            .as_ref()
            .is_some_and(|psk| psk.keys().len().get() > MAX_PARTY_SIGNING_KEYS)
        {
            return Err(PtpError::TooManyPartySigningKeys);
        }

        Ok(Self {
            party,
            threshold,
            participants,
            party_signing_keys,
        })
    }

    /// The party that is to be represented by the participants
    pub fn party(&self) -> &PartyId {
        &self.party
    }

    /// The signatory threshold required by the participants to be able to act on behalf of the
    /// party.
    ///
    /// A mapping with `threshold > 1` is considered a definition of a _consortium party_.
    pub fn threshold(&self) -> NonZeroU31 {
        self.threshold
    }

    /// Which participants will host the party.
    ///
    /// If `threshold > 1`, should be [`Confirmation`][ParticipantPermission::Confirmation] or
    /// [`Observation`][ParticipantPermission::Observation]. Submission with `threshold > 1` will
    /// always fail.
    ///
    /// If all participants have [`Observation`][ParticipantPermission::Observation] permission, the
    /// confirmation threshold is ignored, making the party a purely observing party.
    pub fn participants(&self) -> &[HostingParticipant] {
        &self.participants
    }

    /// Contains protocol signing keys for the party used to authorize externally signed Daml
    /// transactions, along with a signing threshold.
    ///
    /// The max number of keys is 20.
    pub fn party_signing_keys(&self) -> Option<&SigningKeysWithThreshold> {
        self.party_signing_keys.as_ref()
    }
}

impl TopologyMapping for PartyToParticipant {
    type ProtoV30 = v30::PartyToParticipant;
    type ProtoV31 = v31::PartyToParticipant;

    fn code(&self) -> TopologyMappingCode {
        Self::CODE
    }

    fn into_variant(self) -> TopologyMappingVariant {
        TopologyMappingVariant::PartyToParticipant(self)
    }

    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping> {
        match variant {
            TopologyMappingVariant::PartyToParticipant(mapping) => Ok(mapping),
            other => Err(UnexpectedTopologyMapping::new(Self::CODE, other.code())),
        }
    }

    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError> {
        Ok(v30::PartyToParticipant {
            party: self.party.into(),
            threshold: self.threshold.into(),
            participants: self
                .participants
                .into_iter()
                .map(HostingParticipant::into_proto_v30)
                .collect::<Result<_, _>>()?,
            party_signing_keys: self.party_signing_keys.map(Into::into),
        })
    }

    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::PartyToParticipant;

        let party = PartyId::parse(&proto.party).validated_of::<Proto>("party")?;

        let threshold = proto
            .threshold
            .try_into()
            .validated_of::<Proto>("threshold")?;

        let participants = proto
            .participants
            .into_iter()
            .map(HostingParticipant::from_proto_v30)
            .collect::<Result<_, _>>()
            .validated_of::<Proto>("participants")?;

        let party_signing_keys = proto
            .party_signing_keys
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<Proto>("party_signing_keys")?;

        Self::new(party, threshold, participants, party_signing_keys)
            .map_err(|error| {
                let field_name = match error {
                    PtpError::TooManyPartySigningKeys => "party_signing_keys",
                };
                InvalidProtoFieldValue::new_of::<Proto>(field_name, error)
            })
            .map_err(Into::into)
    }

    fn into_proto_v31(self) -> Result<Self::ProtoV31, ProtoEncodeError> {
        Ok(v31::PartyToParticipant {
            party: self.party.into(),
            threshold: self.threshold.into(),
            participants: self
                .participants
                .into_iter()
                .map(HostingParticipant::into_proto_v31)
                .collect(),
            party_signing_keys: self.party_signing_keys.map(Into::into),
        })
    }

    fn from_proto_v31(proto: Self::ProtoV31) -> Result<Self, ProtoDecodeError> {
        type Proto = v31::PartyToParticipant;

        let party = PartyId::parse(&proto.party).validated_of::<Proto>("party")?;

        let threshold = proto
            .threshold
            .try_into()
            .validated_of::<Proto>("threshold")?;

        let participants = proto
            .participants
            .into_iter()
            .map(HostingParticipant::from_proto_v31)
            .collect::<Result<_, _>>()
            .validated_of::<Proto>("participants")?;

        let party_signing_keys = proto
            .party_signing_keys
            .map(TryInto::try_into)
            .transpose()
            .validated_of::<Proto>("party_signing_keys")?;

        Self::new(party, threshold, participants, party_signing_keys)
            .map_err(|error| {
                let field_name = match error {
                    PtpError::TooManyPartySigningKeys => "party_signing_keys",
                };
                InvalidProtoFieldValue::new_of::<Proto>(field_name, error)
            })
            .map_err(Into::into)
    }
}

/// Hosting properties of the participant
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HostingParticipant {
    participant_uid: ParticipantId,
    permission: ParticipantPermission,
    state: HostingState,
}

impl HostingParticipant {
    /// Create new hosting participant mapping
    pub fn new(
        participant_uid: ParticipantId,
        permission: ParticipantPermission,
        state: HostingState,
    ) -> Self {
        Self {
            participant_uid,
            permission,
            state,
        }
    }

    /// The target participant that the party should be mapped to
    pub fn participant_uid(&self) -> &ParticipantId {
        &self.participant_uid
    }

    /// Permission of the participant for this particular party
    ///
    /// The actual will be min of `ParticipantSynchronizerPermission.ParticipantPermission` and this
    /// setting.
    pub fn permission(&self) -> ParticipantPermission {
        self.permission
    }

    /// The hosting state of the participant for this particular party
    pub fn state(&self) -> HostingState {
        self.state
    }

    /// Returns `true` if the participant is in onboarding state of this party
    pub fn onboarding(&self) -> bool {
        self.state == HostingState::Onboarding
    }

    /// Returns `true` if the participant can confirm on behalf of this party
    pub fn can_confirm(&self) -> bool {
        self.permission.can_confirm() && !self.onboarding()
    }

    pub fn into_proto_v30(
        self,
    ) -> Result<v30::party_to_participant::HostingParticipant, ProtoEncodeError> {
        let onboarding = match self.state {
            HostingState::Nominated | HostingState::Freezing => {
                return Err(IncompatibleMessageSchema::new(
                    "Nominated and Freezing states are unsupported by v30 schema",
                )
                .into());
            }
            HostingState::Onboarding => {
                Some(v30::party_to_participant::hosting_participant::Onboarding {})
            }
            HostingState::Active => None,
        };

        Ok(v30::party_to_participant::HostingParticipant {
            participant_uid: UniqueIdentifier::from(self.participant_uid).into(),
            permission: self.permission.into_proto_v30_primitive(),
            onboarding,
        })
    }

    pub fn from_proto_v30(
        proto: v30::party_to_participant::HostingParticipant,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = v30::party_to_participant::HostingParticipant;

        let participant_uid = UniqueIdentifier::parse(&proto.participant_uid)
            .validated_of::<Proto>("participant_uid")?
            .into();

        let permission = ParticipantPermission::from_proto_v30_primitive(proto.permission)
            .validated_of::<Proto>("permission")?;

        let state = if proto.onboarding.is_some() {
            HostingState::Onboarding
        } else {
            HostingState::Active
        };

        Ok(Self::new(participant_uid, permission, state))
    }

    pub fn into_proto_v31(self) -> v31::party_to_participant::HostingParticipant {
        v31::party_to_participant::HostingParticipant {
            participant_uid: UniqueIdentifier::from(self.participant_uid).into(),
            permission: self.permission.into_proto_v30_primitive(),
            state: self.state.into_proto_v31_primitive(),
        }
    }

    pub fn from_proto_v31(
        proto: v31::party_to_participant::HostingParticipant,
    ) -> Result<Self, ProtoDecodeError> {
        type Proto = v31::party_to_participant::HostingParticipant;

        let participant_uid = UniqueIdentifier::parse(&proto.participant_uid)
            .validated_of::<Proto>("participant_uid")?
            .into();

        let permission = ParticipantPermission::from_proto_v30_primitive(proto.permission)
            .validated_of::<Proto>("permission")?;

        let state =
            HostingState::from_proto_v31_primitive(proto.state).validated_of::<Proto>("state")?;

        Ok(Self::new(participant_uid, permission, state))
    }
}

/// The hosting states a participant moves through when it replicates the party, in the order
/// `NOMINATED`, `FREEZING`, `ONBOARDING`, `ACTIVE`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum HostingState {
    /// The party has nominated the participant to host it; the participant has not yet started to
    /// replicate the party.
    Nominated,

    /// The participant replicates the party's active contract set; the party is frozen while any
    /// participant is in this state.
    Freezing,

    /// The participant receives the party's events but does not confirm (equivalent to the v30
    /// `onboarding` flag).
    Onboarding,

    /// The participant hosts the party.
    Active,
}

impl HostingState {
    pub fn into_proto_v31(self) -> v31::party_to_participant::hosting_participant::HostingState {
        type Proto = v31::party_to_participant::hosting_participant::HostingState;

        match self {
            HostingState::Nominated => Proto::Nominated,
            HostingState::Freezing => Proto::Freezing,
            HostingState::Onboarding => Proto::Onboarding,
            HostingState::Active => Proto::Active,
        }
    }

    pub fn from_proto_v31(
        proto: v31::party_to_participant::hosting_participant::HostingState,
    ) -> Result<Self, UnspecifiedEnumValue> {
        type Proto = v31::party_to_participant::hosting_participant::HostingState;

        match proto {
            Proto::Unspecified => Err(UnspecifiedEnumValue::new()),
            Proto::Nominated => Ok(Self::Nominated),
            Proto::Freezing => Ok(Self::Freezing),
            Proto::Onboarding => Ok(Self::Onboarding),
            Proto::Active => Ok(Self::Active),
        }
    }

    pub fn into_proto_v31_primitive(self) -> i32 {
        self.into_proto_v31().into()
    }

    pub fn from_proto_v31_primitive(value: i32) -> Result<Self, ProtoDecodeError> {
        type Proto = v31::party_to_participant::hosting_participant::HostingState;

        let proto = Proto::try_from(value)?;
        Self::from_proto_v31(proto).map_err(Into::into)
    }
}

/// Error in party to participant topology mapping
#[derive(Debug, thiserror::Error)]
pub enum PtpError {
    #[error(
        "too many party signing keys (cannot have more than {})",
        MAX_PARTY_SIGNING_KEYS
    )]
    TooManyPartySigningKeys,
}

#[cfg(test)]
mod tests {
    use super::HostingState;

    #[test]
    fn test_hosting_state_order() {
        assert!(HostingState::Nominated < HostingState::Freezing);
        assert!(HostingState::Freezing < HostingState::Onboarding);
        assert!(HostingState::Onboarding < HostingState::Active);
    }
}
