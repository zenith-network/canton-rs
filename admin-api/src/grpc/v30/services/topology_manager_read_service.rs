use std::time::SystemTime;

use admin_api_types::{
    protocol::v30::{
        TopologyChangeOp,
        mappings::{
            DecentralizedNamespaceDefinition, NamespaceDelegation, OwnerToKeyMapping,
            PartyToParticipant, VettedPackages,
        },
    },
    topology::admin::v30::StoreId,
};
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as proto,
    topology_manager_read_service_client::{self as svc_proto},
};
use canton_types::topology::{Fingerprint, MemberCode, ProtocolVersion};

use crate::grpc::v30::client::InterceptedService;

/// Wrapper for [`TopologyManagerReadServiceClient`][svc_proto::TopologyManagerReadServiceClient].
#[derive(Clone, Debug)]
pub struct TopologyManagerReadClient {
    service: svc_proto::TopologyManagerReadServiceClient<InterceptedService>,
}

impl TopologyManagerReadClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(service: svc_proto::TopologyManagerReadServiceClient<InterceptedService>) -> Self {
        Self { service }
    }

    /// List namespace delegations
    ///
    /// ## Arguments
    ///
    /// - `filter_namespace` - Match namespaces starting with the given prefix. Set to `None` for
    ///   no namespace filter.
    /// - `filter_target_key_fingerprint` - Match the target key's fingerprint exactly. Set to
    ///   `None` for no target key filter.
    pub async fn list_namespace_delegation(
        &mut self,
        base_query: BaseQuery,
        filter_namespace: Option<String>,
        filter_target_key_fingerprint: Option<Fingerprint>,
    ) -> Result<Vec<TopologyResult<NamespaceDelegation>>, ()> {
        let request = proto::ListNamespaceDelegationRequest {
            base_query: Some(base_query.into()),
            filter_namespace: filter_namespace.unwrap_or_default(),
            filter_target_key_fingerprint: filter_target_key_fingerprint
                .map(Into::into)
                .unwrap_or_default(),
        };

        self.service
            .list_namespace_delegation(request)
            .await
            .unwrap()
            .into_inner()
            .results
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// List decentralized namespace definitions
    pub async fn list_decentralized_namespace_definition(
        &mut self,
        base_query: BaseQuery,
        filter_namespace: Option<String>,
    ) -> Result<Vec<TopologyResult<DecentralizedNamespaceDefinition>>, ()> {
        let request = proto::ListDecentralizedNamespaceDefinitionRequest {
            base_query: Some(base_query.into()),
            filter_namespace: filter_namespace.unwrap_or_default(),
        };

        self.service
            .list_decentralized_namespace_definition(request)
            .await
            .unwrap()
            .into_inner()
            .results
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// List owner-to-key mappings
    pub async fn list_owner_to_key_mapping(
        &mut self,
        base_query: BaseQuery,
        filter_key_owner_type: Option<MemberCode>,
        filter_key_owner_uid: Option<String>,
    ) -> Result<Vec<TopologyResult<OwnerToKeyMapping>>, ()> {
        let request = proto::ListOwnerToKeyMappingRequest {
            base_query: Some(base_query.into()),
            filter_key_owner_type: filter_key_owner_type
                .map(|code| code.as_str().to_owned())
                .unwrap_or_default(),
            filter_key_owner_uid: filter_key_owner_uid.unwrap_or_default(),
        };

        self.service
            .list_owner_to_key_mapping(request)
            .await
            .unwrap()
            .into_inner()
            .results
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// List vetted packages
    pub async fn list_vetted_packages(
        &mut self,
        base_query: BaseQuery,
        filter_participant: Option<String>,
    ) -> Result<Vec<TopologyResult<VettedPackages>>, ()> {
        let request = proto::ListVettedPackagesRequest {
            base_query: Some(base_query.into()),
            filter_participant: filter_participant.unwrap_or_default(),
        };

        self.service
            .list_vetted_packages(request)
            .await
            .unwrap()
            .into_inner()
            .results
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// List party-to-participant mappings
    pub async fn list_party_to_participant(
        &mut self,
        base_query: BaseQuery,
        filter_party: Option<String>,
        filter_participant: Option<String>,
    ) -> Result<Vec<TopologyResult<PartyToParticipant>>, ()> {
        let request = proto::ListPartyToParticipantRequest {
            base_query: Some(base_query.into()),
            filter_party: filter_party.unwrap_or_default(),
            filter_participant: filter_participant.unwrap_or_default(),
        };

        self.service
            .list_party_to_participant(request)
            .await
            .unwrap()
            .into_inner()
            .results
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    // TODO: implement missing methods of this service
}

/// Common query for topology mappings
#[derive(Clone, Debug, Default)]
pub struct BaseQuery {
    /// Store to query
    pub store: Option<StoreId>,

    /// Whether to query only for proposals instead of approved topology mappings
    pub proposals: bool,

    /// Topology change operation to query
    pub operation: Option<TopologyChangeOp>,

    /// Time query for topology mappings
    pub time_query: TimeQuery,

    /// Signing key filter
    pub filter_signed_key: Option<String>,

    /// Protocol version of the returned topology mappings
    pub protocol_version: Option<ProtocolVersion>,

    /// The Canton version of the client issuing the request, e.g. `"3.5.0-rc3"`
    pub client_version: Option<String>,
}

impl From<BaseQuery> for proto::BaseQuery {
    fn from(value: BaseQuery) -> Self {
        Self {
            store: value.store.map(Into::into),
            proposals: value.proposals,
            operation: value.operation.map(Into::into).unwrap_or_default(),
            time_query: Some(value.time_query.into()),
            filter_signed_key: value.filter_signed_key.unwrap_or_default(),
            protocol_version: value.protocol_version.map(Into::into),
            client_version: value.client_version,
        }
    }
}

/// Time query for topology mappings
#[derive(Clone, Debug, Default)]
pub enum TimeQuery {
    /// Query the topology state at the given timestamp
    Snapshot(SystemTime),

    /// Query the head topology state
    #[default]
    HeadState,

    /// Query topology transactions in a time range
    Range {
        /// Start of the time range
        from: Option<SystemTime>,

        /// End of the time range
        until: Option<SystemTime>,
    },
}

impl From<TimeQuery> for proto::base_query::TimeQuery {
    fn from(value: TimeQuery) -> Self {
        match value {
            TimeQuery::Snapshot(timestamp) => Self::Snapshot(timestamp.into()),
            TimeQuery::HeadState => Self::HeadState(()),
            TimeQuery::Range { from, until } => Self::Range(proto::base_query::TimeRange {
                from: from.map(Into::into),
                until: until.map(Into::into),
            }),
        }
    }
}

/// Context of a topology mapping result
#[derive(Clone, Debug)]
pub struct BaseResult {
    /// Store containing the topology mapping
    pub store: StoreId,

    /// Timestamp at which the topology transaction was sequenced
    pub sequenced: SystemTime,

    /// Timestamp from which the topology mapping is valid
    pub valid_from: SystemTime,

    /// Timestamp until which the topology mapping is valid
    pub valid_until: Option<SystemTime>,

    /// Topology change operation
    pub operation: TopologyChangeOp,

    /// Hash of the topology transaction
    pub transaction_hash: Vec<u8>,

    /// Serial number of the topology transaction
    pub serial: u32,

    /// Fingerprints of the keys signing the topology transaction
    pub signed_by_fingerprints: Vec<Fingerprint>,
}

impl TryFrom<proto::BaseResult> for BaseResult {
    type Error = ();

    fn try_from(value: proto::BaseResult) -> Result<Self, Self::Error> {
        todo!()
    }
}

/// Topology mapping and its context
#[derive(Clone, Debug)]
pub struct TopologyResult<T> {
    /// Context of the topology mapping
    pub context: BaseResult,

    /// Topology mapping
    pub item: T,
}

impl TryFrom<proto::list_namespace_delegation_response::Result>
    for TopologyResult<NamespaceDelegation>
{
    type Error = ();

    fn try_from(
        value: proto::list_namespace_delegation_response::Result,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<proto::list_decentralized_namespace_definition_response::Result>
    for TopologyResult<DecentralizedNamespaceDefinition>
{
    type Error = ();

    fn try_from(
        value: proto::list_decentralized_namespace_definition_response::Result,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<proto::list_owner_to_key_mapping_response::Result>
    for TopologyResult<OwnerToKeyMapping>
{
    type Error = ();

    fn try_from(
        value: proto::list_owner_to_key_mapping_response::Result,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<proto::list_vetted_packages_response::Result> for TopologyResult<VettedPackages> {
    type Error = ();

    fn try_from(value: proto::list_vetted_packages_response::Result) -> Result<Self, Self::Error> {
        todo!()
    }
}

impl TryFrom<proto::list_party_to_participant_response::Result>
    for TopologyResult<PartyToParticipant>
{
    type Error = ();

    fn try_from(
        value: proto::list_party_to_participant_response::Result,
    ) -> Result<Self, Self::Error> {
        todo!()
    }
}
