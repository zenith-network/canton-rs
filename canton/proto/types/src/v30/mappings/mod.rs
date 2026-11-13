//! Topology mappings

use canton_proto::com::digitalasset::canton::protocol::v30 as proto;
use protobuf_utils::RequiredProtoField as _;

mod decentralized_namespace_definition;
mod namespace_delegation;
mod owner_to_key_mapping;
mod party_to_participant;
mod topology_mapping_code;
mod vetted_packages;

pub use decentralized_namespace_definition::DecentralizedNamespaceDefinition;
pub use namespace_delegation::{NamespaceDelegation, Restriction};
pub use owner_to_key_mapping::OwnerToKeyMapping;
pub use party_to_participant::{HostingParticipant, ParticipantPermission, PartyToParticipant};
pub use topology_mapping_code::TopologyMappingCode;
pub use vetted_packages::{VettedPackage, VettedPackages};

use crate::error::ConvertError;

/// Topology mapping
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TopologyMapping {
    NamespaceDelegation(NamespaceDelegation),
    DecentralizedNamespaceDefinition(DecentralizedNamespaceDefinition),
    OwnerToKeyMapping(OwnerToKeyMapping),
    VettedPackages(VettedPackages),
    PartyToParticipant(PartyToParticipant),
    // TODO: implement all remaining variants:
    //  SynchronizerTrustCertificate
    //  ParticipantPermission
    //  SynchronizerParametersState
    //  MediatorSynchronizerState
    //  SequencerSynchronizerState
    //  SequencingDynamicParametersState
    //  PartyToKeyMapping
    //  SynchronizerUpgradeAnnouncement
    //  SequencerConnectionSuccessor
}

impl TopologyMapping {
    /// Get the code of the mapping
    pub fn code(&self) -> TopologyMappingCode {
        match self {
            TopologyMapping::NamespaceDelegation(_) => TopologyMappingCode::NamespaceDelegation,
            TopologyMapping::DecentralizedNamespaceDefinition(_) => {
                TopologyMappingCode::DecentralizedNamespaceDefinition
            }
            TopologyMapping::OwnerToKeyMapping(_) => TopologyMappingCode::OwnerToKeyMapping,
            TopologyMapping::VettedPackages(_) => TopologyMappingCode::VettedPackages,
            TopologyMapping::PartyToParticipant(_) => TopologyMappingCode::PartyToParticipant,
        }
    }
}

impl From<TopologyMapping> for proto::TopologyMapping {
    fn from(value: TopologyMapping) -> Self {
        use proto::topology_mapping::Mapping::*;
        Self {
            mapping: Some(match value {
                TopologyMapping::NamespaceDelegation(nsd) => NamespaceDelegation(nsd.into()),
                TopologyMapping::DecentralizedNamespaceDefinition(dnd) => {
                    DecentralizedNamespaceDefinition(dnd.into())
                }
                TopologyMapping::OwnerToKeyMapping(otk) => OwnerToKeyMapping(otk.into()),
                TopologyMapping::VettedPackages(vetted) => VettedPackages(vetted.into()),
                TopologyMapping::PartyToParticipant(ptp) => PartyToParticipant(ptp.into()),
            }),
        }
    }
}

impl TryFrom<proto::TopologyMapping> for TopologyMapping {
    type Error = ConvertError;

    fn try_from(value: proto::TopologyMapping) -> Result<Self, Self::Error> {
        use proto::topology_mapping::Mapping::*;

        let mapping = value
            .mapping
            .required_of::<proto::TopologyMapping>("mapping")?;

        match mapping {
            NamespaceDelegation(mapping) => mapping.try_into().map(Self::NamespaceDelegation),
            DecentralizedNamespaceDefinition(mapping) => mapping
                .try_into()
                .map(Self::DecentralizedNamespaceDefinition),
            OwnerToKeyMapping(mapping) => mapping.try_into().map(Self::OwnerToKeyMapping),
            VettedPackages(mapping) => mapping.try_into().map(Self::VettedPackages),
            PartyToParticipant(mapping) => mapping.try_into().map(Self::PartyToParticipant),
            t => todo!("mapping {t:?} is not supported yet"),
        }
    }
}
