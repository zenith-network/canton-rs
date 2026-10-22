use canton::{
    ledger_api::{
        grpc::v2::{
            client::CantonClient,
            retry::{CantonRetryPolicy, RetryConfig},
        },
        types::{
            v2::{
                AcsDelta, ArchivedEvent, ChoiceValue, Commands, CreatedEvent, Event, EventFormat,
                TemplateValue as _, Transaction, TransactionFormat,
            },
            value::v2::{HasIdentifier, Value},
        },
    },
    types::{Choice, ContractId, LedgerString, Name, NonEmpty, PartyId, Template},
};

// It's possible to define your types with derive macros

#[derive(Debug, Template, HasIdentifier, Value)]
#[identifier(package_id = "0123", package_name = "my_package", module = "MyModule")]
struct MyTemplate {
    value: i64,
}

#[derive(Debug, HasIdentifier, Value)]
#[identifier(package_id = "0123", package_name = "my_package", module = "MyModule")]
struct UpdateValue {
    #[name = "newValue"]
    new_value: i64,
}

impl Choice<MyTemplate> for UpdateValue {
    const CONSUMING: bool = true;
    const NAME: Name = Name::new_static_unchecked("MyChoice");
    type Result = ContractId<MyTemplate>;
}

pub async fn example_workflow() {
    let client = CantonClient::builder("http://localhost")
        .with_retry_config(RetryConfig::new().with_canton_policy(Some(CantonRetryPolicy::auto())))
        .connect_lazy()
        .unwrap();

    let my_contract = MyTemplate { value: 1 };

    let command_id = LedgerString::new("create-cmd".to_string()).unwrap();
    let act_as = NonEmpty::single(PartyId::new("my_party".to_string()).unwrap());

    let mut commands = Commands::new(command_id, act_as.clone());
    commands.with_command(my_contract.create().erase().into());

    // This generic here allows to skip Exercise events further
    let format = TransactionFormat::<AcsDelta>::new(EventFormat::new());

    let contract_id = client
        .command()
        .submit_and_wait_for_transaction(commands.clone(), Some(format.clone()))
        .await
        .map(contract_id_from_tx)
        .expect("submit_and_wait_for_transaction failed");

    let update_value = UpdateValue { new_value: 2 };

    let command_id = LedgerString::new("exercise-cmd".to_string()).unwrap();

    let mut commands = Commands::new(command_id, act_as.clone());
    commands.with_command(update_value.exercise(contract_id).erase().into());

    client
        .command()
        .submit_and_wait_for_transaction(commands.clone(), Some(format))
        .await
        .unwrap();
}

fn contract_id_from_tx(
    tx: Transaction<Event<CreatedEvent, ArchivedEvent>>,
) -> ContractId<MyTemplate> {
    let created = tx
        .events
        .into_iter()
        .filter_map(|event| match event {
            Event::Created(event) => Some(event.cast::<MyTemplate>()),
            Event::Archived(_) => None,
            t => unreachable!("unsupported event type: {t:?}"),
        })
        .next()
        .expect("created event is expected")
        .expect("expected to have a proper event");
    created.contract_id
}
