use canton::{
    ledger_api::{
        grpc::v2::{client::CantonClient, error::CantonError},
        types::{
            v2::{
                AcsDelta, ArchivedEvent, ChoiceValue, Commands, CreatedEvent, Event, EventFormat,
                TemplateValue as _, Transaction, TransactionFormat,
            },
            value::v2::Value,
        },
    },
    types::{Choice, ContractId, LedgerString, Name, NonEmpty, PartyId},
};

#[derive(Debug, Value)]
#[value(
    template,
    package_id = "0123",
    package_name = "my_package",
    module_name = "MyModule"
)]
struct MyTemplate {
    value: i64,
}

#[derive(Debug, Value)]
#[value(
    package_id = "0123",
    package_name = "my_package",
    module_name = "MyModule"
)]
struct UpdateValue {
    #[value(name = "newValue")]
    new_value: i64,
}

impl Choice<MyTemplate> for UpdateValue {
    const CONSUMING: bool = true;

    const NAME: Name = Name::new_static_unchecked("MyChoice");

    type Result = ContractId<MyTemplate>;
}

impl ChoiceValue<MyTemplate> for UpdateValue {}

pub async fn example_workflow() {
    let client = CantonClient::builder("http://localhost")
        .connect_lazy()
        .unwrap();

    let my_contract = MyTemplate { value: 1 };

    let command_id = LedgerString::new("create-cmd".to_string()).unwrap();
    let act_as = NonEmpty::single(PartyId::new("my_party".to_string()).unwrap());

    let commands =
        Commands::new(command_id, act_as.clone()).with_command(my_contract.create().erase().into());

    // This generic here allows to skip Exercise events futher
    let format = TransactionFormat::<AcsDelta>::new(EventFormat::new());

    let result = client
        .command()
        .submit_and_wait_for_transaction(commands.clone(), Some(format.clone()))
        .await
        .map(contract_id_from_tx);

    let contract_id = match result {
        Ok(cid) => cid,
        Err(error) => match &error {
            CantonError::CantonGrpc(canton_grpc_error) => {
                if let Some(delay) = canton_grpc_error.category_id.retry() {
                    std::thread::sleep(delay);
                    client
                        .command()
                        .submit_and_wait_for_transaction(commands, Some(format.clone()))
                        .await
                        .map(contract_id_from_tx)
                        .expect("expected retry to succeed")
                } else {
                    panic!("{error}");
                }
            }
            CantonError::Raw(status) => panic!("unparsed error, status: {status:?}"),
            CantonError::ValueError(_) => panic!("{error}"),
        },
    };

    let update_value = UpdateValue { new_value: 2 };

    let command_id = LedgerString::new("exercise-cmd".to_string()).unwrap();

    let commands = Commands::new(command_id, act_as.clone())
        .with_command(update_value.exercise(contract_id).erase().into());

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
        })
        .next()
        .expect("created event is expected")
        .expect("expected to have a proper event");
    created.contract_id
}
