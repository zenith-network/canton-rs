use admin_api_types::{
    protocol::v30::{SignedTopologyTransaction, TopologyChangeOp, mappings::TopologyMapping},
    topology::admin::v30::{ForceFlag, StoreId},
};
use canton_proto::com::digitalasset::canton::topology::admin::v30::{
    self as proto,
    topology_manager_write_service_client::{self as svc_proto},
};
use canton_types::topology::Fingerprint;

use crate::grpc::v30::client::InterceptedService;

/// Wrapper for [`TopologyManagerWriteServiceClient`][svc_proto::TopologyManagerWriteServiceClient].
#[derive(Clone, Debug)]
pub struct TopologyManagerWriteClient {
    service: svc_proto::TopologyManagerWriteServiceClient<InterceptedService>,
}

impl TopologyManagerWriteClient {
    pub fn new(service: svc_proto::TopologyManagerWriteServiceClient<InterceptedService>) -> Self {
        Self { service }
    }

    /// Send [`AuthorizeRequest`] which proposes mapping.
    pub async fn propose_mapping(
        &mut self,
        must_fully_authorize: bool,
        store: Option<StoreId>,
        proposal: Proposal,
    ) -> Result<SignedTopologyTransaction, ()> {
        let request = AuthorizeRequest {
            must_fully_authorize,
            force_changes: Vec::new(),
            signed_by: Vec::new(),
            store: store.map(Into::into),
            wait_to_become_effective: None,
            type_: Type::Proposal(proposal),
        };

        self.authorize(request).await
    }

    pub async fn authorize(
        &mut self,
        request: AuthorizeRequest,
    ) -> Result<SignedTopologyTransaction, ()> {
        self.service
            .authorize(proto::AuthorizeRequest::from(request))
            .await
            .unwrap()
            .into_inner()
            .transaction
            .unwrap()
            .try_into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mapping {
    V30(TopologyMapping),
}

impl From<Mapping> for proto::authorize_request::proposal::Mapping {
    fn from(value: Mapping) -> Self {
        match value {
            Mapping::V30(mapping_v30) => Self::V30(mapping_v30.into()),
        }
    }
}

/// Mapping proposal
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proposal {
    /// Change operation kind
    pub change: TopologyChangeOp,

    /// Serial number of this request (auto-determined if omitted)
    pub serial: Option<u32>,

    /// The mapping to be authorized
    pub mapping: Mapping,
}

impl From<Proposal> for proto::authorize_request::Proposal {
    fn from(value: Proposal) -> Self {
        Self {
            change: value.change.into(),
            serial: value.serial.unwrap_or_default(),
            mapping: Some(value.mapping.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Proposal(Proposal),
    TransactionHash(String), // TODO: better type here
}

impl From<Type> for proto::authorize_request::Type {
    fn from(value: Type) -> Self {
        use proto::authorize_request::Type::*;
        match value {
            Type::Proposal(proposal) => Proposal(proposal.into()),
            Type::TransactionHash(txhash) => TransactionHash(txhash),
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
    /// [`NamespaceDelegation`][admin_api_types::protocol::v30::mappings::NamespaceDelegation]
    /// certificate, which is used to justify the given authorization.
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

    pub type_: Type,
}

impl From<AuthorizeRequest> for proto::AuthorizeRequest {
    fn from(value: AuthorizeRequest) -> Self {
        Self {
            must_fully_authorize: value.must_fully_authorize,
            force_changes: value.force_changes.into_iter().map(Into::into).collect(),
            signed_by: value.signed_by.into_iter().map(Into::into).collect(),
            store: value.store.map(Into::into),
            wait_to_become_effective: value.wait_to_become_effective.map(Into::into),
            r#type: Some(value.type_.into()),
        }
    }
}
