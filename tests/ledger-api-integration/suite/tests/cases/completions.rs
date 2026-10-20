//! Asynchronous submissions and completion responses.

use canton::types::NonEmpty;
use ledger_api::grpc::v2::{error::CantonError, services::CompletionResponse};
use ledger_api_integration::integration::Sample;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::{assert_eq, assert_ne};
use tokio_stream::StreamExt;

use crate::support::*;

/// Checks successful submissions produce matching completion identifiers and ledger effects.
#[tokio::test]
async fn submission_and_successful_completion_match_raw_stream() {
    let ctx = Context::new().await;
    let begin = ok(ctx.client.state().get_ledger_end()).await;
    let cmds = ctx.commands(ctx.sample().create().erase());
    let mut completion = ctx.completion();
    let mut raw_service = p::command_completion_service_client::CommandCompletionServiceClient::new(
        ctx.channel.clone(),
    );
    // Canton may defer response headers until the first completion/checkpoint.
    // Start polling both subscription RPCs first, then submit concurrently;
    // replay from the captured offset also protects against registration races.
    let (stream, raw, ()) = within(async {
        tokio::join!(biased;
            completion.completion_stream(Some(ctx.user.clone()), NonEmpty::single(ctx.owner.clone()), begin),
            raw_service.completion_stream(p::CompletionStreamRequest {
                user_id: ctx.user.to_string(), parties: vec![ctx.owner.to_string()], begin_exclusive: begin,
            }),
            async {
                tokio::task::yield_now().await;
                ok(ctx.submission().submit(cmds.clone())).await;
            },
        )
    }).await;
    let stream = stream.unwrap_or_else(|error| panic!("SDK subscription failed: {error:#?}"));
    tokio::pin!(stream);
    let mut raw = raw.expect("raw completion subscription").into_inner();
    let command_id = cmds.command_id.to_string();
    let (parsed, raw) = tokio::join!(
        matching_sdk_completion(&mut stream, &command_id),
        matching_raw_completion(&mut raw, &command_id),
    );
    assert_eq!(raw.status.as_ref().unwrap().code, tonic::Code::Ok as i32);
    assert_completion(&parsed, &raw);
    assert!(parsed.update_id.is_some());
    assert!(parsed.offset > begin);
    let tx = transaction(
        ok(ctx
            .client
            .update()
            .get_update_by_id(parsed.update_id.unwrap(), ctx.updates()))
        .await,
    );
    assert_eq!(tx.offset, parsed.offset);
    assert_eq!(
        created(&tx)
            .clone()
            .cast::<Sample>()
            .unwrap()
            .create_arguments,
        ctx.sample()
    );
}

async fn matching_raw_completion(
    stream: &mut tonic::Streaming<p::CompletionStreamResponse>,
    command: &str,
) -> p::Completion {
    within(async {
        loop {
            let item = stream
                .message()
                .await
                .expect("raw completion stream error")
                .expect("raw completion stream ended");
            if let Some(p::completion_stream_response::CompletionResponse::Completion(value)) =
                item.completion_response
            {
                eprintln!(
                    "Raw completion: command={} submission={} status={:?}",
                    value.command_id,
                    value.submission_id,
                    value
                        .status
                        .as_ref()
                        .map(|status| (status.code, &status.message))
                );
                if value.command_id == command {
                    return value;
                }
            }
        }
    })
    .await
}

async fn matching_sdk_completion<S>(stream: &mut S, command: &str) -> Completion
where
    S: tokio_stream::Stream<Item = Result<CompletionResponse, CantonError>> + Unpin,
{
    within(async {
        loop {
            let item = stream
                .next()
                .await
                .expect("completion stream ended")
                .unwrap_or_else(|error| panic!("completion item failed: {error:#?}"));
            if let CompletionResponse::Completion(value) = item {
                eprintln!(
                    "SDK completion: command={} submission={:?}",
                    value.command_id, value.submission_id
                );
                if value.command_id.to_string() == command {
                    return value;
                }
            }
        }
    })
    .await
}

fn assert_completion(parsed: &Completion, raw: &p::Completion) {
    assert_eq!(parsed.command_id.to_string(), raw.command_id);
    assert_eq!(parsed.user_id.to_string(), raw.user_id);
    assert_eq!(
        parsed
            .update_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        raw.update_id
    );
    assert_eq!(
        parsed
            .submission_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        raw.submission_id
    );
    assert_eq!(strings(parsed.act_as.iter()), strings(raw.act_as.iter()));
    assert_eq!(parsed.offset, raw.offset);
    assert_eq!(parsed.paid_traffic_cost, raw.paid_traffic_cost);
    assert_eq!(parsed.status.is_some(), raw.status.is_some());
}

/// Checks rejected completions preserve the server rejection code, message, and structured details.
#[tokio::test]
async fn rejected_completion_preserves_server_error_details() {
    let ctx = Context::new().await;
    // A duplicate of a committed command is rejected asynchronously by Submit.
    let cmds = ctx.commands(ctx.sample().create().erase());
    ok(ctx.client.command().submit_and_wait(cmds.clone())).await;
    let begin = ok(ctx.client.state().get_ledger_end()).await;
    let mut duplicate = cmds.clone();
    duplicate.with_random_submission_id();
    let mut completion = ctx.completion();
    let mut raw_service = p::command_completion_service_client::CommandCompletionServiceClient::new(
        ctx.channel.clone(),
    );
    let (stream, raw, ()) = within(async {
        tokio::join!(biased;
            completion.completion_stream(Some(ctx.user.clone()), NonEmpty::single(ctx.owner.clone()), begin),
            raw_service.completion_stream(p::CompletionStreamRequest {
                user_id: ctx.user.to_string(), parties: vec![ctx.owner.to_string()], begin_exclusive: begin,
            }),
            async {
                tokio::task::yield_now().await;
                ok(ctx.submission().submit(duplicate.clone())).await;
            },
        )
    }).await;
    let stream = stream.unwrap_or_else(|error| panic!("SDK subscription failed: {error:#?}"));
    tokio::pin!(stream);
    let mut raw = raw.expect("raw completion subscription").into_inner();
    let command_id = cmds.command_id.to_string();
    let (parsed, raw) = tokio::join!(
        matching_sdk_completion(&mut stream, &command_id),
        matching_raw_completion(&mut raw, &command_id),
    );
    assert_completion(&parsed, &raw);
    assert!(parsed.update_id.is_none());
    let status = raw.status.as_ref().expect("rejection status");
    assert_eq!(status.code, tonic::Code::AlreadyExists as i32);
    assert!(status.message.starts_with("DUPLICATE_COMMAND"));
    assert!(!status.details.is_empty());
    // The real response above establishes that details exist. The SDK exposes
    // only (), so this is an active failing integration assertion, not a skip
    // or a fabricated malformed-input test. Keep it until the SDK is fixed separately.
    let sdk_status = parsed.status.as_ref().expect("SDK rejection status");
    assert_ne!(
        std::any::type_name_of_val(sdk_status),
        "()",
        "SDK discarded the rejection code, message, and details.\nRaw completion: {raw:?}\nSDK completion: {parsed:#?}"
    );
}
