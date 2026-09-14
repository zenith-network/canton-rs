//! Party allocation, participant identity, discovery, pagination, and errors.

use a::party_management_service_client::PartyManagementServiceClient as RawPartyClient;
use canton::types::{PartyId, SynchronizerId};
use ledger_api::grpc::v2::services::admin::AllocatePartyRequest;
use ledger_api_proto::com::daml::ledger::api::v2::{self as p, admin as a};
use ledger_api_types::v2::PartyDetails;
use pretty_assertions::{assert_eq, assert_ne};

use crate::support::*;

/// Compare supported fields directly with the independent protobuf response.
fn assert_details(actual: &PartyDetails, expected: &a::PartyDetails) {
    assert_eq!(actual.party.as_str(), expected.party);
    assert_eq!(actual.is_local, expected.is_local);
    assert_eq!(
        actual.identity_provider_id.as_deref(),
        (!expected.identity_provider_id.is_empty())
            .then_some(expected.identity_provider_id.as_str())
    );
}

/// Observe allocated parties without using the SDK's response conversions.
async fn raw_parties(ctx: &Context, parties: &[PartyId]) -> Vec<a::PartyDetails> {
    ok(
        RawPartyClient::new(ctx.channel.clone()).get_parties(a::GetPartiesRequest {
            parties: parties.iter().map(ToString::to_string).collect(),
            ..Default::default()
        }),
    )
    .await
    .into_inner()
    .party_details
}

/// Build an absent party with a fresh name and the participant's real namespace.
fn unknown_party(ctx: &Context) -> PartyId {
    let (_, namespace) = ctx.owner.rsplit_once("::").expect("Canton party namespace");
    PartyId::new(format!("{}::{namespace}", id("unknown"))).unwrap()
}

/// Checks participant IDs agree with the server and remain stable across client handles.
#[tokio::test]
async fn participant_id_matches_server() {
    let ctx = Context::new().await;
    let raw =
        ok(RawPartyClient::new(ctx.channel.clone())
            .get_participant_id(a::GetParticipantIdRequest {}))
        .await
        .into_inner();
    for _ in 0..2 {
        let actual = ok(ctx.client.party_management().get_participant_id()).await;
        assert_eq!(actual.as_str(), raw.participant_id);
    }
}

/// Checks allocation with and without a hint returns local parties matching server details.
#[tokio::test]
async fn allocation_returns_observable_party_details() {
    let ctx = Context::new().await;
    let hint = PartyId::new(id("allocated").to_string()).unwrap();
    let mut allocated = Vec::new();
    for request in [
        AllocatePartyRequest::with_hint(hint),
        AllocatePartyRequest::default(),
    ] {
        let details = ok(ctx.client.party_management().allocate_party(request)).await;
        assert!(details.is_local);
        assert_eq!(details.identity_provider_id, None);
        let raw = raw_parties(&ctx, std::slice::from_ref(&details.party)).await;
        assert_eq!(raw.len(), 1);
        assert_details(&details, &raw[0]);
        allocated.push(details.party);
    }
    assert_ne!(allocated[0], allocated[1]);
}

/// Checks explicit synchronizer and user fields allocate a party and grant the user's act-as right.
#[tokio::test]
async fn allocation_on_synchronizer_grants_user_rights() {
    let ctx = Context::new().await;
    let connected = ok(
        p::state_service_client::StateServiceClient::new(ctx.channel.clone())
            .get_connected_synchronizers(p::GetConnectedSynchronizersRequest {
                party: ctx.owner.to_string(),
                ..Default::default()
            }),
    )
    .await
    .into_inner();
    assert_eq!(connected.connected_synchronizers.len(), 1);
    let synchronizer = &connected.connected_synchronizers[0].synchronizer_id;
    let details = ok(ctx
        .client
        .party_management()
        .allocate_party(AllocatePartyRequest {
            party_id_hint: Some(PartyId::new(id("user-party").to_string()).unwrap()),
            synchronizer_id: Some(SynchronizerId::new(synchronizer.clone()).unwrap()),
            user_id: Some(ctx.user.clone()),
            ..Default::default()
        }))
    .await;
    let raw = raw_parties(&ctx, std::slice::from_ref(&details.party)).await;
    assert_eq!(raw.len(), 1);
    assert_details(&details, &raw[0]);
    let hosted = ok(
        p::state_service_client::StateServiceClient::new(ctx.channel.clone())
            .get_connected_synchronizers(p::GetConnectedSynchronizersRequest {
                party: details.party.to_string(),
                ..Default::default()
            }),
    )
    .await
    .into_inner();
    assert!(
        hosted
            .connected_synchronizers
            .iter()
            .any(|s| &s.synchronizer_id == synchronizer)
    );
    let rights = ok(
        a::user_management_service_client::UserManagementServiceClient::new(ctx.channel.clone())
            .list_user_rights(a::ListUserRightsRequest {
                user_id: ctx.user.to_string(),
                ..Default::default()
            }),
    )
    .await
    .into_inner();
    assert!(rights.rights.iter().any(|right| matches!(
        &right.kind,
        Some(a::right::Kind::CanActAs(right)) if right.party == details.party.as_str()
    )));
}

/// Checks lookup returns all requested known parties and omits an unknown party.
#[tokio::test]
async fn lookup_returns_only_known_parties() {
    let ctx = Context::new().await;
    let requested = vec![ctx.observer.clone(), unknown_party(&ctx), ctx.owner.clone()];
    let mut raw = raw_parties(&ctx, &requested).await;
    assert_eq!(
        strings(raw.iter().map(|p| &p.party)),
        strings([&ctx.owner, &ctx.observer])
    );
    let mut actual = ok(ctx.client.party_management().get_parties(requested, None)).await;
    // GetParties explicitly does not promise response ordering.
    raw.sort_by(|a, b| a.party.cmp(&b.party));
    actual.sort_by(|a, b| a.party.cmp(&b.party));
    assert_eq!(actual.len(), raw.len());
    for (actual, raw) in actual.iter().zip(&raw) {
        assert_details(actual, raw);
    }
}

/// Checks an unknown-only lookup preserves the server's successful empty result.
#[tokio::test]
async fn lookup_unknown_party_returns_empty_result() {
    let ctx = Context::new().await;
    let party = unknown_party(&ctx);
    let raw = raw_parties(&ctx, std::slice::from_ref(&party)).await;
    assert!(raw.is_empty(), "raw response: {raw:#?}");
    let actual = within(ctx.client.party_management().get_parties(vec![party], None))
        .await
        .unwrap_or_else(|error| {
            panic!("SDK rejected successful empty lookup: {error:#?}\nraw response: {raw:#?}")
        });
    assert_eq!(actual, Vec::<PartyDetails>::new());
}

/// Checks an empty lookup request succeeds and returns the same empty result as the server.
#[tokio::test]
async fn lookup_empty_request_returns_empty_result() {
    let ctx = Context::new().await;
    let raw = raw_parties(&ctx, &[]).await;
    assert!(raw.is_empty(), "raw response: {raw:#?}");
    let actual = within(ctx.client.party_management().get_parties(Vec::new(), None))
        .await
        .unwrap_or_else(|error| {
            panic!("SDK rejected empty lookup request: {error:#?}\nraw response: {raw:#?}")
        });
    assert_eq!(actual, Vec::<PartyDetails>::new());
}

/// Checks default listing includes setup parties and an exact-ID filter returns one matching party.
#[tokio::test]
async fn listing_defaults_and_exact_filter_match_server() {
    let ctx = Context::new().await;
    for filter in [None, Some(ctx.owner.to_string())] {
        let raw = ok(RawPartyClient::new(ctx.channel.clone()).list_known_parties(
            a::ListKnownPartiesRequest {
                filter_party: filter.clone().unwrap_or_default(),
                ..Default::default()
            },
        ))
        .await
        .into_inner();
        let actual =
            ok(ctx
                .client
                .party_management()
                .list_known_parties(None, None, None, filter.clone()))
            .await;
        assert_eq!(
            actual.next_page_token.as_deref().unwrap_or_default(),
            raw.next_page_token
        );
        assert_eq!(actual.party_details.len(), raw.party_details.len());
        for (actual, raw) in actual.party_details.iter().zip(&raw.party_details) {
            assert_details(actual, raw);
        }
        if filter.is_some() {
            assert_eq!(actual.party_details.len(), 1);
            assert_eq!(actual.party_details[0].party, ctx.owner);
        } else {
            for party in [&ctx.owner, &ctx.observer, &ctx.stranger] {
                assert!(
                    actual
                        .party_details
                        .iter()
                        .any(|details| &details.party == party)
                );
            }
        }
    }
}

/// Checks small filtered pages preserve tokens, ordering, and every allocated party without duplicates.
#[tokio::test]
async fn filtered_pages_are_complete_and_ordered() {
    let ctx = Context::new().await;
    let prefix = id("page").to_string();
    let mut expected = Vec::new();
    for suffix in ["a", "b", "c"] {
        let hint = PartyId::new(format!("{prefix}-{suffix}")).unwrap();
        expected.push(
            ok(ctx
                .client
                .party_management()
                .allocate_party(AllocatePartyRequest::with_hint(hint)))
            .await
            .party,
        );
    }
    expected.sort();
    let mut token = None;
    let mut seen = Vec::new();
    let mut pages = 0;
    within(async {
        loop {
            let raw = ok(RawPartyClient::new(ctx.channel.clone()).list_known_parties(
                a::ListKnownPartiesRequest {
                    page_token: token.clone().unwrap_or_default(),
                    page_size: 2,
                    filter_party: prefix.clone(),
                    ..Default::default()
                },
            ))
            .await
            .into_inner();
            let actual = ok(ctx.client.party_management().list_known_parties(
                token.clone(),
                Some(2),
                None,
                Some(prefix.clone()),
            ))
            .await;
            assert_eq!(
                actual.next_page_token.as_deref().unwrap_or_default(),
                raw.next_page_token
            );
            assert_eq!(actual.party_details.len(), raw.party_details.len());
            assert!(actual.party_details.len() <= 2);
            for (details, raw) in actual.party_details.iter().zip(&raw.party_details) {
                assert_details(details, raw);
                assert!(details.party.starts_with(&prefix));
                seen.push(details.party.clone());
            }
            pages += 1;
            token = actual.next_page_token;
            if token.is_none() {
                break;
            }
            assert!(pages < 4, "pagination did not terminate");
        }
    })
    .await;
    assert_eq!(pages, 2);
    assert_eq!(seen, expected);
}

/// Checks a filter with no matches preserves the server's successful empty party list.
#[tokio::test]
async fn listing_unmatched_filter_returns_empty_result() {
    let ctx = Context::new().await;
    let filter = id("absent").to_string();
    let raw = ok(RawPartyClient::new(ctx.channel.clone()).list_known_parties(
        a::ListKnownPartiesRequest {
            filter_party: filter.clone(),
            ..Default::default()
        },
    ))
    .await
    .into_inner();
    assert!(raw.party_details.is_empty(), "raw response: {raw:#?}");
    assert!(raw.next_page_token.is_empty());
    let actual = within(ctx.client.party_management().list_known_parties(
        None,
        None,
        None,
        Some(filter),
    ))
    .await
    .unwrap_or_else(|error| {
        panic!("SDK rejected successful empty listing: {error:#?}\nraw response: {raw:#?}")
    });
    assert_eq!(actual.party_details, Vec::<PartyDetails>::new());
    assert_eq!(actual.next_page_token, None);
}

/// Checks continuation after a full final page preserves the server's empty terminal page.
#[tokio::test]
async fn listing_full_page_continuation_returns_empty_result() {
    let ctx = Context::new().await;
    let filter = ctx.owner.to_string();
    let first = ok(ctx.client.party_management().list_known_parties(
        None,
        Some(1),
        None,
        Some(filter.clone()),
    ))
    .await;
    assert_eq!(first.party_details.len(), 1);
    assert_eq!(first.party_details[0].party, ctx.owner);
    let token = first
        .next_page_token
        .expect("full page must expose continuation token");
    let raw = ok(RawPartyClient::new(ctx.channel.clone()).list_known_parties(
        a::ListKnownPartiesRequest {
            page_token: token.clone(),
            page_size: 1,
            filter_party: filter.clone(),
            ..Default::default()
        },
    ))
    .await
    .into_inner();
    assert!(raw.party_details.is_empty(), "raw response: {raw:#?}");
    assert!(raw.next_page_token.is_empty());
    let actual = within(ctx.client.party_management().list_known_parties(
        Some(token),
        Some(1),
        None,
        Some(filter),
    ))
    .await
    .unwrap_or_else(|error| {
        panic!("SDK rejected successful terminal page: {error:#?}\nraw response: {raw:#?}")
    });
    assert_eq!(actual.party_details, Vec::<PartyDetails>::new());
    assert_eq!(actual.next_page_token, None);
}

/// Checks exceeding the advertised page limit preserves the server's structured error and details.
#[tokio::test]
async fn listing_page_limit_error_matches_server() {
    let ctx = Context::new().await;
    let version = ok(ctx.client.version().get_ledger_api_version()).await;
    let size = version
        .features
        .party_management
        .max_parties_page_size
        .checked_add(1)
        .unwrap();
    let raw = within(RawPartyClient::new(ctx.channel.clone()).list_known_parties(
        a::ListKnownPartiesRequest {
            page_size: size,
            ..Default::default()
        },
    ))
    .await
    .unwrap_err();
    let error = expect_decoded(
        within(
            ctx.client
                .party_management()
                .list_known_parties(None, Some(size), None, None),
        )
        .await
        .unwrap_err(),
    );
    assert_error(&error, &raw, "INVALID_ARGUMENT");
}
