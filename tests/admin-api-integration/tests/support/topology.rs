//! Independent topology fixtures, connections, and transaction assertions.

use std::time::SystemTime;

use admin_api::v30::{
    error::AdminApiError,
    services::{TopologyManagerReadClient, TopologyManagerWriteClient, topology_read::BaseQuery},
};
use admin_api_proto::com::digitalasset::canton::topology::admin::v30 as common;
use admin_api_proto::prost_types;
use canton_base::{
    auth::interceptor::AuthInterceptor,
    error::{CantonError, RedactedCantonError},
    retry::RetryConfig,
};
use canton_proto::com::digitalasset::canton::{
    crypto::{admin::v30 as vault, v30 as crypto},
    protocol::v30 as mapping,
    topology::admin::v30 as p,
    version::v1::{UntypedVersionedMessage, untyped_versioned_message::Wrapper},
};
use canton_protocol::{SignedTopologyTransaction, crypto::v30::EncodedSigningPublicKey};
use canton_types::topology::{Namespace, ParticipantId};
use pretty_assertions::assert_eq;
use prost::Message;
use tonic::transport::{Channel, Endpoint};
use tonic_types::StatusExt;

use super::{Sandbox, ok};

pub type RawRead =
    p::topology_manager_read_service_client::TopologyManagerReadServiceClient<Channel>;
pub type RawWrite =
    p::topology_manager_write_service_client::TopologyManagerWriteServiceClient<Channel>;

/// SDK and protobuf clients on the same participant, without SDK retries.
pub struct Context {
    pub endpoint: String,
    pub channel: Channel,
    pub read: TopologyManagerReadClient,
    pub write: TopologyManagerWriteClient,
    pub participant: ParticipantId,
    pub root: mapping::NamespaceDelegation,
}

impl Context {
    pub async fn new(sandbox: &Sandbox) -> Self {
        let endpoint = sandbox.endpoints().admin_api.clone();
        let channel = ok(
            &endpoint,
            Endpoint::from_shared(endpoint.clone()).unwrap().connect(),
        )
        .await;
        let read = TopologyManagerReadClient::new(
            p::topology_manager_read_service_client::TopologyManagerReadServiceClient::with_interceptor(
                channel.clone(), AuthInterceptor::new(None),
            ),
            RetryConfig::no_retry().into_handler(),
        );
        let write = TopologyManagerWriteClient::new(
            p::topology_manager_write_service_client::TopologyManagerWriteServiceClient::with_interceptor(
                channel.clone(), AuthInterceptor::new(None),
            ),
            RetryConfig::no_retry().into_handler(),
        );
        let owners = ok(
            &endpoint,
            RawRead::new(channel.clone()).list_owner_to_key_mapping(
                p::ListOwnerToKeyMappingRequest {
                    base_query: Some(head(Some(authorized()))),
                    filter_key_owner_type: "PAR".into(),
                    ..Default::default()
                },
            ),
        )
        .await
        .into_inner()
        .results;
        let owner = owners.first().expect("participant owner mapping");
        let p::list_owner_to_key_mapping_response::result::Item::V30(owner) =
            owner.item.as_ref().unwrap();
        let participant = ParticipantId::parse(&owner.member).unwrap();
        let roots = ok(
            &endpoint,
            RawRead::new(channel.clone()).list_namespace_delegation(
                p::ListNamespaceDelegationRequest {
                    base_query: Some(head(Some(authorized()))),
                    filter_namespace: participant.namespace().to_string(),
                    ..Default::default()
                },
            ),
        )
        .await
        .into_inner()
        .results;
        let root = roots
            .into_iter()
            .find_map(|result| {
                let p::list_namespace_delegation_response::result::Item::V30(item) =
                    result.item.unwrap();
                // The namespace key signs its own root certificate.
                result
                    .context
                    .unwrap()
                    .signed_by_fingerprints
                    .contains(&item.namespace)
                    .then_some(item)
            })
            .expect("participant root certificate");
        Self {
            endpoint,
            channel,
            read,
            write,
            participant,
            root,
        }
    }

    pub fn raw_read(&self) -> RawRead {
        RawRead::new(self.channel.clone())
    }

    pub fn raw_write(&self) -> RawWrite {
        RawWrite::new(self.channel.clone())
    }

    /// Seed read tests without going through SDK request or response conversions.
    pub async fn seed(
        &self,
        mapping: mapping::topology_mapping::Mapping,
    ) -> mapping::SignedTopologyTransaction {
        ok(
            &self.endpoint,
            self.raw_write().authorize(proposal(mapping, None)),
        )
        .await
        .into_inner()
        .transaction
        .expect("authorized fixture")
    }

    /// Make a local party whose namespace the participant can authorize.
    pub fn party(&self, name: &str) -> mapping::PartyToParticipant {
        mapping::PartyToParticipant {
            party: format!("{name}::{}", self.participant.namespace()),
            threshold: 1,
            participants: vec![mapping::party_to_participant::HostingParticipant {
                participant_uid: self.participant.uid().to_string(),
                permission: mapping::enums::ParticipantPermission::Submission as i32,
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Generate a root certificate using a real locally held namespace key.
    pub async fn new_root(&self) -> mapping::NamespaceDelegation {
        let mut vault = vault::vault_service_client::VaultServiceClient::new(self.channel.clone());
        let response = ok(
            &self.endpoint,
            vault.generate_signing_key(vault::GenerateSigningKeyRequest {
                key_spec: crypto::SigningKeySpec::EcCurve25519 as i32,
                name: "integration-namespace".into(),
                usage_v30: vec![crypto::SigningKeyUsage::Namespace as i32],
                ..Default::default()
            }),
        )
        .await
        .into_inner();
        let vault::generate_signing_key_response::PublicKey::V30(key) =
            response.public_key.unwrap();
        let namespace = EncodedSigningPublicKey::try_from(key.clone())
            .unwrap()
            .fingerprint()
            .expect("SDK supports node signing keys")
            .to_string();
        mapping::NamespaceDelegation {
            namespace: namespace.clone(),
            target_key: Some(key),
            #[allow(deprecated)]
            is_root_delegation: true,
            ..Default::default()
        }
    }

    /// Create another locally controlled namespace using the native vault and a root certificate.
    pub async fn second_namespace(&self) -> Namespace {
        let root = self.new_root().await;
        let namespace = Namespace::new(root.namespace.clone()).unwrap();
        self.seed(mapping::topology_mapping::Mapping::NamespaceDelegation(
            root,
        ))
        .await;
        namespace
    }

    /// Read the exact stored signed transaction, independently decoding its version wrapper.
    #[allow(
        deprecated,
        reason = "ListAll is available on both tested Canton releases"
    )]
    pub async fn stored(
        &self,
        bytes: &[u8],
        store: common::StoreId,
        proposals: bool,
    ) -> mapping::SignedTopologyTransaction {
        let response = ok(
            &self.endpoint,
            self.raw_read().list_all(p::ListAllRequest {
                base_query: Some(p::BaseQuery {
                    proposals,
                    time_query: Some(p::base_query::TimeQuery::Range(
                        p::base_query::TimeRange::default(),
                    )),
                    ..head(Some(store))
                }),
                ..Default::default()
            }),
        )
        .await
        .into_inner();
        response
            .result
            .expect("stored topology transactions")
            .items
            .into_iter()
            .find_map(|item| {
                let tx = mapping::SignedTopologyTransaction::decode(
                    unversion(&item.transaction).as_slice(),
                )
                .unwrap();
                (tx.transaction == bytes).then_some(tx)
            })
            .expect("transaction in requested store")
    }
}

pub fn authorized() -> common::StoreId {
    common::StoreId {
        store: Some(common::store_id::Store::Authorized(
            common::store_id::Authorized {},
        )),
    }
}

pub fn query() -> BaseQuery {
    BaseQuery {
        store: Some(authorized().try_into().unwrap()),
        ..Default::default()
    }
}

/// Construct the wire query independently of BaseQuery's conversion.
pub fn head(store: Option<common::StoreId>) -> p::BaseQuery {
    p::BaseQuery {
        store,
        time_query: Some(p::base_query::TimeQuery::HeadState(())),
        ..Default::default()
    }
}

pub fn proposal(
    item: mapping::topology_mapping::Mapping,
    serial: Option<u32>,
) -> p::AuthorizeRequest {
    p::AuthorizeRequest {
        must_fully_authorize: true,
        store: Some(authorized()),
        r#type: Some(p::authorize_request::Type::Proposal(
            p::authorize_request::Proposal {
                change: mapping::enums::TopologyChangeOp::AddReplace as i32,
                serial: serial.unwrap_or_default(),
                mapping: Some(p::authorize_request::proposal::Mapping::V30(
                    mapping::TopologyMapping {
                        mapping: Some(item),
                    },
                )),
            },
        )),
        ..Default::default()
    }
}

pub fn unversion(bytes: &[u8]) -> Vec<u8> {
    let wrapped = UntypedVersionedMessage::decode(bytes).expect("versioned topology message");
    assert_eq!(wrapped.version, 30, "schema on Canton 3.5.19 and 3.6.2");
    let Wrapper::Data(data) = wrapped.wrapper.expect("versioned payload");
    data
}

pub fn timestamp(value: &prost_types::Timestamp) -> SystemTime {
    SystemTime::try_from(*value).unwrap()
}

/// Compare stable structured server error details, excluding per-request trace identifiers.
pub fn assert_error(actual: AdminApiError, expected: &tonic::Status) {
    let Some(info) = expected.get_details_error_info() else {
        match actual {
            AdminApiError::CantonError(CantonError::Redacted(actual)) => {
                assert!(expected.message().starts_with(RedactedCantonError::PREFIX));
                assert!(actual.message().starts_with(RedactedCantonError::PREFIX));
                assert_eq!(actual.code(), expected.code());
                assert_eq!(
                    actual.request_id().is_some(),
                    expected.get_details_request_info().is_some()
                );
            }
            AdminApiError::CantonError(CantonError::Raw(actual)) => {
                assert!(!expected.message().starts_with(RedactedCantonError::PREFIX));
                assert_eq!(actual.code(), expected.code());
                assert_eq!(actual.message(), expected.message());
            }
            actual => {
                panic!("expected raw or redacted server error {expected:#?}, got {actual:#?}")
            }
        }
        return;
    };
    let AdminApiError::CantonError(CantonError::Decoded(actual)) = actual else {
        panic!("expected decoded Canton error: {actual:#?}")
    };
    assert_eq!(actual.code(), expected.code());
    assert_eq!(actual.error_code_id().as_str(), info.reason);
    assert_eq!(
        i32::from(actual.category_id()),
        info.metadata["category"].parse::<i32>().unwrap()
    );
    assert_eq!(
        actual.definite_answer(),
        info.metadata
            .get("definite_answer")
            .map(|v| v.parse().unwrap())
    );
    let mut metadata = info.metadata;
    for key in ["category", "tid", "definite_answer"] {
        metadata.remove(key);
    }
    assert_eq!(
        actual
            .metadata()
            .keys()
            .collect::<std::collections::BTreeSet<_>>(),
        metadata.keys().collect()
    );
    for (key, value) in metadata {
        if key == "effective" {
            // The absent-hash error reports the time of each independently executed request.
            let parse = |value: &str| {
                chrono::DateTime::parse_from_rfc3339(
                    value
                        .strip_prefix("EffectiveTime(")
                        .unwrap()
                        .strip_suffix(')')
                        .unwrap(),
                )
                .unwrap()
            };
            assert!(parse(&actual.metadata()[&key]) >= parse(&value));
        } else {
            assert_eq!(actual.metadata()[&key], value, "metadata {key}");
        }
    }
    assert_eq!(
        actual.correlation_id().is_some(),
        expected.get_details_request_info().is_some()
    );
    assert_eq!(
        actual.retry_delay(),
        expected
            .get_details_retry_info()
            .and_then(|info| info.retry_delay)
    );
}

/// Check every exposed signed-transaction field against the independently read stored value.
pub fn assert_transaction(
    actual: &SignedTopologyTransaction,
    expected: &mapping::SignedTopologyTransaction,
) {
    assert_eq!(actual.transaction(), expected.transaction);
    assert_eq!(actual.proposal(), expected.proposal);
    assert_eq!(actual.signatures().len(), expected.signatures.len());
    let mut signatures: Vec<_> = actual.signatures().iter().collect();
    let mut raw_signatures: Vec<_> = expected.signatures.iter().collect();
    signatures.sort_unstable_by_key(|s| s.signed_by());
    raw_signatures.sort_unstable_by_key(|s| &s.signed_by);
    for (actual, expected) in signatures.into_iter().zip(raw_signatures) {
        assert_eq!(actual.signature(), expected.signature);
        assert_eq!(actual.signed_by().as_str(), expected.signed_by);
        assert_eq!(
            crypto::SignatureFormat::from(actual.format()) as i32,
            expected.format
        );
        assert_eq!(
            actual
                .signing_algorithm_spec()
                .map(|spec| crypto::SigningAlgorithmSpec::from(spec) as i32)
                .unwrap_or_default(),
            expected.signing_algorithm_spec
        );
        assert!(actual.signature_delegation().is_none());
        assert!(expected.signature_delegation.is_none());
    }
    assert!(actual.multi_transaction_signatures().is_empty());
    assert!(expected.multi_transaction_signatures.is_empty());
}
