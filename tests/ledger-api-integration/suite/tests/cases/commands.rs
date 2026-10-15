//! Command submission and execution results.

use std::collections::BTreeSet;

use canton::types::ContractId;
use ledger_api_integration::{daml_prim_da_types::da::types::Tuple2, integration::*};
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::assert_eq;

use crate::support::*;

/// Checks submit-and-wait identifiers and created template values against the resulting transaction.
#[tokio::test]
async fn create_values_and_submit_and_wait_identifiers() {
    let ctx = Context::new().await;
    let commands = ctx.commands(ctx.sample().create().erase());
    let command_id = commands.command_id.clone();
    let workflow_id = commands.workflow_id.clone();
    let result = ok(ctx.client.command().submit_and_wait(commands)).await;
    assert!(result.completion_offset > 0);
    let tx = transaction(
        ok(ctx
            .client
            .update()
            .get_update_by_id(result.update_id.clone(), ctx.updates()))
        .await,
    );
    assert_eq!(tx.command_id, Some(command_id));
    assert_eq!(tx.workflow_id, workflow_id);
    assert_eq!(tx.offset, result.completion_offset);
    let raw = ctx.raw_transaction(&result.update_id, ctx.acs()).await;
    assert_transaction(&tx, &raw);
    assert_created(created(&tx), raw_created(&raw));
    let event = created(&tx).clone().cast::<Sample>().unwrap();
    assert_eq!(event.create_arguments, ctx.sample());
    assert_eq!(
        strings(event.signatories.iter()),
        BTreeSet::from([ctx.owner.to_string()])
    );
    assert_eq!(
        strings(event.observers.iter()),
        BTreeSet::from([ctx.observer.to_string()])
    );
    assert_eq!(
        strings(event.witness_parties.iter()),
        BTreeSet::from([ctx.owner.to_string()])
    );
    assert!(event.created_at <= tx.record_time);
}

/// Checks a non-consuming choice returns the typed payload, contract ID, and acting parties.
#[tokio::test]
async fn nonconsuming_exercise_returns_typed_payload() {
    let ctx = Context::new().await;
    let (sample, _, cid) = ctx.create().await;
    let cmds = ctx.commands(Read {}.exercise(cid.clone()).erase());
    let tx = ok(ctx
        .client
        .command()
        .submit_and_wait_for_transaction(cmds, Some(ctx.effects())))
    .await;
    let raw = ctx.raw_transaction(&tx.update_id, ctx.effects()).await;
    assert_transaction(&tx, &raw);
    let exercise = tx
        .events
        .iter()
        .find_map(|event| match event {
            Event::Exercised(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert!(!exercise.consuming);
    let typed = exercise.clone().cast::<Sample, Read>().unwrap();
    assert_eq!(typed.contract_id, cid);
    assert_eq!(typed.exercise_result, sample.payload);
    assert_eq!(typed.choice_argument, Read {});
    assert_eq!(
        strings(typed.acting_parties.iter()),
        BTreeSet::from([ctx.owner.to_string()])
    );
    let raw_exercise = raw
        .events
        .iter()
        .find_map(|event| match &event.event {
            Some(p::event::Event::Exercised(event)) => Some(event),
            _ => None,
        })
        .unwrap();
    assert_eq!(exercise.node_id, raw_exercise.node_id);
    assert_eq!(
        exercise.last_descendant_node_id,
        raw_exercise.last_descendant_node_id
    );
    assert_eq!(exercise.offset, raw_exercise.offset);
    let result: Option<p::Value> = exercise.exercise_result.clone().map(Into::into);
    assert_eq!(result, raw_exercise.exercise_result);
}

/// Checks replacement and archival events agree under ledger-effects and ACS-delta transaction shapes.
#[tokio::test]
async fn consuming_exercise_matches_both_transaction_shapes() {
    let ctx = Context::new().await;
    let (_, _, cid) = ctx.create().await;
    let next = payload(73);
    let choice = Replace {
        new_payload: next.clone(),
    };
    let cmds = ctx.commands(choice.clone().exercise(cid.clone()).erase());
    let effects = ok(ctx
        .client
        .command()
        .submit_and_wait_for_transaction(cmds, Some(ctx.effects())))
    .await;
    let exercise = effects
        .events
        .iter()
        .find_map(|event| match event {
            Event::Exercised(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert!(exercise.consuming);
    let result = exercise.clone().cast::<Sample, Replace>().unwrap();
    assert_eq!(result.contract_id, cid);
    assert_eq!(result.choice_argument, choice);
    let acs = transaction(
        ok(ctx
            .client
            .update()
            .get_update_by_id(effects.update_id.clone(), ctx.updates()))
        .await,
    );
    let new = created(&acs).clone().cast::<Sample>().unwrap();
    assert_eq!(new.contract_id, result.exercise_result);
    assert_eq!(new.create_arguments.payload, next);
    let archived = acs
        .events
        .iter()
        .find_map(|event| match event {
            Event::Archived(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert_eq!(archived.clone().cast::<Sample>().unwrap().contract_id, cid);
    assert_transaction(
        &effects,
        &ctx.raw_transaction(&effects.update_id, ctx.effects()).await,
    );
    assert_transaction(&acs, &ctx.raw_transaction(&acs.update_id, ctx.acs()).await);
}

/// Checks create-and-exercise returns matching created values, contract IDs, and choice results.
#[tokio::test]
async fn create_and_exercise_returns_created_contract_and_result() {
    let ctx = Context::new().await;
    let sample = ctx.sample();
    let cmds = ctx.commands(sample.clone().create_and_exercise(Read {}).erase());
    let tx = ok(ctx
        .client
        .command()
        .submit_and_wait_for_transaction(cmds, Some(ctx.effects())))
    .await;
    let created = tx
        .events
        .iter()
        .find_map(|event| match event {
            Event::Created(event) => Some(event),
            _ => None,
        })
        .unwrap();
    let exercised = tx
        .events
        .iter()
        .find_map(|event| match event {
            Event::Exercised(event) => Some(event),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        created.clone().cast::<Sample>().unwrap().create_arguments,
        sample
    );
    let read = exercised.clone().cast::<Sample, Read>().unwrap();
    assert_eq!(read.contract_id.into_any(), created.contract_id);
    assert_eq!(read.exercise_result, sample.payload);
    assert_transaction(
        &tx,
        &ctx.raw_transaction(&tx.update_id, ctx.effects()).await,
    );
}

/// Checks exercise-by-key results and keyed contract events before and after archival.
#[tokio::test]
async fn exercise_by_key_and_keyed_event_queries() {
    let ctx = Context::new().await;
    let value = ctx.keyed();
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(value.clone().create().erase()),
        Some(ctx.acs()),
    ))
    .await;
    let cid: ContractId<Keyed> = created(&tx).contract_id.clone().into_typed();
    let key = Tuple2 {
        _1: ctx.owner.clone(),
        _2: value.label.clone(),
    };
    let mut query = ctx.event_query();
    let before = ok(query.get_events_by_contract_id_keyed(cid.clone(), ctx.events(true))).await;
    let event = before.created.expect("keyed create");
    assert_eq!(event.create_arguments, value);
    assert_eq!(event.contract_key, key);
    assert!(!event.contract_key_hash.is_empty());
    assert!(before.archived.is_none());
    let tx = ok(ctx.client.command().submit_and_wait_for_transaction(
        ctx.commands(ReadNumber {}.exercise_by_key(key).erase()),
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
    let typed = exercise.clone().cast::<Keyed, ReadNumber>().unwrap();
    assert_eq!(typed.contract_id, cid);
    assert_eq!(typed.exercise_result, 123);
    ok(ctx
        .client
        .command()
        .submit_and_wait(ctx.commands(FinishKeyed {}.exercise(cid.clone()).erase())))
    .await;
    let after = ok(query.get_events_by_contract_id_keyed(cid.clone(), ctx.events(true))).await;
    assert_eq!(after.created.unwrap().contract_id, cid);
    assert_eq!(after.archived.unwrap().contract_id, cid);
}
