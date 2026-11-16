use admin_api_types::topology::v30::{ForceFlag, StoreId};
use canton_base::retry::{RetryConfig, RetryHandler};
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as proto,
    topology_manager_write_service_client::{self as svc_proto},
};
use canton_protocol::{
    SignedTopologyTransaction, TopologyChangeOp,
    error::ProtoEncodeError,
    mappings::{TopologyMapping, TopologyMappingVariant},
};
use canton_types::{NonZeroU31, topology::Fingerprint};
use protobuf_utils::RequiredProtoField as _;

use crate::v30::{client::InterceptedService, error::AdminApiError};

/// Wrapper for [`TopologyManagerWriteServiceClient`][svc_proto::TopologyManagerWriteServiceClient].
///
/// Note: Topology writes currently don't have any anti-duplication mechanism. That means that it's
/// probably a good idea (at least now) to disable retry for this client completely, because there
/// is no stable way of detecting duplicates. Instead user should handle retries manually.
#[derive(Clone, Debug)]
pub struct TopologyManagerWriteClient {
    service: svc_proto::TopologyManagerWriteServiceClient<InterceptedService>,
    retry_handler: RetryHandler,
}

impl TopologyManagerWriteClient {
    /// Create a wrapper from underlying tonic service client
    pub fn new(
        service: svc_proto::TopologyManagerWriteServiceClient<InterceptedService>,
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

    /// Send [`AuthorizeRequest`] which proposes mapping (convenient interface).
    pub async fn propose_mapping<T: TopologyMapping>(
        &self,
        must_fully_authorize: bool,
        store: Option<StoreId>,
        proposal: Proposal<T>,
    ) -> Result<SignedTopologyTransaction, AdminApiError> {
        let request = AuthorizeRequest {
            must_fully_authorize,
            force_changes: Vec::new(),
            signed_by: Vec::new(),
            store,
            wait_to_become_effective: None,
            type_: Type::from_proposal(proposal),
        };

        self.authorize(request).await
    }

    /// Send [`AuthorizeRequest`].
    pub async fn authorize(
        &self,
        request: AuthorizeRequest,
    ) -> Result<SignedTopologyTransaction, AdminApiError> {
        let request =
            proto::AuthorizeRequest::try_from(request).map_err(AdminApiError::input_error)?;

        self.retry_handler
            .call(&self.service, &request, |mut svc, req| async move {
                svc.authorize(req).await.map_err(AdminApiError::from_status)
            })
            .await?
            .transaction
            .required_of::<proto::AuthorizeResponse>("transaction")
            .map_err(AdminApiError::value_error)
            .and_then(|proto| {
                SignedTopologyTransaction::from_proto_v30(proto).map_err(AdminApiError::value_error)
            })
    }
}

/// Interface type for specifying the version of the schema used to represent the mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Mapping<T: TopologyMapping> {
    /// Serialized into v30 schema
    V30(T),

    /// Serialized into v31 schema
    V31(T),
}

impl<T: TopologyMapping> Mapping<T> {
    /// Replace underlying mapping with the variant type
    pub fn erase(self) -> Mapping<TopologyMappingVariant> {
        match self {
            Mapping::V30(mapping) => Mapping::V30(mapping.into_variant()),
            Mapping::V31(mapping) => Mapping::V31(mapping.into_variant()),
        }
    }
}

impl<T: TopologyMapping> TryFrom<Mapping<T>> for proto::authorize_request::proposal::Mapping {
    type Error = ProtoEncodeError;

    fn try_from(
        value: Mapping<T>,
    ) -> Result<proto::authorize_request::proposal::Mapping, ProtoEncodeError> {
        match value {
            Mapping::V30(mapping) => mapping.into_proto_mapping_v30().map(Self::V30),
            Mapping::V31(mapping) => mapping.into_proto_mapping_v31().map(Self::V31),
        }
    }
}

/// Mapping proposal
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Proposal<T: TopologyMapping> {
    /// Change operation kind
    pub change: TopologyChangeOp,

    /// Serial number of this request (auto-determined if omitted)
    pub serial: Option<NonZeroU31>,

    /// The mapping to be authorized
    pub mapping: Mapping<T>,
}

impl<T: TopologyMapping> Proposal<T> {
    /// Create a proposal to add/replace topology mapping (use schema v30)
    pub fn add_replace_v30(mapping: T) -> Self {
        Self {
            change: TopologyChangeOp::Replace,
            serial: None,
            mapping: Mapping::V30(mapping),
        }
    }

    /// Create a proposal to add/replace topology mapping (use schema v31)
    pub fn add_replace_v31(mapping: T) -> Self {
        Self {
            change: TopologyChangeOp::Replace,
            serial: None,
            mapping: Mapping::V31(mapping),
        }
    }

    /// Create a proposal to remove topology mapping (use schema v30)
    pub fn remove_v30(mapping: T) -> Self {
        Self {
            change: TopologyChangeOp::Remove,
            serial: None,
            mapping: Mapping::V30(mapping),
        }
    }

    /// Create a proposal to remove topology mapping (use schema v31)
    pub fn remove_v31(mapping: T) -> Self {
        Self {
            change: TopologyChangeOp::Remove,
            serial: None,
            mapping: Mapping::V31(mapping),
        }
    }

    /// Set specific serial value
    pub fn with_serial(mut self, serial: NonZeroU31) -> Self {
        self.serial = Some(serial);
        self
    }
}

impl<T: TopologyMapping> TryFrom<Proposal<T>> for proto::authorize_request::Proposal {
    type Error = ProtoEncodeError;

    fn try_from(value: Proposal<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            change: value.change.into_proto_v30_primitive(),
            serial: value.serial.map(Into::into).unwrap_or_default(),
            mapping: Some(value.mapping.try_into()?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Type {
    // We use "erased" type here
    Proposal(Proposal<TopologyMappingVariant>),
    TransactionHash(String), // TODO: is a better type needed here?
}

impl Type {
    pub fn from_proposal<T: TopologyMapping>(proposal: Proposal<T>) -> Self {
        Self::Proposal(Proposal {
            change: proposal.change,
            serial: proposal.serial,
            mapping: proposal.mapping.erase(),
        })
    }
}

impl TryFrom<Type> for proto::authorize_request::Type {
    type Error = ProtoEncodeError;

    fn try_from(value: Type) -> Result<Self, Self::Error> {
        match value {
            Type::Proposal(proposal) => proposal
                .try_into()
                .map(proto::authorize_request::Type::Proposal),
            Type::TransactionHash(txhash) => {
                Ok(proto::authorize_request::Type::TransactionHash(txhash))
            }
        }
    }
}

/// Authorize request
#[derive(Clone, Debug)]
pub struct AuthorizeRequest {
    /// If `true`: the transaction is only signed if the new signatures will result in the
    /// transaction being fully authorized. Otherwise returns as an error.
    ///
    /// If `false`: the transaction is signed and the signature distributed. The transaction may
    /// still not be fully authorized and remain as a proposal.
    pub must_fully_authorize: bool,

    /// Force specific changes even if dangerous
    pub force_changes: Vec<ForceFlag>,

    /// Fingerprint of the keys signing the authorization
    ///
    /// The signing key is used to identify a particular
    /// [`NamespaceDelegation`][canton_protocol::mappings::NamespaceDelegation] certificate, which
    /// is used to justify the given authorization.
    ///
    /// If empty, suitable signing keys available known to the node are automatically
    /// selected.
    pub signed_by: Vec<Fingerprint>,

    /// The store that is used as the underlying source for executing this request.
    ///
    /// If `store` is a synchronizer store, the resulting topology transaction will only be
    /// available on the respective synchronizer. If `store` is the authorized store, the resulting
    /// topology transaction may or may not be synchronized automatically to all synchronizers that
    /// the node is currently connected to or will be connected to in the future.
    ///
    /// Selecting a specific synchronizers store might be necessary, if the transaction to authorize
    /// by hash or the previous generation of the submitted proposal is only available on the
    /// synchronizers store and not in the authorized store.
    pub store: Option<StoreId>,

    /// Timeout to wait for the transaction to become effective in the store.
    pub wait_to_become_effective: Option<chrono::Duration>,

    /// Request type.
    pub type_: Type,
}

impl TryFrom<AuthorizeRequest> for proto::AuthorizeRequest {
    type Error = ProtoEncodeError;

    fn try_from(value: AuthorizeRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            must_fully_authorize: value.must_fully_authorize,
            force_changes: value.force_changes.into_iter().map(Into::into).collect(),
            signed_by: value.signed_by.into_iter().map(Into::into).collect(),
            store: value.store.map(Into::into),
            wait_to_become_effective: value.wait_to_become_effective.map(Into::into),
            r#type: Some(value.type_.try_into()?),
        })
    }
}
