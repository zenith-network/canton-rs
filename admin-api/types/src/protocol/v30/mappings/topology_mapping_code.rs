use canton_proto::com::digitalasset::canton::protocol::v30 as proto;

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
        todo!()
    }
}

impl From<TopologyMappingCode> for i32 {
    fn from(value: TopologyMappingCode) -> Self {
        proto::enums::TopologyMappingCode::from(value).into()
    }
}

impl TryFrom<proto::enums::TopologyMappingCode> for TopologyMappingCode {
    type Error = ();

    fn try_from(value: proto::enums::TopologyMappingCode) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<i32> for TopologyMappingCode {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        todo!()
    }
}
