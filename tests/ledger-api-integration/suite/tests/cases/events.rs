//! Typed and untyped contract event queries.

use ledger_api_integration::integration::Finish;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks typed and untyped contract event queries match server events before and after archival.
#[tokio::test]
async fn typed_and_untyped_event_queries_before_and_after_archive() {
    let ctx = Context::new().await;
    let (sample, create, cid) = ctx.create().await;
    let mut query = ctx.event_query();
    let before = ok(query.get_events_by_contract_id(cid.clone(), ctx.events(true))).await;
    assert_eq!(before.created.unwrap().create_arguments, sample);
    assert!(before.archived.is_none());
    let untyped =
        ok(query.get_events_by_contract_id_any(cid.clone().into_any(), ctx.events(true))).await;
    assert_eq!(untyped.created.unwrap(), created(&create).clone());
    ok(ctx
        .client
        .command()
        .submit_and_wait(ctx.commands(Finish {}.exercise(cid.clone()).erase())))
    .await;
    let after = ok(query.get_events_by_contract_id(cid.clone(), ctx.events(true))).await;
    assert_eq!(after.created.unwrap().contract_id, cid);
    assert_eq!(after.archived.unwrap().contract_id, cid);
    let raw = ok(
        p::event_query_service_client::EventQueryServiceClient::new(ctx.channel.clone())
            .get_events_by_contract_id(p::GetEventsByContractIdRequest {
                contract_id: cid.to_string(),
                event_format: Some(ctx.events(true).into()),
            }),
    )
    .await
    .into_inner();
    let untyped = ok(query.get_events_by_contract_id_any(cid.into_any(), ctx.events(true))).await;
    assert_created(
        &untyped.created.unwrap(),
        raw.created
            .as_ref()
            .unwrap()
            .created_event
            .as_ref()
            .unwrap(),
    );
    let archived = untyped.archived.unwrap();
    let raw_archived = raw.archived.unwrap().archived_event.unwrap();
    assert_eq!(archived.contract_id.to_string(), raw_archived.contract_id);
    assert_eq!(archived.offset, raw_archived.offset);
    assert_eq!(archived.node_id, raw_archived.node_id);
    assert_eq!(
        strings(archived.witness_parties.iter()),
        strings(raw_archived.witness_parties.iter())
    );
}
