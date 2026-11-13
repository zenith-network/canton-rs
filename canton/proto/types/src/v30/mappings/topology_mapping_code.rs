use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use protobuf_utils::UnspecifiedEnumValue;

use crate::error::ConvertError;

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

impl From<TopologyMappingCode> for proto::enums::TopologyMappingCode {
    fn from(value: TopologyMappingCode) -> Self {
        use proto::enums::TopologyMappingCode::*;

        match value {
            TopologyMappingCode::NamespaceDelegation => NamespaceDelegation,
            TopologyMappingCode::DecentralizedNamespaceDefinition => {
                DecentralizedNamespaceDefinition
            }
            TopologyMappingCode::OwnerToKeyMapping => OwnerToKeyMapping,
            TopologyMappingCode::SynchronizerTrustCertificate => SynchronizerTrustCertificate,
            TopologyMappingCode::ParticipantPermission => ParticipantPermission,
            TopologyMappingCode::PartyHostingLimits => PartyHostingLimits,
            TopologyMappingCode::VettedPackages => VettedPackages,
            TopologyMappingCode::PartyToParticipant => PartyToParticipant,
            TopologyMappingCode::SynchronizerParametersState => SynchronizerParametersState,
            TopologyMappingCode::MediatorSynchronizerState => MediatorSynchronizerState,
            TopologyMappingCode::SequencerSynchronizerState => SequencerSynchronizerState,
            TopologyMappingCode::SequencingDynamicParametersState => {
                SequencingDynamicParametersState
            }
            TopologyMappingCode::PartyToKeyMapping => PartyToKeyMapping,
            TopologyMappingCode::LsuAnnouncement => LsuAnnouncement,
            TopologyMappingCode::SequencerConnectionSuccessor => SequencerConnectionSuccessor,
        }
    }
}

impl From<TopologyMappingCode> for i32 {
    fn from(value: TopologyMappingCode) -> Self {
        proto::enums::TopologyMappingCode::from(value).into()
    }
}

impl TryFrom<proto::enums::TopologyMappingCode> for TopologyMappingCode {
    type Error = UnspecifiedEnumValue;

    fn try_from(value: proto::enums::TopologyMappingCode) -> Result<Self, Self::Error> {
        use TopologyMappingCode::*;

        match value {
            proto::enums::TopologyMappingCode::Unspecified => Err(UnspecifiedEnumValue::new()),
            proto::enums::TopologyMappingCode::NamespaceDelegation => Ok(NamespaceDelegation),
            proto::enums::TopologyMappingCode::DecentralizedNamespaceDefinition => {
                Ok(DecentralizedNamespaceDefinition)
            }
            proto::enums::TopologyMappingCode::OwnerToKeyMapping => Ok(OwnerToKeyMapping),
            proto::enums::TopologyMappingCode::SynchronizerTrustCertificate => {
                Ok(SynchronizerTrustCertificate)
            }
            proto::enums::TopologyMappingCode::ParticipantPermission => Ok(ParticipantPermission),
            proto::enums::TopologyMappingCode::PartyHostingLimits => Ok(PartyHostingLimits),
            proto::enums::TopologyMappingCode::VettedPackages => Ok(VettedPackages),
            proto::enums::TopologyMappingCode::PartyToParticipant => Ok(PartyToParticipant),
            proto::enums::TopologyMappingCode::SynchronizerParametersState => {
                Ok(SynchronizerParametersState)
            }
            proto::enums::TopologyMappingCode::MediatorSynchronizerState => {
                Ok(MediatorSynchronizerState)
            }
            proto::enums::TopologyMappingCode::SequencerSynchronizerState => {
                Ok(SequencerSynchronizerState)
            }
            proto::enums::TopologyMappingCode::SequencingDynamicParametersState => {
                Ok(SequencingDynamicParametersState)
            }
            proto::enums::TopologyMappingCode::PartyToKeyMapping => Ok(PartyToKeyMapping),
            proto::enums::TopologyMappingCode::LsuAnnouncement => Ok(LsuAnnouncement),
            proto::enums::TopologyMappingCode::SequencerConnectionSuccessor => {
                Ok(SequencerConnectionSuccessor)
            }
        }
    }
}

impl TryFrom<i32> for TopologyMappingCode {
    type Error = ConvertError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        proto::enums::TopologyMappingCode::try_from(value)?
            .try_into()
            .map_err(Into::into)
    }
}
