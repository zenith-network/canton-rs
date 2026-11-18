use std::time::SystemTime;

use admin_api_types::{error::ValueError, topology::v30::StoreId};
use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as proto,
    topology_manager_read_service_client::{self as svc_proto},
};
use canton_protocol::{
    TopologyChangeOp,
    mappings::{
        DecentralizedNamespaceDefinition, NamespaceDelegation, OwnerToKeyMapping,
        PartyToParticipant, TopologyMapping as _, VettedPackages,
    },
};
use canton_types::topology::{Fingerprint, MemberCode};
use protobuf_utils::{InvalidProtoField as _, RequiredProtoField as _};

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`TopologyManagerReadServiceClient`][svc_proto::TopologyManagerReadServiceClient].
#[derive(Clone, Debug)]
pub struct TopologyManagerReadClient {
    service: svc_proto::TopologyManagerReadServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl TopologyManagerReadClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::TopologyManagerReadServiceClient<InterceptedService>,
        retry_handler: RetryHandler,
    ) -> Self {
        Self {
            service,
            retry_handler,
        }
    }

    /// Set retry config for the client
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
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
        &self,
        base_query: BaseQuery,
        filter_namespace: Option<String>,
        filter_target_key_fingerprint: Option<Fingerprint>,
    ) -> Result<Vec<TopologyResult<NamespaceDelegation>>, AdminApiError> {
        let request = proto::ListNamespaceDelegationRequest {
            base_query: Some(base_query.into()),
            filter_namespace: filter_namespace.unwrap_or_default(),
            filter_target_key_fingerprint: filter_target_key_fingerprint
                .map(Into::into)
                .unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_namespace_delegation(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .results
            .into_iter()
            .map(|res| res.try_into().map_err(AdminApiError::value_error))
            .collect()
    }

    /// List decentralized namespace definitions
    pub async fn list_decentralized_namespace_definition(
        &self,
        base_query: BaseQuery,
        filter_namespace: Option<String>,
    ) -> Result<Vec<TopologyResult<DecentralizedNamespaceDefinition>>, AdminApiError> {
        let request = proto::ListDecentralizedNamespaceDefinitionRequest {
            base_query: Some(base_query.into()),
            filter_namespace: filter_namespace.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_decentralized_namespace_definition(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .results
            .into_iter()
            .map(|res| res.try_into().map_err(AdminApiError::value_error))
            .collect()
    }

    /// List owner-to-key mappings
    pub async fn list_owner_to_key_mapping(
        &self,
        base_query: BaseQuery,
        filter_key_owner_type: Option<MemberCode>,
        filter_key_owner_uid: Option<String>,
    ) -> Result<Vec<TopologyResult<OwnerToKeyMapping>>, AdminApiError> {
        let request = proto::ListOwnerToKeyMappingRequest {
            base_query: Some(base_query.into()),
            filter_key_owner_type: filter_key_owner_type
                .map(|code| code.as_str().to_owned())
                .unwrap_or_default(),
            filter_key_owner_uid: filter_key_owner_uid.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_owner_to_key_mapping(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .results
            .into_iter()
            .map(|res| res.try_into().map_err(AdminApiError::value_error))
            .collect()
    }

    /// List vetted packages
    pub async fn list_vetted_packages(
        &self,
        base_query: BaseQuery,
        filter_participant: Option<String>,
    ) -> Result<Vec<TopologyResult<VettedPackages>>, AdminApiError> {
        let request = proto::ListVettedPackagesRequest {
            base_query: Some(base_query.into()),
            filter_participant: filter_participant.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_vetted_packages(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .results
            .into_iter()
            .map(|res| res.try_into().map_err(AdminApiError::value_error))
            .collect()
    }

    /// List party-to-participant mappings
    pub async fn list_party_to_participant(
        &self,
        base_query: BaseQuery,
        filter_party: Option<String>,
        filter_participant: Option<String>,
    ) -> Result<Vec<TopologyResult<PartyToParticipant>>, AdminApiError> {
        let request = proto::ListPartyToParticipantRequest {
            base_query: Some(base_query.into()),
            filter_party: filter_party.unwrap_or_default(),
            filter_participant: filter_participant.unwrap_or_default(),
        };

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.list_party_to_participant(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?
            .results
            .into_iter()
            .map(|res| res.try_into().map_err(AdminApiError::value_error))
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
}

impl From<BaseQuery> for proto::BaseQuery {
    fn from(value: BaseQuery) -> Self {
        Self {
            store: value.store.map(Into::into),
            proposals: value.proposals,
            operation: value
                .operation
                .map(TopologyChangeOp::into_proto_v30_primitive)
                .unwrap_or_default(),
            time_query: Some(value.time_query.into()),
            filter_signed_key: value.filter_signed_key.unwrap_or_default(),
            // We do not set explicit protocol version. For our queries the participant will
            // use the version on his choice. The fact that results will be properly decoded is not
            // enforced by this version. Instead it's enforced by having appropriate messages
            // versions (v31, v32, ...) in bindings.
            protocol_version: None,
            // Currently it seems that participant doesn't use this value.
            client_version: None,
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
    type Error = ValueError;

    fn try_from(value: proto::BaseResult) -> Result<Self, Self::Error> {
        Ok(Self {
            store: value
                .store
                .required_of::<proto::BaseResult>("store")?
                .try_into()
                .validated_of::<proto::BaseResult>("store")?,
            sequenced: value
                .sequenced
                .required_of::<proto::BaseResult>("sequenced")?
                .try_into()
                .validated_of::<proto::BaseResult>("sequenced")?,
            valid_from: value
                .valid_from
                .required_of::<proto::BaseResult>("valid_from")?
                .try_into()
                .validated_of::<proto::BaseResult>("valid_from")?,
            valid_until: value
                .valid_until
                .map(TryInto::try_into)
                .transpose()
                .validated_of::<proto::BaseResult>("valid_until")?,
            operation: TopologyChangeOp::from_proto_v30_primitive(value.operation)
                .validated_of::<proto::BaseResult>("operation")?,
            transaction_hash: value.transaction_hash,
            serial: value
                .serial
                .try_into()
                .validated_of::<proto::BaseResult>("serial")?,
            signed_by_fingerprints: value
                .signed_by_fingerprints
                .into_iter()
                .map(Fingerprint::try_from)
                .collect::<Result<_, _>>()
                .validated_of::<proto::BaseResult>("signed_by_fingerprints")?,
        })
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
    type Error = ValueError;

    fn try_from(
        value: proto::list_namespace_delegation_response::Result,
    ) -> Result<Self, Self::Error> {
        use proto::list_namespace_delegation_response::{Result as Proto, result::Item};

        let context = value
            .context
            .required_of::<Proto>("context")?
            .try_into()
            .validated_of::<Proto>("context")?;
        let item = match value.item.required_of::<Proto>("item")? {
            Item::V30(item) => NamespaceDelegation::from_proto_v30(item),
        }
        .validated_of::<Proto>("item")?;

        Ok(Self { context, item })
    }
}

impl TryFrom<proto::list_decentralized_namespace_definition_response::Result>
    for TopologyResult<DecentralizedNamespaceDefinition>
{
    type Error = ValueError;

    fn try_from(
        value: proto::list_decentralized_namespace_definition_response::Result,
    ) -> Result<Self, Self::Error> {
        use proto::list_decentralized_namespace_definition_response::Result as Proto;

        let context = value
            .context
            .required_of::<Proto>("context")?
            .try_into()
            .validated_of::<Proto>("context")?;
        let item = DecentralizedNamespaceDefinition::from_proto_v30(
            value.item.required_of::<Proto>("item")?,
        )
        .validated_of::<Proto>("item")?;

        Ok(Self { context, item })
    }
}

impl TryFrom<proto::list_owner_to_key_mapping_response::Result>
    for TopologyResult<OwnerToKeyMapping>
{
    type Error = ValueError;

    fn try_from(
        value: proto::list_owner_to_key_mapping_response::Result,
    ) -> Result<Self, Self::Error> {
        use proto::list_owner_to_key_mapping_response::{Result as Proto, result::Item};

        let context = value
            .context
            .required_of::<Proto>("context")?
            .try_into()
            .validated_of::<Proto>("context")?;
        let item = match value.item.required_of::<Proto>("item")? {
            Item::V30(item) => OwnerToKeyMapping::from_proto_v30(item),
        }
        .validated_of::<Proto>("item")?;

        Ok(Self { context, item })
    }
}

impl TryFrom<proto::list_vetted_packages_response::Result> for TopologyResult<VettedPackages> {
    type Error = ValueError;

    fn try_from(value: proto::list_vetted_packages_response::Result) -> Result<Self, Self::Error> {
        use proto::list_vetted_packages_response::Result as Proto;

        let context = value
            .context
            .required_of::<Proto>("context")?
            .try_into()
            .validated_of::<Proto>("context")?;
        let item = VettedPackages::from_proto_v30(value.item.required_of::<Proto>("item")?)
            .validated_of::<Proto>("item")?;

        Ok(Self { context, item })
    }
}

impl TryFrom<proto::list_party_to_participant_response::Result>
    for TopologyResult<PartyToParticipant>
{
    type Error = ValueError;

    fn try_from(
        value: proto::list_party_to_participant_response::Result,
    ) -> Result<Self, Self::Error> {
        use proto::list_party_to_participant_response::{Result as Proto, result::Item};

        let context = value
            .context
            .required_of::<Proto>("context")?
            .try_into()
            .validated_of::<Proto>("context")?;
        let item = match value.item.required_of::<Proto>("item")? {
            Item::V30(item) => PartyToParticipant::from_proto_v30(item),
            Item::V31(item) => PartyToParticipant::from_proto_v31(item),
        }
        .validated_of::<Proto>("item")?;

        Ok(Self { context, item })
    }
}
