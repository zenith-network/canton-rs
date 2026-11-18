//! Topology mappings, request filters, stores, and historical state against real participants.

use std::time::{Duration, SystemTime};

use admin_api::v30::services::topology_read::{BaseQuery, BaseResult, TimeQuery, TopologyResult};
use admin_api_proto::com::digitalasset::canton::{
    admin::participant::v30 as status, topology::admin::v30 as common,
};
use admin_api_types::topology::v30::{StoreId, Synchronizer, store::Temporary};
use canton_proto::com::digitalasset::canton::{
    crypto::v30 as crypto, protocol::v30 as m, topology::admin::v30 as p,
};
use canton_protocol::{
    TopologyChangeOp,
    crypto::v30::{EncodedPublicKey, EncodedSigningPublicKey},
    mappings::{
        DecentralizedNamespaceDefinition, NamespaceDelegation, OwnerToKeyMapping,
        PartyToParticipant, Restriction, VettedPackages,
    },
};
use canton_types::topology::{Fingerprint, MemberCode};
use pretty_assertions::assert_eq;

use crate::support::{Sandbox, ok, sandbox, topology::*, within};

/// Default and explicit stores preserve namespace certificates and both namespace filters.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn namespace_delegations_and_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let all = namespaces(&ctx, BaseQuery::default(), head(None), None, None).await;
    assert!(!all.is_empty());
    let namespace = ctx.root.namespace.as_str();
    let filtered = namespaces(
        &ctx,
        query(),
        head(Some(authorized())),
        Some(namespace),
        Some(namespace),
    )
    .await;
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].item.namespace().as_str(), namespace);
    assert!(
        !namespaces(
            &ctx,
            query(),
            head(Some(authorized())),
            Some(&namespace[..12]),
            None
        )
        .await
        .is_empty()
    );
    assert!(
        namespaces(
            &ctx,
            query(),
            head(Some(authorized())),
            Some("integration-absent"),
            None
        )
        .await
        .is_empty()
    );
    assert!(
        namespaces(
            &ctx,
            query(),
            head(Some(authorized())),
            None,
            Some("integration-absent")
        )
        .await
        .is_empty()
    );
    // An explicitly empty string has the same meaning as an omitted filter.
    assert!(
        !namespaces(&ctx, query(), head(Some(authorized())), Some(""), None)
            .await
            .is_empty()
    );
}

/// Owner type and UID filters retain the server's full public-key information.
#[sandbox(canton_version = "^3.5.19", shared)]
#[tokio::test]
async fn owner_to_key_mappings_and_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    assert!(!owners(&ctx, None, None).await.is_empty());
    for code in [
        MemberCode::Participant,
        MemberCode::Mediator,
        MemberCode::Sequencer,
    ] {
        let results = owners(&ctx, Some(code), None).await;
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .all(|r| r.item.member().member_code() == code)
        );
    }
    let uid = ctx.participant.uid().to_string();
    let results = owners(&ctx, Some(MemberCode::Participant), Some(&uid)).await;
    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|r| r.item.member().uid() == ctx.participant.uid())
    );
    assert!(
        !owners(
            &ctx,
            Some(MemberCode::Participant),
            Some(ctx.participant.identifier())
        )
        .await
        .is_empty()
    );
    assert!(
        owners(&ctx, None, Some("integration-absent"))
            .await
            .is_empty()
    );
    assert!(
        owners(&ctx, Some(MemberCode::Mediator), Some(&uid))
            .await
            .is_empty()
    );
}

/// A native single-owner definition provides a nonempty decentralized-namespace response.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn decentralized_namespaces_and_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let dnd =
        DecentralizedNamespaceDefinition::initial_single_owner(ctx.participant.namespace().clone());
    let namespace = dnd.decentralized_namespace().to_string();
    ctx.seed(
        m::topology_mapping::Mapping::DecentralizedNamespaceDefinition(
            m::DecentralizedNamespaceDefinition {
                decentralized_namespace: namespace.clone(),
                threshold: 1,
                owners: vec![ctx.participant.namespace().to_string()],
            },
        ),
    )
    .await;
    for filter in [None, Some(namespace.as_str()), Some(&namespace[..12])] {
        let results = definitions(&ctx, filter).await;
        assert!(
            results
                .iter()
                .any(|r| r.item.decentralized_namespace().as_str() == namespace)
        );
    }
    assert!(
        definitions(&ctx, Some("integration-absent"))
            .await
            .is_empty()
    );
}

/// Read vetted package IDs and validity intervals from a real, nonempty server mapping.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn vetted_packages_and_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let mut request = proposal(
        m::topology_mapping::Mapping::VettedPackages(m::VettedPackages {
            participant_uid: ctx.participant.uid().to_string(),
            packages: vec![
                m::vetted_packages::VettedPackage {
                    package_id: "1".repeat(64),
                    ..Default::default()
                },
                m::vetted_packages::VettedPackage {
                    package_id: "2".repeat(64),
                    valid_from_inclusive: Some(
                        (SystemTime::UNIX_EPOCH + Duration::from_secs(1)).into(),
                    ),
                    valid_until_exclusive: Some(
                        (SystemTime::UNIX_EPOCH + Duration::from_secs(4_102_444_800)).into(),
                    ),
                },
            ],
            ..Default::default()
        }),
        None,
    );
    request.force_changes = vec![p::ForceFlag::AllowUnknownPackage as i32];
    ok(&ctx.endpoint, ctx.raw_write().authorize(request)).await;
    for filter in [
        None,
        Some(ctx.participant.identifier()),
        Some(ctx.participant.uid().to_string().as_str()),
    ] {
        let results = packages(&ctx, filter).await;
        let participant = results
            .iter()
            .find(|r| r.item.participant_uid() == &ctx.participant)
            .unwrap();
        assert_eq!(participant.item.packages().len(), 2);
    }
    assert!(packages(&ctx, Some("integration-absent")).await.is_empty());
}

/// Party and participant filters are independently applied to a known local party.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn party_to_participant_mappings_and_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-read");
    ctx.seed(m::topology_mapping::Mapping::PartyToParticipant(
        party.clone(),
    ))
    .await;
    for (party_filter, participant_filter) in [
        (None, None),
        (Some(party.party.as_str()), None),
        (Some("integration-read"), Some(ctx.participant.identifier())),
    ] {
        let results = parties(
            &ctx,
            query(),
            head(Some(authorized())),
            party_filter,
            participant_filter,
        )
        .await;
        assert!(results.iter().any(|r| *r.item.party() == party.party));
    }
    for (party_filter, participant_filter) in [
        (Some("integration-absent"), None),
        (None, Some("integration-absent")),
    ] {
        assert!(
            parties(
                &ctx,
                query(),
                head(Some(authorized())),
                party_filter,
                participant_filter
            )
            .await
            .is_empty()
        );
    }
}

/// Store selection includes synchronizer and native temporary stores; absent stores report errors.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn topology_store_selection_matches_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let party = ctx.party("integration-store");
    let stores = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_available_stores(p::ListAvailableStoresRequest {}),
    )
    .await
    .into_inner()
    .store_ids;
    assert!(
        stores
            .iter()
            .any(|s| matches!(s.store, Some(common::store_id::Store::Synchronizer(_))))
    );
    for store in stores {
        let results = namespaces(
            &ctx,
            BaseQuery {
                store: Some(store.clone().try_into().unwrap()),
                ..Default::default()
            },
            head(Some(store)),
            None,
            None,
        )
        .await;
        assert!(!results.is_empty());
    }
    let status = ok(
        &ctx.endpoint,
        status::participant_status_service_client::ParticipantStatusServiceClient::new(
            ctx.channel.clone(),
        )
        .participant_status(status::ParticipantStatusRequest::default()),
    )
    .await
    .into_inner();
    let Some(status::participant_status_response::Kind::Status(status)) = status.kind else {
        panic!("initialized participant")
    };
    let temp = ok(
        &ctx.endpoint,
        ctx.raw_write()
            .create_temporary_topology_store(p::CreateTemporaryTopologyStoreRequest {
                name: "integration-store".into(),
                protocol_version: status.supported_protocol_versions[0].try_into().unwrap(),
            }),
    )
    .await
    .into_inner()
    .store_id
    .unwrap();
    let store = common::StoreId {
        store: Some(common::store_id::Store::Temporary(temp.clone())),
    };
    let mut seed = proposal(
        m::topology_mapping::Mapping::NamespaceDelegation(ctx.root.clone()),
        None,
    );
    seed.store = Some(store.clone());
    ok(&ctx.endpoint, ctx.raw_write().authorize(seed)).await;
    let mut seed = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party),
        None,
    );
    seed.store = Some(store.clone());
    ok(&ctx.endpoint, ctx.raw_write().authorize(seed)).await;
    let results = parties(
        &ctx,
        BaseQuery {
            store: Some(store.clone().try_into().unwrap()),
            ..Default::default()
        },
        head(Some(store.clone())),
        None,
        None,
    )
    .await;
    assert_eq!(results.len(), 1);
    assert!(matches!(results[0].context.store, StoreId::Temporary(_)));
    ok(
        &ctx.endpoint,
        ctx.raw_write()
            .drop_temporary_topology_store(p::DropTemporaryTopologyStoreRequest {
                store_id: Some(temp),
            }),
    )
    .await;
    let expected = within(
        &ctx.endpoint,
        ctx.raw_read()
            .list_party_to_participant(p::ListPartyToParticipantRequest {
                base_query: Some(head(Some(store))),
                ..Default::default()
            }),
    )
    .await
    .unwrap_err();
    let actual = within(
        &ctx.endpoint,
        ctx.read.list_party_to_participant(
            BaseQuery {
                store: Some(StoreId::Temporary(Temporary::with_name(
                    "integration-store".into(),
                ))),
                ..Default::default()
            },
            None,
            None,
        ),
    )
    .await
    .unwrap_err();
    assert_error(actual, &expected);
}

/// Replacements and removal expose old validity intervals, snapshots, ranges, and operation filters.
#[sandbox(canton_version = "^3.5.19", fresh)]
#[tokio::test]
async fn topology_history_and_base_query_filters_match_server(sandbox: &Sandbox) {
    let ctx = Context::new(sandbox).await;
    let mut party = ctx.party("integration-history");
    ctx.seed(m::topology_mapping::Mapping::PartyToParticipant(
        party.clone(),
    ))
    .await;
    party.participants[0].permission = m::enums::ParticipantPermission::Observation as i32;
    ctx.seed(m::topology_mapping::Mapping::PartyToParticipant(
        party.clone(),
    ))
    .await;
    let mut remove = proposal(
        m::topology_mapping::Mapping::PartyToParticipant(party.clone()),
        None,
    );
    let Some(p::authorize_request::Type::Proposal(proposal)) = &mut remove.r#type else {
        unreachable!()
    };
    proposal.change = m::enums::TopologyChangeOp::Remove as i32;
    ok(&ctx.endpoint, ctx.raw_write().authorize(remove)).await;
    let raw_range = p::base_query::TimeQuery::Range(p::base_query::TimeRange::default());
    let history = parties(
        &ctx,
        BaseQuery {
            time_query: TimeQuery::Range {
                from: None,
                until: None,
            },
            ..query()
        },
        p::BaseQuery {
            time_query: Some(raw_range),
            ..head(Some(authorized()))
        },
        Some(&party.party),
        None,
    )
    .await;
    assert_eq!(history.len(), 3);
    let first = history.iter().find(|r| r.context.serial == 1).unwrap();
    let second = history.iter().find(|r| r.context.serial == 2).unwrap();
    assert!(first.context.valid_until.is_some());
    let from = first.context.sequenced;
    let until = second.context.sequenced;
    for (from, until) in [
        (None, None),
        (Some(from), None),
        (None, Some(until)),
        (Some(from), Some(until)),
    ] {
        parties(
            &ctx,
            BaseQuery {
                time_query: TimeQuery::Range { from, until },
                ..query()
            },
            p::BaseQuery {
                time_query: Some(p::base_query::TimeQuery::Range(p::base_query::TimeRange {
                    from: from.map(Into::into),
                    until: until.map(Into::into),
                })),
                ..head(Some(authorized()))
            },
            Some(&party.party),
            None,
        )
        .await;
    }
    // Snapshots use the same server timestamp on both paths, including the validity boundary.
    for time in [first.context.valid_from, first.context.valid_until.unwrap()] {
        parties(
            &ctx,
            BaseQuery {
                time_query: TimeQuery::Snapshot(time),
                ..query()
            },
            p::BaseQuery {
                time_query: Some(p::base_query::TimeQuery::Snapshot(time.into())),
                ..head(Some(authorized()))
            },
            Some(&party.party),
            None,
        )
        .await;
    }
    for (operation, raw) in [
        (
            TopologyChangeOp::Replace,
            m::enums::TopologyChangeOp::AddReplace,
        ),
        (TopologyChangeOp::Remove, m::enums::TopologyChangeOp::Remove),
    ] {
        let results = parties(
            &ctx,
            BaseQuery {
                operation: Some(operation),
                time_query: TimeQuery::Range {
                    from: None,
                    until: None,
                },
                ..query()
            },
            p::BaseQuery {
                operation: raw as i32,
                time_query: Some(p::base_query::TimeQuery::Range(
                    p::base_query::TimeRange::default(),
                )),
                ..head(Some(authorized()))
            },
            Some(&party.party),
            None,
        )
        .await;
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.context.operation == operation));
    }
    for key in [
        ctx.root.namespace.as_str(),
        &ctx.root.namespace[..12],
        "integration-absent",
    ] {
        let results = parties(
            &ctx,
            BaseQuery {
                filter_signed_key: Some(key.into()),
                time_query: TimeQuery::Range {
                    from: None,
                    until: None,
                },
                ..query()
            },
            p::BaseQuery {
                filter_signed_key: key.into(),
                time_query: Some(p::base_query::TimeQuery::Range(
                    p::base_query::TimeRange::default(),
                )),
                ..head(Some(authorized()))
            },
            Some(&party.party),
            None,
        )
        .await;
        assert_eq!(
            results.len(),
            if key == "integration-absent" { 0 } else { 3 }
        );
    }
    assert!(
        parties(
            &ctx,
            BaseQuery {
                proposals: true,
                ..query()
            },
            p::BaseQuery {
                proposals: true,
                ..head(Some(authorized()))
            },
            Some(&party.party),
            None
        )
        .await
        .is_empty()
    );
}

async fn namespaces(
    ctx: &Context,
    query: BaseQuery,
    raw: p::BaseQuery,
    namespace: Option<&str>,
    target: Option<&str>,
) -> Vec<TopologyResult<NamespaceDelegation>> {
    let expected = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_namespace_delegation(p::ListNamespaceDelegationRequest {
                base_query: Some(raw),
                filter_namespace: namespace.unwrap_or_default().into(),
                filter_target_key_fingerprint: target.unwrap_or_default().into(),
            }),
    )
    .await
    .into_inner()
    .results
    .into_iter()
    .map(|r| {
        let p::list_namespace_delegation_response::result::Item::V30(item) = r.item.unwrap();
        (r.context.unwrap(), item)
    })
    .collect();
    let actual = ok(
        &ctx.endpoint,
        ctx.read.list_namespace_delegation(
            query,
            namespace.map(Into::into),
            target.map(|s| Fingerprint::new(s.into()).unwrap()),
        ),
    )
    .await;
    compare(&actual, expected, assert_namespace);
    actual
}

async fn owners(
    ctx: &Context,
    code: Option<MemberCode>,
    uid: Option<&str>,
) -> Vec<TopologyResult<OwnerToKeyMapping>> {
    let expected = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_owner_to_key_mapping(p::ListOwnerToKeyMappingRequest {
                base_query: Some(head(None)),
                filter_key_owner_type: code.map(|c| c.as_str()).unwrap_or_default().into(),
                filter_key_owner_uid: uid.unwrap_or_default().into(),
            }),
    )
    .await
    .into_inner()
    .results
    .into_iter()
    .map(|r| {
        let p::list_owner_to_key_mapping_response::result::Item::V30(item) = r.item.unwrap();
        (r.context.unwrap(), item)
    })
    .collect();
    let actual = ok(
        &ctx.endpoint,
        ctx.read
            .list_owner_to_key_mapping(BaseQuery::default(), code, uid.map(Into::into)),
    )
    .await;
    compare(&actual, expected, |actual, expected| {
        assert_eq!(actual.member().to_string(), expected.member);
        assert_eq!(actual.public_keys().len().get(), expected.public_keys.len());
        for (actual, expected) in actual.public_keys().iter().zip(&expected.public_keys) {
            match (actual, expected.key.as_ref().unwrap()) {
                (EncodedPublicKey::Signing(a), crypto::public_key::Key::SigningPublicKey(e)) => {
                    assert_signing_key(a, e)
                }
                (
                    EncodedPublicKey::Encryption(a),
                    crypto::public_key::Key::EncryptionPublicKey(e),
                ) => {
                    assert_eq!(a.public_key(), e.public_key);
                    assert_eq!(crypto::CryptoKeyFormat::from(a.format()) as i32, e.format);
                    assert_eq!(
                        crypto::EncryptionKeySpec::from(a.key_spec()) as i32,
                        e.key_spec
                    );
                }
                other => panic!("different public key variants: {other:?}"),
            }
        }
    });
    actual
}

async fn definitions(
    ctx: &Context,
    namespace: Option<&str>,
) -> Vec<TopologyResult<DecentralizedNamespaceDefinition>> {
    let expected = ok(
        &ctx.endpoint,
        ctx.raw_read().list_decentralized_namespace_definition(
            p::ListDecentralizedNamespaceDefinitionRequest {
                base_query: Some(head(Some(authorized()))),
                filter_namespace: namespace.unwrap_or_default().into(),
            },
        ),
    )
    .await
    .into_inner()
    .results
    .into_iter()
    .map(|r| (r.context.unwrap(), r.item.unwrap()))
    .collect();
    let actual = ok(
        &ctx.endpoint,
        ctx.read
            .list_decentralized_namespace_definition(query(), namespace.map(Into::into)),
    )
    .await;
    compare(&actual, expected, |actual, expected| {
        assert_eq!(
            actual.decentralized_namespace().as_str(),
            expected.decentralized_namespace
        );
        assert_eq!(i32::from(actual.threshold()), expected.threshold);
        let mut owners: Vec<_> = expected.owners.iter().map(String::as_str).collect();
        owners.sort_unstable();
        assert_eq!(
            actual.owners_iter().map(|o| o.as_str()).collect::<Vec<_>>(),
            owners
        );
    });
    actual
}

#[allow(
    deprecated,
    reason = "compare legacy package ID fields returned by older Canton nodes"
)]
async fn packages(ctx: &Context, participant: Option<&str>) -> Vec<TopologyResult<VettedPackages>> {
    let expected = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_vetted_packages(p::ListVettedPackagesRequest {
                base_query: Some(head(Some(authorized()))),
                filter_participant: participant.unwrap_or_default().into(),
            }),
    )
    .await
    .into_inner()
    .results
    .into_iter()
    .map(|r| (r.context.unwrap(), r.item.unwrap()))
    .collect();
    let actual = ok(
        &ctx.endpoint,
        ctx.read
            .list_vetted_packages(query(), participant.map(Into::into)),
    )
    .await;
    compare(&actual, expected, |actual, expected| {
        assert_eq!(
            actual.participant_uid().uid().to_string(),
            expected.participant_uid
        );
        let mut raw: Vec<_> = expected
            .packages
            .iter()
            .map(|p| {
                (
                    p.package_id.as_str(),
                    p.valid_from_inclusive.as_ref().map(timestamp),
                    p.valid_until_exclusive.as_ref().map(timestamp),
                )
            })
            .collect();
        raw.extend(
            expected
                .package_ids
                .iter()
                .map(|id| (id.as_str(), None, None)),
        );
        raw.sort_unstable();
        let mut actual: Vec<_> = actual
            .packages()
            .iter()
            .map(|p| {
                (
                    p.package_id().as_str(),
                    p.valid_from_inclusive(),
                    p.valid_until_exclusive(),
                )
            })
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, raw);
    });
    actual
}

pub async fn parties(
    ctx: &Context,
    query: BaseQuery,
    raw: p::BaseQuery,
    party: Option<&str>,
    participant: Option<&str>,
) -> Vec<TopologyResult<PartyToParticipant>> {
    let expected = ok(
        &ctx.endpoint,
        ctx.raw_read()
            .list_party_to_participant(p::ListPartyToParticipantRequest {
                base_query: Some(raw),
                filter_party: party.unwrap_or_default().into(),
                filter_participant: participant.unwrap_or_default().into(),
            }),
    )
    .await
    .into_inner()
    .results
    .into_iter()
    .map(|r| {
        let p::list_party_to_participant_response::result::Item::V30(item) = r.item.unwrap() else {
            panic!("v30 on the tested Canton releases")
        };
        (r.context.unwrap(), item)
    })
    .collect();
    let actual = ok(
        &ctx.endpoint,
        ctx.read.list_party_to_participant(
            query,
            party.map(Into::into),
            participant.map(Into::into),
        ),
    )
    .await;
    compare(&actual, expected, |actual, expected| {
        assert_eq!(actual.party().to_string(), expected.party);
        assert_eq!(u32::from(actual.threshold()), expected.threshold);
        assert_eq!(actual.participants().len(), expected.participants.len());
        for (actual, expected) in actual.participants().iter().zip(&expected.participants) {
            assert_eq!(
                actual.participant_uid().uid().to_string(),
                expected.participant_uid
            );
            assert_eq!(
                actual.permission().into_proto_v30_primitive(),
                expected.permission
            );
            assert_eq!(actual.onboarding(), expected.onboarding.is_some());
        }
        assert!(actual.party_signing_keys().is_none());
        assert!(expected.party_signing_keys.is_none());
    });
    actual
}

/// Match by transaction and store so duplicated mappings across stores cannot hide omissions.
fn compare<T, P>(
    actual: &[TopologyResult<T>],
    mut expected: Vec<(p::BaseResult, P)>,
    item: impl Fn(&T, &P),
) {
    assert_eq!(actual.len(), expected.len());
    for actual in actual {
        let index = expected
            .iter()
            .position(|(e, _)| {
                e.transaction_hash == actual.context.transaction_hash
                    && raw_store_name(e.store.as_ref().unwrap())
                        == store_name(&actual.context.store)
            })
            .expect("matching transaction and store");
        let (context, expected) = expected.swap_remove(index);
        assert_context(&actual.context, &context);
        item(&actual.item, &expected);
    }
}

fn assert_context(actual: &BaseResult, expected: &p::BaseResult) {
    assert_eq!(
        store_name(&actual.store),
        raw_store_name(expected.store.as_ref().unwrap())
    );
    assert_eq!(
        actual.sequenced,
        timestamp(expected.sequenced.as_ref().unwrap())
    );
    assert_eq!(
        actual.valid_from,
        timestamp(expected.valid_from.as_ref().unwrap())
    );
    assert_eq!(
        actual.valid_until,
        expected.valid_until.as_ref().map(timestamp)
    );
    assert_eq!(
        actual.operation.into_proto_v30_primitive(),
        expected.operation
    );
    assert_eq!(actual.transaction_hash, expected.transaction_hash);
    assert_eq!(actual.serial, expected.serial as u32);
    let mut actual: Vec<_> = actual
        .signed_by_fingerprints
        .iter()
        .map(|f| f.as_str())
        .collect();
    let mut expected: Vec<_> = expected
        .signed_by_fingerprints
        .iter()
        .map(String::as_str)
        .collect();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn store_name(store: &StoreId) -> String {
    match store {
        StoreId::Authorized(_) => "authorized".into(),
        StoreId::Temporary(temp) => format!("temporary:{}", temp.name),
        StoreId::Synchronizer(Synchronizer::Id(id)) => format!("synchronizer:{id}"),
        StoreId::Synchronizer(Synchronizer::PhysicalId(id)) => format!("synchronizer:{id}"),
        _ => panic!("unexpected store: {store:?}"),
    }
}

fn raw_store_name(store: &common::StoreId) -> String {
    match store.store.as_ref().unwrap() {
        common::store_id::Store::Authorized(_) => "authorized".into(),
        common::store_id::Store::Temporary(temp) => format!("temporary:{}", temp.name),
        common::store_id::Store::Synchronizer(sync) => {
            let id = match sync.kind.as_ref().unwrap() {
                common::synchronizer::Kind::Id(id) | common::synchronizer::Kind::PhysicalId(id) => {
                    id
                }
            };
            format!("synchronizer:{id}")
        }
    }
}

#[allow(
    deprecated,
    reason = "compare both legacy and structured delegation restrictions"
)]
fn assert_namespace(actual: &NamespaceDelegation, expected: &m::NamespaceDelegation) {
    assert_eq!(actual.namespace().as_str(), expected.namespace);
    assert_signing_key(actual.target_key(), expected.target_key.as_ref().unwrap());
    use m::namespace_delegation::Restriction as Raw;
    match expected.restriction.as_ref() {
        Some(Raw::CanSignAllMappings(_)) => assert!(matches!(
            actual.restriction(),
            Restriction::CanSignAllMappings(_)
        )),
        Some(Raw::CanSignAllButNamespaceDelegations(_)) => assert!(matches!(
            actual.restriction(),
            Restriction::CanSignAllButNamespaceDelegations(_)
        )),
        Some(Raw::CanSignSpecificMapings(raw)) => {
            let Restriction::CanSignSpecificMappings(actual) = actual.restriction() else {
                panic!("specific delegation restriction")
            };
            let mut expected = raw.mappings.clone();
            expected.sort_unstable();
            assert_eq!(
                actual
                    .mappings()
                    .iter()
                    .map(|c| c.into_proto_v30_primitive())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        None if expected.is_root_delegation => assert!(matches!(
            actual.restriction(),
            Restriction::CanSignAllMappings(_)
        )),
        None => assert!(matches!(
            actual.restriction(),
            Restriction::CanSignAllButNamespaceDelegations(_)
        )),
    }
}

fn assert_signing_key(actual: &EncodedSigningPublicKey, expected: &crypto::SigningPublicKey) {
    assert_eq!(actual.public_key(), expected.public_key);
    assert_eq!(
        crypto::CryptoKeyFormat::from(actual.format()) as i32,
        expected.format
    );
    assert_eq!(
        crypto::SigningKeySpec::from(actual.key_spec()) as i32,
        expected.key_spec
    );
    let mut actual: Vec<_> = actual
        .usage()
        .iter()
        .map(|u| crypto::SigningKeyUsage::from(*u) as i32)
        .collect();
    let mut expected = expected.usage.clone();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}
