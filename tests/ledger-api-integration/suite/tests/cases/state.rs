//! Active-contract snapshots and visibility filters.

use ledger_api_integration::integration::{Finish, Keyed, Sample};
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::{v2::*, value::v2::HasIdentifier};
use pretty_assertions::assert_eq;
use tokio_stream::StreamExt;

use crate::support::*;

async fn snapshot(
    ctx: &Context,
    offset: i64,
    format: EventFormat,
) -> Vec<ledger_api::grpc::v2::services::ActiveContractResponse> {
    let mut service = ctx.client.state();
    let stream = ok(service.get_active_contracts(offset, format, None)).await;
    tokio::pin!(stream);
    within(async {
        let mut results = Vec::new();
        while let Some(item) = stream.next().await {
            results.push(item.unwrap_or_else(|e| panic!("ACS item failed: {e:#?}")));
        }
        results
    })
    .await
}

/// Checks snapshot offsets, party/template filters, verbosity, and archival against server responses.
#[tokio::test]
async fn state_snapshots_filters_and_verbose_values() {
    let ctx = Context::new().await;
    let (sample, create, cid) = ctx.create().await;
    let offset = ok(ctx.client.state().get_ledger_end()).await;
    assert!(offset >= create.offset);
    let template =
        EventFormat::new().with_filter(ctx.owner.clone(), Filters::template(Sample::identifier()));
    for format in [
        ctx.events(true),
        ctx.events(false),
        template.with_verbose(true),
    ] {
        let results = snapshot(&ctx, offset, format.clone()).await;
        assert_eq!(results.len(), 1);
        let ContractEntry::ActiveContract(active) = &results[0].contract_entry else {
            panic!("expected active contract")
        };
        assert_eq!(
            active.created_event.contract_id.to_string(),
            cid.to_string()
        );
        assert_eq!(
            active
                .created_event
                .clone()
                .cast::<Sample>()
                .unwrap()
                .create_arguments,
            sample
        );
        assert_eq!(active.reassignment_counter, 0);
        assert_eq!(active.synchronizer_id, create.synchronizer_id);
        let mut raw = ok(
            p::state_service_client::StateServiceClient::new(ctx.channel.clone())
                .get_active_contracts(p::GetActiveContractsRequest {
                    active_at_offset: offset,
                    event_format: Some(format.into()),
                    stream_continuation_token: None,
                }),
        )
        .await
        .into_inner();
        let raw_item = ok(raw.message()).await.unwrap();
        let p::get_active_contracts_response::ContractEntry::ActiveContract(raw_active) =
            raw_item.contract_entry.unwrap()
        else {
            panic!("raw active contract")
        };
        assert_created(
            &active.created_event,
            raw_active.created_event.as_ref().unwrap(),
        );
        assert_eq!(
            results[0]
                .workflow_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            raw_item.workflow_id
        );
        assert_eq!(
            results[0]
                .stream_continuation_token
                .as_deref()
                .unwrap_or_default(),
            raw_item.stream_continuation_token
        );
    }
    let observer = EventFormat::new().with_filter(ctx.observer.clone(), Filters::wildcard());
    assert_eq!(snapshot(&ctx, offset, observer).await.len(), 1);
    let hidden = EventFormat::new().with_filter(ctx.stranger.clone(), Filters::wildcard());
    assert!(snapshot(&ctx, offset, hidden).await.is_empty());
    let wrong_template =
        EventFormat::new().with_filter(ctx.owner.clone(), Filters::template(Keyed::identifier()));
    assert!(snapshot(&ctx, offset, wrong_template).await.is_empty());
    ok(ctx
        .client
        .command()
        .submit_and_wait(ctx.commands(Finish {}.exercise(cid).erase())))
    .await;
    let end = ok(ctx.client.state().get_ledger_end()).await;
    assert!(end > offset);
    assert!(snapshot(&ctx, end, ctx.events(true)).await.is_empty());
    assert_eq!(snapshot(&ctx, offset, ctx.events(true)).await.len(), 1);
}
