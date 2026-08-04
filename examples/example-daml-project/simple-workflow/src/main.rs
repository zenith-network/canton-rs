use std::collections::BTreeMap;

use canton::{
    ledger_api::{
        grpc::v2::{client::CantonClient, error::CantonError},
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
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // Build client
    let client = CantonClient::builder("http://localhost:9999")
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

    let commands =
        Commands::new(command_id, act_as.clone()).with_command(temp.create().erase().into());

    // This generic here allows to skip Exercise events futher
    let format = TransactionFormat::<AcsDelta>::new(EventFormat::new());

    let result = client
        .command()
        .submit_and_wait_for_transaction(commands.clone(), Some(format.clone()))
        .await
        .map(created_from_tx);

    // this event represents the created contract
    let created = match result {
        Ok(cid) => cid,
        // On error we can retry
        Err(error) => match &error {
            CantonError::CantonGrpc(canton_grpc_error) => {
                if let Some(delay) = canton_grpc_error.category_id.retry() {
                    sleep(delay).await;
                    client
                        .command()
                        .submit_and_wait_for_transaction(commands, Some(format.clone()))
                        .await
                        .map(created_from_tx)
                        .expect("expected retry to succeed")
                } else {
                    panic!("{error}");
                }
            }
            CantonError::Raw(status) => panic!("unparsed error, status: {status:?}"),
            CantonError::ValueError(_) => panic!("{error}"),
        },
    };

    // Our created contract has expected values:
    let contract_data = created.create_arguments;
    assert!(contract_data.values.map.get(&1).is_some());

    // This will be the contract ID of the created contract
    let contract_id = created.contract_id;

    let add_value = AddValue { value: 2 };
    // This is a typed command for exercising this choice
    let exercise = add_value.exercise(contract_id);

    let command_id = LedgerString::new("exercise-cmd".to_string()).unwrap();

    client
        .command()
        .submit_and_wait(
            Commands::new(command_id, act_as.clone()).with_command(exercise.erase().into()),
        )
        .await
        .unwrap();
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
        })
        .next()
        .expect("created event is expected")
        .expect("expected to have a well-formed event")
}
