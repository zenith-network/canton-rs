//! Topology mappings

// TODO: implement all remaining topology mappings:
//  SynchronizerTrustCertificate
//  ParticipantPermission
//  SynchronizerParametersState
//  MediatorSynchronizerState
//  SequencerSynchronizerState
//  SequencingDynamicParametersState
//  PartyToKeyMapping
//  SynchronizerUpgradeAnnouncement
//  SequencerConnectionSuccessor

use canton_proto::com::digitalasset::canton::protocol::{v30, v31};
use protobuf_utils::RequiredProtoField as _;

mod decentralized_namespace_definition;
mod namespace_delegation;
mod owner_to_key_mapping;
mod party_to_participant;
mod vetted_packages;

pub use decentralized_namespace_definition::{DecentralizedNamespaceDefinition, DndError};
pub use namespace_delegation::{
    CanSignAllButNamespaceDelegations, CanSignAllMappings, CanSignSpecificMappings,
    NamespaceDelegation, NsdError, Restriction,
};
pub use owner_to_key_mapping::{OtkError, OwnerToKeyMapping};
pub use party_to_participant::{HostingParticipant, HostingState, PartyToParticipant, PtpError};
pub use vetted_packages::{VettedPackage, VettedPackageError, VettedPackages};

use crate::{
    TopologyMappingCode,
    error::{ProtoDecodeError, ProtoEncodeError, UnexpectedTopologyMapping},
};

/// Types which defined topology mappings
pub trait TopologyMapping: Sized {
    /// Protobuf message type in schema v30
    type ProtoV30;

    /// Protobuf message type in schema v31
    type ProtoV31;

    /// Get the code of the mapping
    fn code(&self) -> TopologyMappingCode;

    /// Convert into variant type
    fn into_variant(self) -> TopologyMappingVariant;

    /// Convert from variant type
    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping>;

    /// Convert into message of schema v30
    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError>;

    /// Convert from message of schema v30
    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError>;

    /// Convert into message of schema v31
    fn into_proto_v31(self) -> Result<Self::ProtoV31, ProtoEncodeError>;

    /// Convert from message of schema v31
    fn from_proto_v31(proto: Self::ProtoV31) -> Result<Self, ProtoDecodeError>;

    /// Convert into topology mapping message of schema v30
    fn into_proto_mapping_v30(self) -> Result<v30::TopologyMapping, ProtoEncodeError> {
        self.into_variant().into_proto_v30()
    }

    /// Convert from topology mapping message of schema v30
    fn from_proto_mapping_v30(proto: v30::TopologyMapping) -> Result<Self, ProtoDecodeError> {
        let variant = TopologyMappingVariant::from_proto_v30(proto)?;
        Self::from_variant(variant).map_err(ProtoDecodeError::other)
    }

    /// Convert into topology mapping message of schema v31
    fn into_proto_mapping_v31(self) -> Result<v31::TopologyMapping, ProtoEncodeError> {
        self.into_variant().into_proto_v31()
    }

    /// Convert from topology mapping message of schema v31
    fn from_proto_mapping_v31(proto: v31::TopologyMapping) -> Result<Self, ProtoDecodeError> {
        let variant = TopologyMappingVariant::from_proto_v31(proto)?;
        Self::from_variant(variant).map_err(ProtoDecodeError::other)
    }
}

/// All topology mapping variants
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TopologyMappingVariant {
    NamespaceDelegation(NamespaceDelegation),
    DecentralizedNamespaceDefinition(DecentralizedNamespaceDefinition),
    OwnerToKeyMapping(OwnerToKeyMapping),
    VettedPackages(VettedPackages),
    PartyToParticipant(PartyToParticipant),
}

impl TopologyMapping for TopologyMappingVariant {
    type ProtoV30 = v30::TopologyMapping;
    type ProtoV31 = v31::TopologyMapping;

    fn code(&self) -> TopologyMappingCode {
        match self {
            TopologyMappingVariant::NamespaceDelegation(_) => {
                TopologyMappingCode::NamespaceDelegation
            }
            TopologyMappingVariant::DecentralizedNamespaceDefinition(_) => {
                TopologyMappingCode::DecentralizedNamespaceDefinition
            }
            TopologyMappingVariant::OwnerToKeyMapping(_) => TopologyMappingCode::OwnerToKeyMapping,
            TopologyMappingVariant::VettedPackages(_) => TopologyMappingCode::VettedPackages,
            TopologyMappingVariant::PartyToParticipant(_) => {
                TopologyMappingCode::PartyToParticipant
            }
        }
    }

    fn into_variant(self) -> TopologyMappingVariant {
        self
    }

    fn from_variant(variant: TopologyMappingVariant) -> Result<Self, UnexpectedTopologyMapping> {
        Ok(variant)
    }

    fn into_proto_v30(self) -> Result<Self::ProtoV30, ProtoEncodeError> {
        use v30::topology_mapping::Mapping;

        let mapping = match self {
            Self::NamespaceDelegation(nsd) => Mapping::NamespaceDelegation(nsd.into_proto_v30()?),
            Self::DecentralizedNamespaceDefinition(dnd) => {
                Mapping::DecentralizedNamespaceDefinition(dnd.into_proto_v30()?)
            }
            Self::OwnerToKeyMapping(otk) => Mapping::OwnerToKeyMapping(otk.into_proto_v30()?),
            Self::VettedPackages(vetted) => Mapping::VettedPackages(vetted.into_proto_v30()?),
            Self::PartyToParticipant(ptp) => Mapping::PartyToParticipant(ptp.into_proto_v30()?),
        };

        Ok(v30::TopologyMapping {
            mapping: Some(mapping),
        })
    }

    fn from_proto_v30(proto: Self::ProtoV30) -> Result<Self, ProtoDecodeError> {
        use v30::topology_mapping::Mapping;

        let mapping = proto
            .mapping
            .required_of::<v30::TopologyMapping>("mapping")?;

        match mapping {
            Mapping::NamespaceDelegation(mapping) => {
                NamespaceDelegation::from_proto_v30(mapping).map(Self::NamespaceDelegation)
            }
            Mapping::DecentralizedNamespaceDefinition(mapping) => {
                DecentralizedNamespaceDefinition::from_proto_v30(mapping)
                    .map(Self::DecentralizedNamespaceDefinition)
            }
            Mapping::OwnerToKeyMapping(mapping) => {
                OwnerToKeyMapping::from_proto_v30(mapping).map(Self::OwnerToKeyMapping)
            }
            Mapping::VettedPackages(mapping) => {
                VettedPackages::from_proto_v30(mapping).map(Self::VettedPackages)
            }
            Mapping::PartyToParticipant(mapping) => {
                PartyToParticipant::from_proto_v30(mapping).map(Self::PartyToParticipant)
            }
            t => todo!("mapping {t:?} is not supported yet"),
        }
    }

    fn into_proto_v31(self) -> Result<Self::ProtoV31, ProtoEncodeError> {
        use v31::topology_mapping::Mapping;

        let mapping = match self {
            Self::NamespaceDelegation(nsd) => Mapping::NamespaceDelegation(nsd.into_proto_v30()?),
            Self::DecentralizedNamespaceDefinition(dnd) => {
                Mapping::DecentralizedNamespaceDefinition(dnd.into_proto_v30()?)
            }
            Self::OwnerToKeyMapping(otk) => Mapping::OwnerToKeyMapping(otk.into_proto_v30()?),
            Self::VettedPackages(vetted) => Mapping::VettedPackages(vetted.into_proto_v30()?),
            Self::PartyToParticipant(ptp) => Mapping::PartyToParticipant(ptp.into_proto_v31()?),
        };

        Ok(v31::TopologyMapping {
            mapping: Some(mapping),
        })
    }

    fn from_proto_v31(proto: Self::ProtoV31) -> Result<Self, ProtoDecodeError> {
        use v31::topology_mapping::Mapping;

        let mapping = proto
            .mapping
            .required_of::<v31::TopologyMapping>("mapping")?;

        match mapping {
            Mapping::NamespaceDelegation(mapping) => {
                NamespaceDelegation::from_proto_v30(mapping).map(Self::NamespaceDelegation)
            }
            Mapping::DecentralizedNamespaceDefinition(mapping) => {
                DecentralizedNamespaceDefinition::from_proto_v30(mapping)
                    .map(Self::DecentralizedNamespaceDefinition)
            }
            Mapping::OwnerToKeyMapping(mapping) => {
                OwnerToKeyMapping::from_proto_v30(mapping).map(Self::OwnerToKeyMapping)
            }
            Mapping::VettedPackages(mapping) => {
                VettedPackages::from_proto_v30(mapping).map(Self::VettedPackages)
            }
            Mapping::PartyToParticipant(mapping) => {
                PartyToParticipant::from_proto_v31(mapping).map(Self::PartyToParticipant)
            }
            t => todo!("mapping {t:?} is not supported yet"),
        }
    }

    fn into_proto_mapping_v30(self) -> Result<v30::TopologyMapping, ProtoEncodeError> {
        self.into_proto_v30()
    }

    fn from_proto_mapping_v30(proto: v30::TopologyMapping) -> Result<Self, ProtoDecodeError> {
        Self::from_proto_v30(proto)
    }

    fn into_proto_mapping_v31(self) -> Result<v31::TopologyMapping, ProtoEncodeError> {
        self.into_proto_v31()
    }

    fn from_proto_mapping_v31(proto: v31::TopologyMapping) -> Result<Self, ProtoDecodeError> {
        Self::from_proto_v31(proto)
    }
}

impl From<NamespaceDelegation> for TopologyMappingVariant {
    fn from(value: NamespaceDelegation) -> Self {
        Self::NamespaceDelegation(value)
    }
}

impl From<DecentralizedNamespaceDefinition> for TopologyMappingVariant {
    fn from(value: DecentralizedNamespaceDefinition) -> Self {
        Self::DecentralizedNamespaceDefinition(value)
    }
}

impl From<OwnerToKeyMapping> for TopologyMappingVariant {
    fn from(value: OwnerToKeyMapping) -> Self {
        Self::OwnerToKeyMapping(value)
    }
}

impl From<VettedPackages> for TopologyMappingVariant {
    fn from(value: VettedPackages) -> Self {
        Self::VettedPackages(value)
    }
}

impl From<PartyToParticipant> for TopologyMappingVariant {
    fn from(value: PartyToParticipant) -> Self {
        Self::PartyToParticipant(value)
    }
}
