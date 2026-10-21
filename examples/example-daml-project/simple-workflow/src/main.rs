use std::collections::BTreeMap;

use canton::{
    ledger_api::{
        grpc::v2::{
            client::CantonClient,
            retry::{CantonRetryPolicy, RetryConfig},
        },
        types::v2::{
            AcsDelta, AcsDeltaEvent, ArchivedEvent, ChoiceValue as _, Commands, Created,
            CreatedEvent, Event, EventFormat, TemplateValue as _, Transaction, TransactionFormat,
        },
    },
    types::{LedgerString, NonEmpty, PartyId},
};
use my_contracts::{
    daml_stdlib_da_set_types::da::set::types::Set,
    my_values::{AddValue, MyValues},
};

#[tokio::main]
async fn main() {
    // Build client
    let client = CantonClient::builder("http://localhost:9999")
        .with_retry_config(RetryConfig::new().with_canton_policy(Some(CantonRetryPolicy::auto())))
        .connect_lazy()
        .unwrap();

    let my_party = PartyId::new("someid::somens".to_string()).unwrap();

    // Create contract from MyTemplate
    let temp = MyValues {
        values: Set {
            map: BTreeMap::from([(1, ())]),
        },
        owner: my_party.clone(),
    };

    let command_id = LedgerString::new("create-cmd".to_string()).unwrap();
    let act_as = NonEmpty::single(my_party.clone());

    let mut commands = Commands::new(command_id, act_as.clone());
    commands.with_command(temp.create().erase().into());

    // This generic here allows to skip Exercise events further
    let format = TransactionFormat::<AcsDelta>::new(EventFormat::new());

    let created = client
        .command()
        .submit_and_wait_for_transaction(commands, Some(format.clone()))
        .await
        .map(created_from_tx)
        .expect("submit_and_wait_for_transaction failed");

    // Our created contract has expected values:
    let contract_data = created.create_arguments;
    assert!(contract_data.values.map.get(&1).is_some());

    // This will be the contract ID of the created contract
    let contract_id = created.contract_id;

    let add_value = AddValue { value: 2 };
    // This is a typed command for exercising this choice
    let exercise = add_value.exercise(contract_id);

    let command_id = LedgerString::new("exercise-cmd".to_string()).unwrap();

    let mut commands = Commands::new(command_id, act_as.clone());
    commands.with_command(exercise.erase().into());
    client.command().submit_and_wait(commands).await.unwrap();
}

/// Try to extract Contract ID from transaction
fn created_from_tx(
    tx: Transaction<AcsDeltaEvent<CreatedEvent, ArchivedEvent>>,
) -> Created<MyValues> {
    tx.events
        .into_iter()
        .filter_map(|event| match event {
            // Here we cast event into typed form
            Event::Created(event) => Some(event.cast::<MyValues>()),
            Event::Archived(_) => None,
            // This is needed because Event is non-exhaustive to support future updates
            t => unreachable!("unsupported event type: {t:?}"),
        })
        .next()
        .expect("created event is expected")
        .expect("expected to have a well-formed event")
}
