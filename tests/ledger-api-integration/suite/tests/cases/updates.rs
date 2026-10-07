//! Update lookup, bounded streams, and pagination.

use std::collections::BTreeSet;

use ledger_api_integration::integration::Finish;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::assert_eq;
use tokio_stream::StreamExt;

use crate::support::*;

/// Checks lookup by ID and offset agrees with an ordered, bounded update stream.
#[tokio::test]
async fn updates_by_id_offset_and_bounded_stream_are_consistent() {
    let ctx = Context::new().await;
    let begin = ok(ctx.client.state().get_ledger_end()).await;
    let (_, first, cid) = ctx.create().await;
    let second = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(Finish {}.exercise(cid).erase()),
        Some(ctx.acs()),
    ))
    .await;
    let expected = [first, second];
    for tx in &expected {
        let by_id = transaction(
            ok(ctx
                .client
                .update()
                .get_update_by_id(tx.update_id.clone(), ctx.updates()))
            .await,
        );
        let by_offset = transaction(
            ok(ctx
                .client
                .update()
                .get_update_by_offset(tx.offset, ctx.updates()))
            .await,
        );
        assert_eq!(by_id.update_id, tx.update_id);
        assert_eq!(by_offset.update_id, tx.update_id);
        let raw = ctx.raw_transaction(&tx.update_id, ctx.acs()).await;
        assert_transaction(&by_id, &raw);
        assert_transaction(&by_offset, &raw);
    }
    let mut service = ctx.client.update();
    let stream = ok(service.get_updates(begin, Some(expected[1].offset), ctx.updates())).await;
    tokio::pin!(stream);
    let received = within(async {
        let mut transactions = Vec::new();
        while let Some(item) = stream.next().await {
            match item.unwrap_or_else(|e| panic!("update item failed: {e:#?}")) {
                Update::Transaction(tx) => transactions.push(tx),
                Update::OffsetCheckpoint(checkpoint) => assert!(checkpoint.offset >= begin),
                _ => panic!("unexpected non-transaction update"),
            }
        }
        transactions
    })
    .await;
    assert_eq!(
        received
            .iter()
            .map(|tx| tx.update_id.clone())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|tx| tx.update_id.clone())
            .collect::<Vec<_>>()
    );
}

/// Checks small pages and continuation tokens return every transaction in order without duplicates.
#[tokio::test]
async fn update_pages_continue_without_missing_or_duplicate_transactions() {
    let ctx = Context::new().await;
    let begin = ok(ctx.client.state().get_ledger_end()).await;
    let mut expected = Vec::new();
    for _ in 0..3 {
        expected.push(ctx.create().await.1);
    }
    let end = expected.last().unwrap().offset;
    let mut token = None;
    let mut seen_tokens = BTreeSet::new();
    let mut actual = Vec::new();
    for _ in 0..10 {
        let raw_token = token
            .as_ref()
            .map(|token: &PageToken| token.as_ref().to_vec());
        let raw = ok(
            p::update_service_client::UpdateServiceClient::new(ctx.channel.clone())
                .get_updates_page(p::GetUpdatesPageRequest {
                    begin_offset_exclusive: Some(begin),
                    end_offset_inclusive: Some(end),
                    max_page_size: Some(1),
                    update_format: Some(ctx.updates().into()),
                    page_token: raw_token,
                    descending_order: false,
                }),
        )
        .await
        .into_inner();
        let page = ok(ctx.client.update().get_updates_page(
            Some(begin),
            Some(end),
            Some(1),
            ctx.updates(),
            token,
        ))
        .await;
        assert_eq!(
            page.lowest_page_offset_exclusive,
            raw.lowest_page_offset_exclusive
        );
        assert_eq!(
            page.highest_page_offset_inclusive,
            raw.highest_page_offset_inclusive
        );
        assert_eq!(page.items.len(), raw.updates.len());
        assert!(page.items.len() <= 1);
        for (item, raw_item) in page.items.into_iter().zip(raw.updates) {
            let tx = transaction(item);
            let Some(p::get_update_response::Update::Transaction(raw_tx)) = raw_item.update else {
                panic!("raw page transaction")
            };
            assert_transaction(&tx, &raw_tx);
            actual.push(tx.update_id);
        }
        token = page.next_page_token;
        if let Some(ref next) = token {
            assert!(
                seen_tokens.insert(next.as_ref().to_vec()),
                "pagination token repeated"
            );
        } else {
            break;
        }
    }
    assert!(token.is_none(), "pagination did not terminate");
    assert!(
        !seen_tokens.is_empty(),
        "small pages must exercise continuation"
    );
    assert_eq!(
        actual,
        expected
            .into_iter()
            .map(|tx| tx.update_id)
            .collect::<Vec<_>>()
    );
}
