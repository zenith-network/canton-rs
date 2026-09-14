//! Structured RPC, Daml, and streaming failures.

use canton::types::PackageId;
use ledger_api::grpc::v2::error::{CantonError, CategoryId, ErrorCodeId};
use ledger_api_integration::integration::{Finish, Read, Reject};
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::assert_eq;
use tokio_stream::StreamExt;
use tonic_types::StatusExt;

use crate::support::*;

/// Checks an unknown package produces the same decoded error code and details as the server.
#[tokio::test]
async fn unknown_package_error_matches_real_error_info() {
    let ctx = Context::new().await;
    let package = PackageId::new("0".repeat(64)).unwrap();
    let raw = within(
        p::package_service_client::PackageServiceClient::new(ctx.channel.clone()).get_package(
            p::GetPackageRequest {
                package_id: package.to_string(),
            },
        ),
    )
    .await
    .unwrap_err();
    let error = expect_decoded(
        within(ctx.client.package().get_package(package))
            .await
            .unwrap_err(),
    );
    assert_error(&error, &raw, "PACKAGE_NOT_FOUND");
    assert_eq!(error.error_code_id(), &ErrorCodeId::PackageNotFound);
}

/// Checks an unknown update produces the same decoded error code and details as the server.
#[tokio::test]
async fn unknown_update_error_matches_real_error_info() {
    let ctx = Context::new().await;
    // Match the participant's own ID encoding; an arbitrary LedgerString is
    // not necessarily a valid Canton update ID.
    let (_, tx, _) = ctx.create().await;
    let mut bytes = tx.update_id.to_string().into_bytes();
    let last = bytes.last_mut().unwrap();
    *last = if *last == b'0' { b'1' } else { b'0' };
    let unknown = canton::types::LedgerString::new(String::from_utf8(bytes).unwrap()).unwrap();
    let raw = within(
        p::update_service_client::UpdateServiceClient::new(ctx.channel.clone()).get_update_by_id(
            p::GetUpdateByIdRequest {
                update_id: unknown.to_string(),
                update_format: Some(ctx.updates().into()),
            },
        ),
    )
    .await
    .unwrap_err();
    let error = expect_decoded(
        within(ctx.client.update().get_update_by_id(unknown, ctx.updates()))
            .await
            .unwrap_err(),
    );
    assert_error(&error, &raw, "UPDATE_NOT_FOUND");
    assert_eq!(error.error_code_id(), &ErrorCodeId::UpdateNotFound);
}

/// Checks a contract hidden by party filters produces the same structured error as the server.
#[tokio::test]
async fn filtered_contract_event_error_matches_real_error_info() {
    let ctx = Context::new().await;
    let (_, _, cid) = ctx.create().await;
    let format = EventFormat::new().with_filter(ctx.stranger.clone(), Filters::wildcard());
    let raw = within(
        p::event_query_service_client::EventQueryServiceClient::new(ctx.channel.clone())
            .get_events_by_contract_id(p::GetEventsByContractIdRequest {
                contract_id: cid.to_string(),
                event_format: Some(format.clone().into()),
            }),
    )
    .await
    .unwrap_err();
    let error = expect_decoded(
        within(
            ctx.event_query()
                .get_events_by_contract_id_any(cid.into_any(), format),
        )
        .await
        .unwrap_err(),
    );
    assert_error(&error, &raw, "CONTRACT_EVENTS_NOT_FOUND");
}

/// Checks duplicate-command errors preserve server details, completion offsets, and submission IDs.
#[tokio::test]
async fn duplicate_command_error_matches_real_error_info() {
    let ctx = Context::new().await;
    let cmds = ctx.commands(ctx.sample().create().erase());
    ok(ctx.client.command().submit_and_wait(cmds.clone())).await;
    let mut duplicate = cmds;
    duplicate.with_random_submission_id();
    let raw = within(
        p::command_service_client::CommandServiceClient::new(ctx.channel.clone()).submit_and_wait(
            p::SubmitAndWaitRequest {
                commands: Some(duplicate.clone().into()),
            },
        ),
    )
    .await
    .unwrap_err();
    duplicate.with_random_submission_id();
    let error = expect_decoded(
        within(ctx.client.command().submit_and_wait(duplicate))
            .await
            .unwrap_err(),
    );
    assert_error(&error, &raw, "DUPLICATE_COMMAND");
    assert_eq!(error.error_code_id(), &ErrorCodeId::DuplicateCommand);
    let info = raw.get_details_error_info().unwrap();
    assert_eq!(
        error.completion_offset(),
        info.metadata
            .get("completion_offset")
            .map(|v| v.parse().unwrap())
    );
    assert_eq!(
        error
            .existing_submission_id()
            .as_ref()
            .map(ToString::to_string),
        info.metadata.get("existing_submission_id").cloned()
    );
}

/// Checks exercising an archived contract produces the same structured error as the server.
#[tokio::test]
async fn archived_contract_exercise_error_matches_real_error_info() {
    let ctx = Context::new().await;
    let (_, _, cid) = ctx.create().await;
    ok(ctx
        .client
        .command()
        .submit_and_wait(ctx.commands(Finish {}.exercise(cid.clone()).erase())))
    .await;
    let raw_cmds = ctx.commands(Read {}.exercise(cid.clone()).erase());
    let raw = within(
        p::command_service_client::CommandServiceClient::new(ctx.channel.clone()).submit_and_wait(
            p::SubmitAndWaitRequest {
                commands: Some(raw_cmds.into()),
            },
        ),
    )
    .await
    .unwrap_err();
    let cmds = ctx.commands(Read {}.exercise(cid).erase());
    let error = expect_decoded(
        within(ctx.client.command().submit_and_wait(cmds))
            .await
            .unwrap_err(),
    );
    assert_error(&error, &raw, "CONTRACT_NOT_FOUND");
}

async fn structured_daml_failure(state_dependent: bool) {
    let ctx = Context::new().await;
    let (_, _, cid) = ctx.create().await;
    let command = || ctx.commands(Reject { state_dependent }.exercise(cid.clone()).erase());
    let raw = within(
        p::command_service_client::CommandServiceClient::new(ctx.channel.clone()).submit_and_wait(
            p::SubmitAndWaitRequest {
                commands: Some(command().into()),
            },
        ),
    )
    .await
    .unwrap_err();
    let error = within(ctx.client.command().submit_and_wait(command()))
        .await
        .unwrap_err();
    let CantonError::Daml(error) = error else {
        panic!("expected Daml failure: {error:#?}")
    };
    let info = raw
        .get_details_error_info()
        .expect("DAML_FAILURE ErrorInfo");
    assert_eq!(info.reason, "DAML_FAILURE");
    assert_eq!(error.code(), raw.code());
    assert_eq!(
        error.category_id(),
        if state_dependent {
            CategoryId::InvalidGivenCurrentSystemStateOther
        } else {
            CategoryId::InvalidIndependentOfSystemState
        }
    );
    assert_eq!(
        i32::from(error.category_id()),
        info.metadata["category"].parse::<i32>().unwrap()
    );
    assert_eq!(error.error_id(), Some("integration.canton-rs/rejected"));
    assert_eq!(
        error.metadata().get("fixture").map(String::as_str),
        Some("Sample")
    );
    assert_eq!(
        error.metadata().get("detail").map(String::as_str),
        Some("controlled")
    );
    assert_eq!(info.metadata["error_id"], error.error_id().unwrap());
    assert!(error.full_message().starts_with("DAML_FAILURE"));
    assert_eq!(
        error.failure_message(),
        Some("Controlled rejection: payload is unacceptable"),
        "Daml failure message must recover the fixture's controlled message. Raw status: {raw:#?}"
    );
    assert_eq!(
        error.human_message(),
        "Controlled rejection: payload is unacceptable"
    );
}

/// Checks a state-independent Daml failure preserves its category, metadata, and controlled message.
#[tokio::test]
async fn state_independent_daml_failure_is_parsed() {
    structured_daml_failure(false).await;
}

/// Checks a state-dependent Daml failure preserves its category, metadata, and controlled message.
#[tokio::test]
async fn state_dependent_daml_failure_is_parsed() {
    structured_daml_failure(true).await;
}

/// Checks errors received while consuming an update stream retain the server's structured details.
#[tokio::test]
async fn streaming_error_is_decoded_when_the_stream_is_consumed() {
    let ctx = Context::new().await;
    // Validly typed request, but its offset is beyond this participant's ledger.
    let offset = ok(ctx.client.state().get_ledger_end()).await + 1_000_000;
    let request = p::GetUpdatesRequest {
        begin_exclusive: offset,
        end_inclusive: Some(offset + 1),
        update_format: Some(ctx.updates().into()),
        descending_order: false,
    };
    let raw = match within(
        p::update_service_client::UpdateServiceClient::new(ctx.channel.clone())
            .get_updates(request),
    )
    .await
    {
        Err(status) => status,
        Ok(response) => within(response.into_inner().message())
            .await
            .expect_err("expected streaming error"),
    };
    let mut service = ctx.client.update();
    let error = match within(service.get_updates(offset, Some(offset + 1), ctx.updates())).await {
        Err(error) => error,
        Ok(stream) => {
            tokio::pin!(stream);
            within(stream.next())
                .await
                .expect("stream must emit rejection")
                .expect_err("expected rejected stream item")
        }
    };
    let reason = raw.get_details_error_info().unwrap().reason;
    assert_eq!(reason, "OFFSET_AFTER_LEDGER_END");
    assert_error(&expect_decoded(error), &raw, &reason);
}
