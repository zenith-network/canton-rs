//! Ledger event extraction and comparisons with protobuf responses.

use std::collections::BTreeSet;

use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::{
    AcsDeltaEvent, ArchivedEvent, CreatedEvent, Event, Transaction, Update,
};
use pretty_assertions::assert_eq;

/// Extract a transaction, failing if another kind of update was returned.
pub fn transaction<T, R, C, P>(update: Update<T, R, C, P>) -> T {
    match update {
        Update::Transaction(tx) => tx,
        _ => panic!("expected transaction update"),
    }
}

/// Find the created event in an ACS-delta transaction.
pub fn created(tx: &Transaction<AcsDeltaEvent<CreatedEvent, ArchivedEvent>>) -> &CreatedEvent {
    tx.events
        .iter()
        .find_map(|event| match event {
            Event::Created(event) => Some(event),
            _ => None,
        })
        .expect("transaction must contain a created event")
}

/// Find the corresponding created event in a protobuf transaction.
pub fn raw_created(tx: &p::Transaction) -> &p::CreatedEvent {
    tx.events
        .iter()
        .find_map(|event| match &event.event {
            Some(p::event::Event::Created(event)) => Some(event),
            _ => None,
        })
        .expect("raw created event")
}

/// Collect string representations into a set for unordered comparisons.
pub fn strings<'a, T: ToString + 'a>(items: impl IntoIterator<Item = &'a T>) -> BTreeSet<String> {
    items.into_iter().map(ToString::to_string).collect()
}

/// Compare transaction identifiers, offsets, timestamps, and metadata with the server.
pub fn assert_transaction<E>(tx: &Transaction<E>, raw: &p::Transaction) {
    assert_eq!(tx.update_id.to_string(), raw.update_id);
    assert_eq!(tx.offset, raw.offset);
    assert_eq!(
        tx.command_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        raw.command_id
    );
    assert_eq!(
        tx.workflow_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default(),
        raw.workflow_id
    );
    assert_eq!(tx.synchronizer_id.to_string(), raw.synchronizer_id);
    assert_eq!(
        tx.effective_at,
        raw.effective_at.clone().unwrap().try_into().unwrap()
    );
    assert_eq!(
        tx.record_time,
        raw.record_time.clone().unwrap().try_into().unwrap()
    );
    assert_eq!(tx.paid_traffic_cost, raw.paid_traffic_cost);
    assert_eq!(tx.events.len().get(), raw.events.len());
}

/// Compare created-event metadata and values with the server response.
pub fn assert_created(event: &CreatedEvent, raw: &p::CreatedEvent) {
    assert_eq!(event.contract_id.to_string(), raw.contract_id);
    assert_eq!(
        event.template_id.package_id.to_string(),
        raw.template_id.as_ref().unwrap().package_id
    );
    assert_eq!(event.offset, raw.offset);
    assert_eq!(event.node_id, raw.node_id);
    assert_eq!(
        event.created_at,
        raw.created_at.clone().unwrap().try_into().unwrap()
    );
    assert_eq!(event.created_event_blob, raw.created_event_blob);
    assert_eq!(event.contract_key_hash, raw.contract_key_hash);
    assert_eq!(event.acs_delta, raw.acs_delta);
    assert_eq!(event.package_name.to_string(), raw.package_name);
    assert_eq!(
        strings(event.signatories.iter()),
        strings(raw.signatories.iter())
    );
    assert_eq!(
        strings(event.observers.iter()),
        strings(raw.observers.iter())
    );
    assert_eq!(
        strings(event.witness_parties.iter()),
        strings(raw.witness_parties.iter())
    );
    let encoded: p::Record = event.create_arguments.clone().into();
    assert_eq!(Some(encoded), raw.create_arguments);
    let key: Option<p::Value> = event.contract_key.clone().map(Into::into);
    assert_eq!(key, raw.contract_key);
}
