use canton_crypto::v30::SigningKeysWithThreshold;
use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use canton_types::{
    NonZeroU31,
    topology::{ParticipantId, PartyId, UniqueIdentifier},
};
use protobuf_utils::{InvalidProtoField as _, UnspecifiedEnumValue};

use crate::error::ConvertError;

/// Mapping that maps a party to a participant (PTP)
///
/// The [`PartyToParticipant`] mapping may also specify a list of signing keys for setting up an
/// external party, in which case the keys and the threshold take precedence over any
/// `PartyToKeyMapping` for the same party.
///
/// Additionally, the list of signing keys may contain the public key of the party's namespace,
/// which allows this mapping to authorize itself without the need of a
/// [`NamespaceDelegation`][crate::protocol::v30::mappings::NamespaceDelegation] root certificate
/// (called self-signed).
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
/// [`NamespaceDelegation`][crate::protocol::v30::mappings::NamespaceDelegation] root certificate
/// for that namespace.
///
/// ## Uniqueness key
///
/// ```plaintext
/// party
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PartyToParticipant {
    /// The party that is to be represented by the participants
    pub party: PartyId,

    /// The signatory threshold required by the participants to be able to act on behalf of the
    /// party.
    ///
    /// A mapping with `threshold > 1` is considered a definition of a _consortium party_.
    pub threshold: NonZeroU31,

    /// Which participants will host the party.
    ///
    /// If `threshold > 1`, should be [`Confirmation`][ParticipantPermission::Confirmation] or
    /// [`Observation`][ParticipantPermission::Observation]. Submission with `threshold > 1` will
    /// always fail.
    ///
    /// If all participants have [`Observation`][ParticipantPermission::Observation] permission, the
    /// confirmation threshold is ignored, making the party a purely observing party.
    pub participants: Vec<HostingParticipant>,

    /// Contains protocol signing keys for the party used to authorize externally signed Daml
    /// transactions, along with a signing threshold.
    ///
    /// The max number of keys is 20.
    pub party_signing_keys: Option<SigningKeysWithThreshold>,
}

impl From<PartyToParticipant> for proto::PartyToParticipant {
    fn from(value: PartyToParticipant) -> Self {
        todo!()
    }
}

impl TryFrom<proto::PartyToParticipant> for PartyToParticipant {
    type Error = ConvertError;

    fn try_from(value: proto::PartyToParticipant) -> Result<Self, Self::Error> {
        todo!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct Onboarding {}

impl Onboarding {
    pub fn new() -> Self {
        Self::default()
    }
}

impl From<Onboarding> for proto::party_to_participant::hosting_participant::Onboarding {
    fn from(_: Onboarding) -> Self {
        Self {}
    }
}

impl TryFrom<proto::party_to_participant::hosting_participant::Onboarding> for Onboarding {
    type Error = ConvertError;

    fn try_from(
        _: proto::party_to_participant::hosting_participant::Onboarding,
    ) -> Result<Self, Self::Error> {
        Ok(Self {})
    }
}

/// Hosting properties of the participant
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HostingParticipant {
    /// The target participant that the party should be mapped to
    pub participant_uid: ParticipantId,

    /// Permission of the participant for this particular party
    ///
    /// The actual will be min of `ParticipantSynchronizerPermission.ParticipantPermission` and this
    /// setting.
    pub permission: ParticipantPermission,

    /// Set to `Some` iff the party is being onboarded to the participant
    pub onboarding: Option<Onboarding>,
}

impl HostingParticipant {
    /// Create new hosting participant mapping
    pub fn new(
        participant_uid: ParticipantId,
        permission: ParticipantPermission,
        onboarding: Option<Onboarding>,
    ) -> Self {
        Self {
            participant_uid,
            permission,
            onboarding,
        }
    }

    /// Create new hosting participant with onboarding flag set
    pub fn new_onboarding(
        participant_uid: ParticipantId,
        permission: ParticipantPermission,
    ) -> Self {
        Self {
            participant_uid,
            permission,
            onboarding: Some(Onboarding::default()),
        }
    }

    /// Set to `Some` iff the party is being onboarded to the participant
    pub fn is_onboarding(&self) -> bool {
        self.onboarding.is_some()
    }
}

impl From<HostingParticipant> for proto::party_to_participant::HostingParticipant {
    fn from(value: HostingParticipant) -> Self {
        Self {
            participant_uid: UniqueIdentifier::from(value.participant_uid).into(),
            permission: proto::enums::ParticipantPermission::from(value.permission).into(),
            onboarding: value.onboarding.map(Into::into),
        }
    }
}

impl TryFrom<proto::party_to_participant::HostingParticipant> for HostingParticipant {
    type Error = ConvertError;

    fn try_from(
        value: proto::party_to_participant::HostingParticipant,
    ) -> Result<Self, Self::Error> {
        let participant_uid = UniqueIdentifier::parse(&value.participant_uid)
            .validated_of::<proto::party_to_participant::HostingParticipant>("")?
            .into();
        let permission = proto::enums::ParticipantPermission::try_from(value.permission)
            .validated_of::<proto::party_to_participant::HostingParticipant>("value.permission")?
            .try_into()
            .validated_of::<proto::party_to_participant::HostingParticipant>("value.permission")?;
        let onboarding =
            value
                .onboarding
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::party_to_participant::HostingParticipant>("onboarding")?;

        Ok(Self::new(participant_uid, permission, onboarding))
    }
}

/// Participant permission level
///
/// Regardless of the participant permission level, all participants can submit a reassignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ParticipantPermission {
    /// Participant is active, can submit transactions and reassignments
    Submission,
    /// Participant is passive, can only confirm transactions and submit reassignments
    Confirmation,
    /// Participant is passive, can only observe transactions and submit reassignments
    Observation,
}

impl From<ParticipantPermission> for proto::enums::ParticipantPermission {
    fn from(value: ParticipantPermission) -> Self {
        use proto::enums::ParticipantPermission::*;

        match value {
            ParticipantPermission::Submission => Submission,
            ParticipantPermission::Confirmation => Confirmation,
            ParticipantPermission::Observation => Observation,
        }
    }
}

impl TryFrom<proto::enums::ParticipantPermission> for ParticipantPermission {
    type Error = UnspecifiedEnumValue;

    fn try_from(value: proto::enums::ParticipantPermission) -> Result<Self, Self::Error> {
        use ParticipantPermission::*;

        match value {
            proto::enums::ParticipantPermission::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::enums::ParticipantPermission::Submission => Ok(Submission),
            proto::enums::ParticipantPermission::Confirmation => Ok(Confirmation),
            proto::enums::ParticipantPermission::Observation => Ok(Observation),
        }
    }
}

// impl TryFrom<i32> for ParticipantPermission {
//     type Error = ValueError;

//     fn try_from(value: i32) -> Result<Self, Self::Error> {
//         proto::enums::ParticipantPermission::try_from(value)?
//             .try_into()
//             .map_err(Into::into)
//     }
// }
