//! Generated value decoding and numeric serialization.

use ledger_api_integration::integration::*;
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::{v2::*, value::v2::TryFromRecord};
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks empty collections, alternate variants, and a consuming choice returning unit decode correctly.
#[tokio::test]
async fn empty_collections_variant_and_unit_choice_results_are_parsed() {
    let ctx = Context::new().await;
    let mut sample = ctx.sample();
    sample.payload.color = Color::Blue;
    sample.payload.detail = Detail::Label("alternative".into());
    sample.payload.amount = "-7.0000000000".parse().unwrap();
    sample.payload.nested = Nested {
        enabled: false,
        numbers: vec![],
        note: Some("present".into()),
    };
    sample.payload.labels = Default::default();
    sample.payload.day = canton::types::Date::from_ymd_opt(1969, 12, 31).unwrap();
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(sample.clone().create().erase()),
        Some(ctx.acs()),
    ))
    .await;
    let event = created(&tx);
    let parsed = Sample::try_from_record(event.create_arguments.clone()).unwrap_or_else(|error| {
        panic!("Failed to decode actual participant payload: {error:#?}\nResponse: {event:#?}")
    });
    assert_eq!(parsed, sample);
    let event = event.clone().cast::<Sample>().unwrap();
    assert_eq!(event.create_arguments, sample);
    let cid = event.contract_id;
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(Finish {}.exercise(cid.clone()).erase()),
        Some(ctx.effects()),
    ))
    .await;
    let exercise = tx
        .events
        .iter()
        .find_map(|event| match event {
            Event::Exercised(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert!(exercise.consuming);
    let result = exercise.clone().cast::<Sample, Finish>().unwrap();
    assert_eq!(result.contract_id, cid);
    assert_eq!(result.exercise_result, ());
}

/// Checks generated records restore a trailing Optional None omitted by Canton.
#[tokio::test]
async fn trailing_none_record_fields_are_decoded() {
    let ctx = Context::new().await;
    let mut sample = ctx.sample();
    sample.payload.nested.note = None;
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(sample.clone().create().erase()),
        Some(ctx.acs()),
    ))
    .await;
    let event = created(&tx);
    let raw = ctx.raw_transaction(&tx.update_id, ctx.acs()).await;
    assert_created(event, raw_created(&raw));
    // Canton omits trailing Optional None fields on value reads. This is a
    // valid real response, not malformed input; generated bindings must accept it.
    let parsed = Sample::try_from_record(event.create_arguments.clone())
        .unwrap_or_else(|error| panic!("SDK failed to restore the omitted trailing Optional None: {error:#?}\nRaw created event: {:?}", raw_created(&raw)));
    assert_eq!(parsed, sample);
    assert_eq!(
        event.clone().cast::<Sample>().unwrap().create_arguments,
        sample
    );
}

/// Checks small Decimals use a participant-accepted wire format and round-trip through the SDK.
#[tokio::test]
async fn small_decimal_is_accepted_by_participant_and_sdk() {
    let ctx = Context::new().await;
    let mut sample = ctx.sample();
    let decimal = "0.0000000001";
    sample.payload.amount = decimal.parse().unwrap();
    // Control: send the valid fixed-point wire value directly. This distinguishes
    // an SDK serializer failure from a participant/fixture limitation.
    let mut commands: p::Commands = ctx.commands(sample.clone().create().erase()).into();
    let Some(p::command::Command::Create(create)) = commands.commands[0].command.as_mut() else {
        unreachable!()
    };
    let payload = create
        .create_arguments
        .as_mut()
        .unwrap()
        .fields
        .iter_mut()
        .find(|field| field.label == "payload")
        .unwrap();
    let Some(p::value::Sum::Record(record)) = payload.value.as_mut().unwrap().sum.as_mut() else {
        unreachable!()
    };
    let amount = record
        .fields
        .iter_mut()
        .find(|field| field.label == "amount")
        .unwrap();
    amount.value = Some(p::Value {
        sum: Some(p::value::Sum::Numeric(decimal.into())),
    });
    let raw = ok(
        p::command_service_client::CommandServiceClient::new(ctx.channel.clone())
            .submit_and_wait_for_transaction(p::SubmitAndWaitForTransactionRequest {
                commands: Some(commands),
                transaction_format: Some(ctx.acs().into()),
            }),
    )
    .await
    .into_inner();
    assert!(
        raw.transaction.is_some(),
        "participant accepted the valid small Decimal"
    );
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(sample.clone().create().erase()),
        Some(ctx.acs()),
    ))
    .await;
    assert_eq!(
        created(&tx)
            .clone()
            .cast::<Sample>()
            .unwrap()
            .create_arguments,
        sample
    );
}
