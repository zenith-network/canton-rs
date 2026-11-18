//! Authorization, signed transactions, proposal completion, and real server errors.

use admin_api::v30::{
    error::AdminApiError,
    services::{
        topology_read::BaseQuery,
        topology_write::{AuthorizeRequest, Proposal, Type},
    },
};
use admin_api_types::topology::v30::{ForceFlag, StoreId, store::Temporary};
use canton_base::retry::RetryConfig;
use canton_proto::com::digitalasset::canton::{
    protocol::{v30 as m, v31},
    topology::admin::v30 as p,
};
use canton_protocol::{
    SignedTopologyTransaction,
    mappings::{
        DecentralizedNamespaceDefinition, HostingParticipant, NamespaceDelegation,
        OwnerToKeyMapping, PartyToParticipant, TopologyMapping, VettedPackage, VettedPackages,
    },
};
use canton_types::{NonZeroU31, PackageId, topology::Fingerprint};
use pretty_assertions::assert_eq;
use prost::Message;

use crate::support::{Sandbox, ok, sandbox, topology::*, within};

/// Convenience proposals create, replace, and remove a party with automatic and explicit serials.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn propose_mapping_lifecycle_is_observable(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let mut service = ctx.write.clone();
    service.set_retry_config(RetryConfig::no_retry());
    let mut party = ctx.party("integration-write");
    let tx = ok(
        &ctx.endpoint,
        service.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::add_replace_v30(PartyToParticipant::from_proto_v30(party.clone()).unwrap()),
        ),
    )
    .await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        1,
        m::enums::TopologyChangeOp::AddReplace,
    )
    .await;

    party.participants[0].permission = m::enums::ParticipantPermission::Observation as i32;
    let tx = ok(
        &ctx.endpoint,
        service.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::add_replace_v30(PartyToParticipant::from_proto_v30(party.clone()).unwrap())
                .with_serial(NonZeroU31::try_from(2u32).unwrap()),
        ),
    )
    .await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        2,
        m::enums::TopologyChangeOp::AddReplace,
    )
    .await;

    let tx = ok(
        &ctx.endpoint,
        service.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::remove_v30(PartyToParticipant::from_proto_v30(party.clone()).unwrap()),
        ),
    )
    .await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        3,
        m::enums::TopologyChangeOp::Remove,
    )
    .await;
    let head = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_party_to_participant(p::ListPartyToParticipantRequest {
                base_query: Some(head(Some(authorized()))),
                filter_party: party.party.clone(),
                ..Default::default()
            }),
    )
    .await
    .into_inner();
    assert_eq!(head.results.len(), 1);
    assert_eq!(
        head.results[0].context.as_ref().unwrap().operation,
        m::enums::TopologyChangeOp::Remove as i32
    );
    let removed = ok(
        &ctx.endpoint,
        ctx.read
            .list_party_to_participant(query(), Some(party.party.clone()), None),
    )
    .await;
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].context.serial, 3);
    assert_eq!(
        removed[0].context.operation,
        canton_protocol::TopologyChangeOp::Remove
    );
}

/// Explicit authorization honors the signing key, selected store, and effectiveness timeout.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn authorize_with_signing_key_and_timeout_is_observable(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-authorize");
    let mut request = request(Type::from_proposal(Proposal::add_replace_v30(
        PartyToParticipant::from_proto_v30(party.clone()).unwrap(),
    )));
    request.signed_by = vec![Fingerprint::new(ctx.root.namespace.clone()).unwrap()];
    request.wait_to_become_effective = Some(chrono::Duration::seconds(10));
    let tx = ok(&ctx.endpoint, ctx.write.authorize(request)).await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        1,
        m::enums::TopologyChangeOp::AddReplace,
    )
    .await;
    assert_eq!(tx.signatures().len(), 1);
    assert_eq!(tx.signatures()[0].signed_by().as_str(), ctx.root.namespace);
    let results = ok(
        &ctx.endpoint,
        ctx.read
            .list_party_to_participant(query(), Some(party.party), None),
    )
    .await;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].context.transaction_hash,
        tx.transaction_hash().to_bytes()
    );
}

/// A namespace root certificate can be proposed through the typed SDK mapping.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn namespace_delegation_can_be_authorized(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let root = ctx.new_root().await;
    let mapping = NamespaceDelegation::from_proto_v30(root.clone())
        .expect("SDK must accept the real node's namespace signing key format");
    let tx = ok(
        &ctx.endpoint,
        ctx.write.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::add_replace_v30(mapping),
        ),
    )
    .await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::NamespaceDelegation(root),
        1,
        m::enums::TopologyChangeOp::AddReplace,
    )
    .await;
}

/// Reauthorizing an unchanged owner mapping exposes the same duplicate-mapping error as the server.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn duplicate_owner_key_mapping_error_matches_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let raw = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_owner_to_key_mapping(p::ListOwnerToKeyMappingRequest {
                base_query: Some(head(Some(authorized()))),
                filter_key_owner_type: "PAR".into(),
                filter_key_owner_uid: ctx.participant.uid().to_string(),
            }),
    )
    .await
    .into_inner()
    .results
    .pop()
    .expect("participant owner mapping");
    let p::list_owner_to_key_mapping_response::result::Item::V30(mapping) = raw.item.unwrap();
    let typed = OwnerToKeyMapping::from_proto_v30(mapping.clone()).unwrap();
    let expected = within(
        &ctx.endpoint,
        ctx.raw_write().authorize(proposal(
            m::topology_mapping::Mapping::OwnerToKeyMapping(mapping),
            None,
        )),
    )
    .await
    .unwrap_err();
    let actual = within(
        &ctx.endpoint,
        ctx.write.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::add_replace_v30(typed),
        ),
    )
    .await
    .unwrap_err();
    assert_error(actual, &expected);
}

/// A proposal remains separate from approved mappings until a second local owner signs its hash.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn partial_authorization_can_be_completed_by_hash(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let second = ctx.second_namespace().await;
    let dnd = DecentralizedNamespaceDefinition::initial(
        NonZeroU31::try_from(2u32).unwrap(),
        [ctx.participant.namespace().clone(), second.clone()],
    )
    .unwrap();
    let namespace = dnd.decentralized_namespace().to_string();
    let expected = m::DecentralizedNamespaceDefinition {
        decentralized_namespace: namespace.clone(),
        threshold: 2,
        owners: dnd.owners_iter().map(ToString::to_string).collect(),
    };
    let mut partial = request(Type::from_proposal(Proposal::add_replace_v30(dnd)));
    partial.must_fully_authorize = false;
    partial.signed_by = vec![Fingerprint::new(ctx.root.namespace.clone()).unwrap()];
    let tx = ok(&ctx.endpoint, ctx.write.authorize(partial.clone())).await;
    assert!(tx.proposal());
    let stored = ctx.stored(tx.transaction(), authorized(), true).await;
    assert_transaction(&tx, &stored);
    let approved = ok(
        &ctx.endpoint,
        ctx.read
            .list_decentralized_namespace_definition(query(), Some(namespace.clone())),
    )
    .await;
    assert!(approved.is_empty());
    let proposals = ok(
        &ctx.endpoint,
        ctx.read.list_decentralized_namespace_definition(
            BaseQuery {
                proposals: true,
                ..query()
            },
            Some(namespace.clone()),
        ),
    )
    .await;
    let raw = ok(
        &ctx.endpoint,
        ctx.raw_read().list_decentralized_namespace_definition(
            p::ListDecentralizedNamespaceDefinitionRequest {
                base_query: Some(p::BaseQuery {
                    proposals: true,
                    ..head(Some(authorized()))
                }),
                filter_namespace: namespace.clone(),
            },
        ),
    )
    .await
    .into_inner()
    .results;
    assert_eq!(proposals.len(), 1);
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].item.as_ref().unwrap(), &expected);
    assert_eq!(
        proposals[0].context.transaction_hash,
        raw[0].context.as_ref().unwrap().transaction_hash
    );
    assert_eq!(
        proposals[0].item,
        DecentralizedNamespaceDefinition::from_proto_v30(expected.clone()).unwrap()
    );

    // The native signing RPC provides an independent expected response without storing it.
    let signed = ok(
        &ctx.endpoint,
        ctx.raw_write()
            .sign_transactions(p::SignTransactionsRequest {
                transactions: vec![stored],
                signed_by: vec![second.to_string()],
                store: Some(authorized()),
                ..Default::default()
            }),
    )
    .await
    .into_inner()
    .transactions
    .pop()
    .unwrap();
    let mut complete = request(Type::TransactionHash(hex::encode(
        tx.transaction_hash().to_bytes(),
    )));
    complete.signed_by = vec![Fingerprint::new(second.to_string()).unwrap()];
    let completed = ok(&ctx.endpoint, ctx.write.authorize(complete)).await;
    assert_transaction(&completed, &signed);
    assert_mapping(
        &completed,
        m::topology_mapping::Mapping::DecentralizedNamespaceDefinition(expected),
        1,
        m::enums::TopologyChangeOp::AddReplace,
    );
    let persisted = ctx
        .stored(completed.transaction(), authorized(), false)
        .await;
    assert!(
        !persisted.proposal,
        "the store promotes the signed proposal to an approved mapping"
    );
    assert_eq!(completed.transaction_hash(), tx.transaction_hash());
    assert_eq!(completed.signatures().len(), 2);
    let proposals = ok(
        &ctx.endpoint,
        ctx.read.list_decentralized_namespace_definition(
            BaseQuery {
                proposals: true,
                ..query()
            },
            Some(namespace.clone()),
        ),
    )
    .await;
    assert!(proposals.is_empty());
    let approved = ok(
        &ctx.endpoint,
        ctx.read
            .list_decentralized_namespace_definition(query(), Some(namespace.clone())),
    )
    .await;
    assert_eq!(approved.len(), 1);
    assert_eq!(approved[0].context.signed_by_fingerprints.len(), 2);
    assert_eq!(
        approved[0].context.transaction_hash,
        completed.transaction_hash().to_bytes()
    );

    // A new party in this namespace needs both owners when full authorization is required.
    let mut party = ctx.party("integration-insufficient");
    party.party = format!("integration-insufficient::{namespace}");
    let mut raw = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        None,
    );
    raw.signed_by = vec![ctx.root.namespace.clone()];
    let error = within(&ctx.endpoint, ctx.raw_write().authorize(raw))
        .await
        .unwrap_err();
    let mut full = request(Type::from_proposal(Proposal::add_replace_v30(
        PartyToParticipant::from_proto_v30(party).unwrap(),
    )));
    full.signed_by = vec![Fingerprint::new(ctx.root.namespace.clone()).unwrap()];
    assert_error(
        within(&ctx.endpoint, ctx.write.authorize(full))
            .await
            .unwrap_err(),
        &error,
    );
}

/// Unknown packages fail without a force flag and are persisted when the flag is supplied.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn authorize_unknown_package_requires_force_flag(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let package = "3".repeat(64);
    let raw = m::VettedPackages {
        participant_uid: ctx.participant.uid().to_string(),
        packages: vec![m::vetted_packages::VettedPackage {
            package_id: package.clone(),
            ..Default::default()
        }],
        ..Default::default()
    };
    let typed = VettedPackages::new(
        ctx.participant.clone(),
        vec![VettedPackage::new_unbounded_validity(
            PackageId::new(package).unwrap(),
        )],
    )
    .unwrap();
    let mut request = request(Type::from_proposal(Proposal::add_replace_v30(typed)));
    let expected = within(
        &ctx.endpoint,
        ctx.raw_write().authorize(proposal(
            m::topology_mapping::Mapping::VettedPackages(raw.clone()),
            None,
        )),
    )
    .await
    .unwrap_err();
    let actual = within(&ctx.endpoint, ctx.write.authorize(request.clone()))
        .await
        .unwrap_err();
    assert_error(actual, &expected);
    request.force_changes = vec![ForceFlag::AllowUnknownPackage];
    let tx = ok(&ctx.endpoint, ctx.write.authorize(request)).await;
    verify(
        &ctx,
        &tx,
        m::topology_mapping::Mapping::VettedPackages(raw),
        1,
        m::enums::TopologyChangeOp::AddReplace,
    )
    .await;
    let vetted = ok(
        &ctx.endpoint,
        ctx.read
            .list_vetted_packages(query(), Some(ctx.participant.identifier().into())),
    )
    .await;
    assert_eq!(vetted.len(), 1);
    assert_eq!(vetted[0].item.packages().len(), 1);
}

/// Stale serials produce the same error and leave the existing topology generation intact.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn stale_serial_error_matches_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let mut party = ctx.party("integration-stale");
    ctx.seed(m::topology_mapping::Mapping::PartyToParticipant(
        party.clone(),
    ))
    .await;
    party.participants[0].permission = m::enums::ParticipantPermission::Observation as i32;
    let expected = within(
        &ctx.endpoint,
        ctx.raw_write().authorize(proposal(
            m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
            Some(1),
        )),
    )
    .await
    .unwrap_err();
    let actual = within(
        &ctx.endpoint,
        ctx.write.propose_mapping(
            true,
            Some(authorized().try_into().unwrap()),
            Proposal::add_replace_v30(PartyToParticipant::from_proto_v30(party.clone()).unwrap())
                .with_serial(NonZeroU31::MIN),
        ),
    )
    .await
    .unwrap_err();
    assert_error(actual, &expected);
    let results = ok(
        &ctx.endpoint,
        ctx.read
            .list_party_to_participant(query(), Some(party.party), None),
    )
    .await;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].context.serial, 1);
    assert_eq!(
        results[0].item.participants()[0].permission(),
        canton_protocol::ParticipantPermission::Submission
    );
}

/// Absent and malformed hashes retain server error details through the SDK.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn transaction_hash_errors_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    for hash in [format!("1220{}", "f".repeat(64)), "not-hex".into()] {
        let expected = within(
            &ctx.endpoint,
            ctx.raw_write().authorize(p::AuthorizeRequest {
                must_fully_authorize: true,
                store: Some(authorized()),
                r#type: Some(p::authorize_request::Type::TransactionHash(hash.clone())),
                ..Default::default()
            }),
        )
        .await
        .unwrap_err();
        let actual = within(
            &ctx.endpoint,
            ctx.write.authorize(request(Type::TransactionHash(hash))),
        )
        .await
        .unwrap_err();
        assert_error(actual, &expected);
    }
}

/// Requests cannot select an unavailable signing key or a nonexistent store.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn signing_key_and_store_errors_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-error");
    let typed = PartyToParticipant::from_proto_v30(party.clone()).unwrap();
    let mut raw = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        None,
    );
    let key = format!("1220{}", "f".repeat(64));
    raw.signed_by = vec![key.clone()];
    let expected = within(&ctx.endpoint, ctx.raw_write().authorize(raw))
        .await
        .unwrap_err();
    let mut actual = request(Type::from_proposal(Proposal::add_replace_v30(
        typed.clone(),
    )));
    actual.signed_by = vec![Fingerprint::new(key).unwrap()];
    assert_error(
        within(&ctx.endpoint, ctx.write.authorize(actual))
            .await
            .unwrap_err(),
        &expected,
    );

    let store = StoreId::Temporary(Temporary::with_name("integration-absent".into()));
    let mut raw = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        None,
    );
    raw.store = Some(store.clone().into());
    let expected = within(&ctx.endpoint, ctx.raw_write().authorize(raw))
        .await
        .unwrap_err();
    let mut actual = request(Type::from_proposal(Proposal::add_replace_v30(
        typed.clone(),
    )));
    actual.store = Some(store);
    assert_error(
        within(&ctx.endpoint, ctx.write.authorize(actual))
            .await
            .unwrap_err(),
        &expected,
    );

    // Both tested releases require a store, even though it is optional on the protobuf request.
    let mut raw = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party),
        None,
    );
    raw.store = None;
    let expected = within(&ctx.endpoint, ctx.raw_write().authorize(raw))
        .await
        .unwrap_err();
    let mut actual = request(Type::from_proposal(Proposal::add_replace_v30(
        typed.clone(),
    )));
    actual.store = None;
    assert_error(
        within(&ctx.endpoint, ctx.write.authorize(actual))
            .await
            .unwrap_err(),
        &expected,
    );
    assert_error(
        within(
            &ctx.endpoint,
            ctx.write
                .propose_mapping(true, None, Proposal::add_replace_v30(typed)),
        )
        .await
        .unwrap_err(),
        &expected,
    );
}

/// Neither tested release accepts v31 proposals; SDK requests must expose that server rejection.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn unsupported_v31_proposal_error_matches_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-v31");
    let expected = within(&ctx.endpoint, ctx.raw_write().authorize(p::AuthorizeRequest {
        must_fully_authorize: true, store: Some(authorized()),
        r#type: Some(p::authorize_request::Type::Proposal(p::authorize_request::Proposal {
            change: m::enums::TopologyChangeOp::AddReplace as i32, serial: 0,
            mapping: Some(p::authorize_request::proposal::Mapping::V31(v31::TopologyMapping {
                mapping: Some(v31::topology_mapping::Mapping::PartyToParticipant(v31::PartyToParticipant {
                    party: party.party.clone(), threshold: 1,
                    participants: vec![v31::party_to_participant::HostingParticipant {
                        participant_uid: ctx.participant.uid().to_string(), permission: m::enums::ParticipantPermission::Submission as i32,
                        state: v31::party_to_participant::hosting_participant::HostingState::Active as i32,
                    }], ..Default::default()
                })),
            })),
        })), ..Default::default()
    })).await.unwrap_err();
    let typed = PartyToParticipant::from_proto_v30(party.clone()).unwrap();
    for proposal in [
        Proposal::add_replace_v31(typed.clone()),
        Proposal::remove_v31(typed),
    ] {
        let actual = within(
            &ctx.endpoint,
            ctx.write.propose_mapping(true, None, proposal),
        )
        .await
        .unwrap_err();
        assert_error(actual, &expected);
    }
    assert!(
        ok(
            &ctx.endpoint,
            ctx.read
                .list_party_to_participant(query(), Some(party.party), None)
        )
        .await
        .is_empty()
    );
}

/// Unsupported v30 hosting states fail locally and never create a party mapping.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn incompatible_mapping_is_an_input_error(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-input-error");
    let mapping = PartyToParticipant::new(
        party.party.parse().unwrap(),
        NonZeroU31::MIN,
        vec![
            HostingParticipant::from_proto_v31(v31::party_to_participant::HostingParticipant {
                participant_uid: ctx.participant.uid().to_string(),
                permission: m::enums::ParticipantPermission::Submission as i32,
                state: v31::party_to_participant::hosting_participant::HostingState::Nominated
                    as i32,
            })
            .unwrap(),
        ],
        None,
    )
    .unwrap();
    let proposal = Proposal::add_replace_v30(mapping);
    let convenience = within(
        &ctx.endpoint,
        ctx.write.propose_mapping(true, None, proposal.clone()),
    )
    .await
    .unwrap_err();
    let explicit = within(
        &ctx.endpoint,
        ctx.write.authorize(request(Type::from_proposal(proposal))),
    )
    .await
    .unwrap_err();
    for error in [convenience, explicit] {
        assert!(
            matches!(error, AdminApiError::InputError(_)),
            "local encoding error: {error:#?}"
        );
    }
    let raw = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_party_to_participant(p::ListPartyToParticipantRequest {
                base_query: Some(head(Some(authorized()))),
                filter_party: party.party,
                ..Default::default()
            }),
    )
    .await
    .into_inner();
    assert!(raw.results.is_empty());
}

fn request(type_: Type) -> AuthorizeRequest {
    AuthorizeRequest {
        must_fully_authorize: true,
        force_changes: Vec::new(),
        signed_by: Vec::new(),
        store: Some(authorized().try_into().unwrap()),
        wait_to_become_effective: None,
        type_,
    }
}

/// Decode transaction contents independently and compare the SDK response with the stored signature.
async fn verify(
    ctx: &Context,
    tx: &SignedTopologyTransaction,
    expected: m::topology_mapping::Mapping,
    serial: u32,
    operation: m::enums::TopologyChangeOp,
) {
    assert_mapping(tx, expected, serial, operation);
    assert!(!tx.proposal());
    let stored = ctx.stored(tx.transaction(), authorized(), false).await;
    assert_transaction(tx, &stored);
}

fn assert_mapping(
    tx: &SignedTopologyTransaction,
    expected: m::topology_mapping::Mapping,
    serial: u32,
    operation: m::enums::TopologyChangeOp,
) {
    let decoded = m::TopologyTransaction::decode(unversion(tx.transaction()).as_slice()).unwrap();
    assert_eq!(decoded.serial, serial);
    assert_eq!(decoded.operation, operation as i32);
    assert_eq!(
        canonical_mapping(decoded.mapping.unwrap().mapping.unwrap()),
        canonical_mapping(expected)
    );
}

/// Legacy and structured namespace restrictions describe the same mapping on these releases.
#[allow(
    deprecated,
    reason = "normalize the legacy delegation flag for semantic comparison"
)]
fn canonical_mapping(mut mapping: m::topology_mapping::Mapping) -> m::topology_mapping::Mapping {
    if let m::topology_mapping::Mapping::NamespaceDelegation(namespace) = &mut mapping {
        if namespace.restriction.is_none() {
            namespace.restriction = Some(if namespace.is_root_delegation {
                m::namespace_delegation::Restriction::CanSignAllMappings(
                    m::namespace_delegation::CanSignAllMappings {},
                )
            } else {
                m::namespace_delegation::Restriction::CanSignAllButNamespaceDelegations(
                    m::namespace_delegation::CanSignAllButNamespaceDelegations {},
                )
            });
        }
        namespace.is_root_delegation = false;
    }
    mapping
}
