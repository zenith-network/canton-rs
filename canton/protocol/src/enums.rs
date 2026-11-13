use core::fmt;

use canton_proto::com::digitalasset::canton::protocol::v30::enums as proto_v30;
use protobuf_utils::UnspecifiedEnumValue;

use crate::error::ProtoDecodeError;

/// Participant permission level
///
/// Regardless of the participant permission level, all participants can submit a reassignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ParticipantPermission {
    /// Participant is passive, can only observe transactions and submit reassignments
    Observation,
    /// Participant is passive, can only confirm transactions and submit reassignments
    Confirmation,
    /// Participant is active, can submit transactions and reassignments
    Submission,
}

impl ParticipantPermission {
    /// Returns `true` if a participant with this permission can confirm transactions.
    ///
    /// Note: For a participant to be able to confirm, having `Submission` or `Confirmation`
    /// permission is a necessary, but not sufficient condition (e.g.
    /// [`HostingParticipant.onboarding`][crate::mappings::HostingParticipant::onboarding]
    /// must be `false`). To prevent accidental use of this method, this is kept `pub(crate)`.
    pub(crate) fn can_confirm(&self) -> bool {
        match self {
            ParticipantPermission::Submission | ParticipantPermission::Confirmation => true,
            ParticipantPermission::Observation => false,
        }
    }

    pub fn into_proto_v30(self) -> proto_v30::ParticipantPermission {
        match self {
            Self::Submission => proto_v30::ParticipantPermission::Submission,
            Self::Confirmation => proto_v30::ParticipantPermission::Confirmation,
            Self::Observation => proto_v30::ParticipantPermission::Observation,
        }
    }

    pub fn from_proto_v30(
        proto: proto_v30::ParticipantPermission,
    ) -> Result<Self, UnspecifiedEnumValue> {
        match proto {
            proto_v30::ParticipantPermission::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto_v30::ParticipantPermission::Submission => Ok(Self::Submission),
            proto_v30::ParticipantPermission::Confirmation => Ok(Self::Confirmation),
            proto_v30::ParticipantPermission::Observation => Ok(Self::Observation),
        }
    }

    pub fn into_proto_v30_primitive(self) -> i32 {
        self.into_proto_v30().into()
    }

    pub fn from_proto_v30_primitive(value: i32) -> Result<Self, ProtoDecodeError> {
        let proto = proto_v30::ParticipantPermission::try_from(value)?;
        Self::from_proto_v30(proto).map_err(Into::into)
    }
}

/// Topology mapping code
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TopologyMappingCode {
    NamespaceDelegation,
    DecentralizedNamespaceDefinition,
    OwnerToKeyMapping,
    SynchronizerTrustCertificate,
    ParticipantPermission,
    PartyHostingLimits,
    VettedPackages,
    PartyToParticipant,
    SynchronizerParametersState,
    MediatorSynchronizerState,
    SequencerSynchronizerState,
    SequencingDynamicParametersState,
    PartyToKeyMapping,
    LsuAnnouncement,
    SequencerConnectionSuccessor,
}

impl TopologyMappingCode {
    pub fn into_proto_v30(self) -> proto_v30::TopologyMappingCode {
        match self {
            TopologyMappingCode::NamespaceDelegation => {
                proto_v30::TopologyMappingCode::NamespaceDelegation
            }
            TopologyMappingCode::DecentralizedNamespaceDefinition => {
                proto_v30::TopologyMappingCode::DecentralizedNamespaceDefinition
            }
            TopologyMappingCode::OwnerToKeyMapping => {
                proto_v30::TopologyMappingCode::OwnerToKeyMapping
            }
            TopologyMappingCode::SynchronizerTrustCertificate => {
                proto_v30::TopologyMappingCode::SynchronizerTrustCertificate
            }
            TopologyMappingCode::ParticipantPermission => {
                proto_v30::TopologyMappingCode::ParticipantPermission
            }
            TopologyMappingCode::PartyHostingLimits => {
                proto_v30::TopologyMappingCode::PartyHostingLimits
            }
            TopologyMappingCode::VettedPackages => proto_v30::TopologyMappingCode::VettedPackages,
            TopologyMappingCode::PartyToParticipant => {
                proto_v30::TopologyMappingCode::PartyToParticipant
            }
            TopologyMappingCode::SynchronizerParametersState => {
                proto_v30::TopologyMappingCode::SynchronizerParametersState
            }
            TopologyMappingCode::MediatorSynchronizerState => {
                proto_v30::TopologyMappingCode::MediatorSynchronizerState
            }
            TopologyMappingCode::SequencerSynchronizerState => {
                proto_v30::TopologyMappingCode::SequencerSynchronizerState
            }
            TopologyMappingCode::SequencingDynamicParametersState => {
                proto_v30::TopologyMappingCode::SequencingDynamicParametersState
            }
            TopologyMappingCode::PartyToKeyMapping => {
                proto_v30::TopologyMappingCode::PartyToKeyMapping
            }
            TopologyMappingCode::LsuAnnouncement => proto_v30::TopologyMappingCode::LsuAnnouncement,
            TopologyMappingCode::SequencerConnectionSuccessor => {
                proto_v30::TopologyMappingCode::SequencerConnectionSuccessor
            }
        }
    }

    pub fn from_proto_v30(
        proto: proto_v30::TopologyMappingCode,
    ) -> Result<Self, UnspecifiedEnumValue> {
        match proto {
            proto_v30::TopologyMappingCode::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto_v30::TopologyMappingCode::NamespaceDelegation => Ok(Self::NamespaceDelegation),
            proto_v30::TopologyMappingCode::DecentralizedNamespaceDefinition => {
                Ok(Self::DecentralizedNamespaceDefinition)
            }
            proto_v30::TopologyMappingCode::OwnerToKeyMapping => Ok(Self::OwnerToKeyMapping),
            proto_v30::TopologyMappingCode::SynchronizerTrustCertificate => {
                Ok(Self::SynchronizerTrustCertificate)
            }
            proto_v30::TopologyMappingCode::ParticipantPermission => {
                Ok(Self::ParticipantPermission)
            }
            proto_v30::TopologyMappingCode::PartyHostingLimits => Ok(Self::PartyHostingLimits),
            proto_v30::TopologyMappingCode::VettedPackages => Ok(Self::VettedPackages),
            proto_v30::TopologyMappingCode::PartyToParticipant => Ok(Self::PartyToParticipant),
            proto_v30::TopologyMappingCode::SynchronizerParametersState => {
                Ok(Self::SynchronizerParametersState)
            }
            proto_v30::TopologyMappingCode::MediatorSynchronizerState => {
                Ok(Self::MediatorSynchronizerState)
            }
            proto_v30::TopologyMappingCode::SequencerSynchronizerState => {
                Ok(Self::SequencerSynchronizerState)
            }
            proto_v30::TopologyMappingCode::SequencingDynamicParametersState => {
                Ok(Self::SequencingDynamicParametersState)
            }
            proto_v30::TopologyMappingCode::PartyToKeyMapping => Ok(Self::PartyToKeyMapping),
            proto_v30::TopologyMappingCode::LsuAnnouncement => Ok(Self::LsuAnnouncement),
            proto_v30::TopologyMappingCode::SequencerConnectionSuccessor => {
                Ok(Self::SequencerConnectionSuccessor)
            }
        }
    }

    pub fn into_proto_v30_primitive(self) -> i32 {
        self.into_proto_v30().into()
    }

    pub fn from_proto_v30_primitive(value: i32) -> Result<Self, ProtoDecodeError> {
        let proto = proto_v30::TopologyMappingCode::try_from(value)?;
        Self::from_proto_v30(proto).map_err(Into::into)
    }

    /// Returns short (abbreviated) string form of the code
    ///
    /// Matches the code names in Canton.
    pub fn as_str(&self) -> &'static str {
        match self {
            TopologyMappingCode::NamespaceDelegation => "nsd",
            TopologyMappingCode::DecentralizedNamespaceDefinition => "dnd",
            TopologyMappingCode::OwnerToKeyMapping => "otk",
            TopologyMappingCode::SynchronizerTrustCertificate => "dtc",
            TopologyMappingCode::ParticipantPermission => "pdp",
            TopologyMappingCode::PartyHostingLimits => "phl",
            TopologyMappingCode::VettedPackages => "vtp",
            TopologyMappingCode::PartyToParticipant => "ptp",
            TopologyMappingCode::SynchronizerParametersState => "dop",
            TopologyMappingCode::MediatorSynchronizerState => "mds",
            TopologyMappingCode::SequencerSynchronizerState => "sds",
            TopologyMappingCode::SequencingDynamicParametersState => "sep",
            TopologyMappingCode::PartyToKeyMapping => "ptk",
            TopologyMappingCode::LsuAnnouncement => "lsu",
            TopologyMappingCode::SequencerConnectionSuccessor => "scs",
        }
    }
}

impl fmt::Display for TopologyMappingCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::ParticipantPermission;

    #[test]
    fn test_participant_permission_order() {
        assert!(ParticipantPermission::Observation < ParticipantPermission::Confirmation);
        assert!(ParticipantPermission::Confirmation < ParticipantPermission::Submission);
    }
}
