//! Participant connections, isolated identities, and fixture commands.

use canton::types::{ContractId, LedgerString, NonEmpty, PartyId, UserId};
use ledger_api::grpc::v2::{
    auth::AuthInterceptor,
    client::CantonClient,
    retry::RetryConfig,
    services::{
        CommandCompletionServiceClient, CommandSubmissionServiceClient, EventQueryServiceClient,
        admin::AllocatePartyRequest,
    },
};
use ledger_api_integration::integration::{Keyed, Sample};
use ledger_api_proto::com::daml::ledger::api::v2 as p;
use ledger_api_types::v2::*;
use pretty_assertions::assert_eq;
use tonic::transport::Channel;

use super::{created, endpoint, id, ok, payload};

/// SDK and protobuf clients with fresh parties and a user for each test.
///
/// The owner can act, the observer can see the sample, and the stranger is used
/// to check visibility filters. Parties are allocated through the SDK; the
/// generated user-management client grants the owner's command rights.
pub struct Context {
    pub client: CantonClient,
    pub channel: Channel,
    pub owner: PartyId,
    pub observer: PartyId,
    pub stranger: PartyId,
    pub user: UserId,
}

impl Context {
    /// Connect without SDK retries and allocate isolated test identities.
    pub async fn new() -> Self {
        let channel = ok(tonic::transport::Endpoint::from_shared(endpoint())
            .unwrap()
            .connect())
        .await;
        let client = ok(CantonClient::builder(endpoint())
            .with_retry_config(RetryConfig::no_retry())
            .connect())
        .await;
        let owner = allocate_party(&client, "owner").await;
        let observer = allocate_party(&client, "observer").await;
        let stranger = allocate_party(&client, "stranger").await;
        let user = UserId::new(id("user").to_string()).unwrap();
        create_user(&channel, &owner, &user).await;
        Self {
            client,
            channel,
            owner,
            observer,
            stranger,
            user,
        }
    }

    /// Build the default sample owned by this test, with its observer party.
    pub fn sample(&self) -> Sample {
        Sample {
            owner: self.owner.clone(),
            observer_party: self.observer.clone(),
            payload: payload(42),
        }
    }

    /// Build a keyed template with a unique label to avoid key collisions.
    pub fn keyed(&self) -> Keyed {
        Keyed {
            owner: self.owner.clone(),
            label: id("key").to_string(),
            number: 123,
        }
    }

    /// Wrap a command with this test user, acting party, and unique identifiers.
    pub fn commands(&self, command: impl Into<Command>) -> Commands {
        let mut cmds = Commands::new(id("command"), NonEmpty::single(self.owner.clone()));
        cmds.with_user_id(Some(self.user.clone()))
            .with_workflow_id(Some(id("workflow")))
            .with_submission_id(Some(id("submission")))
            .with_command(command.into());
        cmds
    }

    /// Filter events to the owner, with the requested verbosity.
    pub fn events(&self, verbose: bool) -> EventFormat {
        EventFormat::new()
            .with_filter(self.owner.clone(), Filters::wildcard())
            .with_verbose(verbose)
    }

    /// Request created and archived events in ACS-delta form.
    pub fn acs(&self) -> TransactionFormat<AcsDelta> {
        TransactionFormat::<AcsDelta>::new(self.events(true))
    }

    /// Request ledger effects, including exercised events and choice results.
    pub fn effects(&self) -> TransactionFormat<LedgerEffects> {
        TransactionFormat::<LedgerEffects>::new(self.events(true))
    }

    /// Include owner-visible ACS-delta transactions in update responses.
    pub fn updates(&self) -> UpdateFormat<AcsDelta> {
        UpdateFormat::new().include_transaction(self.acs())
    }

    /// Construct the submission wrapper without authentication or retries.
    pub fn submission(&self) -> CommandSubmissionServiceClient {
        CommandSubmissionServiceClient::new(
            p::command_submission_service_client::CommandSubmissionServiceClient::with_interceptor(
                self.channel.clone(),
                AuthInterceptor::new(None),
            ),
            RetryConfig::no_retry().into_handler(),
        )
    }

    /// Construct the completion wrapper without authentication or retries.
    pub fn completion(&self) -> CommandCompletionServiceClient {
        CommandCompletionServiceClient::new(
            p::command_completion_service_client::CommandCompletionServiceClient::with_interceptor(
                self.channel.clone(),
                AuthInterceptor::new(None),
            ),
            RetryConfig::no_retry().into_handler(),
        )
    }

    /// Construct the event-query wrapper without authentication or retries.
    pub fn event_query(&self) -> EventQueryServiceClient {
        EventQueryServiceClient::new(
            p::event_query_service_client::EventQueryServiceClient::with_interceptor(
                self.channel.clone(),
                AuthInterceptor::new(None),
            ),
            RetryConfig::no_retry().into_handler(),
        )
    }

    /// Submit the default sample and verify its decoded value before returning its ID.
    pub async fn create(
        &self,
    ) -> (
        Sample,
        Transaction<AcsDeltaEvent<CreatedEvent, ArchivedEvent>>,
        ContractId<Sample>,
    ) {
        let value = self.sample();
        let commands = self.commands(value.clone().create().erase());
        let tx = ok(self
            .client
            .command()
            .submit_and_wait_for_transaction(commands, Some(self.acs())))
        .await;
        let created = created(&tx);
        assert_eq!(
            created.clone().cast::<Sample>().unwrap().create_arguments,
            value
        );
        let cid = created.contract_id.clone().into_typed();
        (value, tx, cid)
    }

    /// Read a transaction through the protobuf client for independent comparisons.
    pub async fn raw_transaction<S: TxShape>(
        &self,
        update_id: &LedgerString,
        format: TransactionFormat<S>,
    ) -> p::Transaction {
        let response = ok(
            p::update_service_client::UpdateServiceClient::new(self.channel.clone())
                .get_update_by_id(p::GetUpdateByIdRequest {
                    update_id: update_id.to_string(),
                    update_format: Some(UpdateFormat::new().include_transaction(format).into()),
                }),
        )
        .await
        .into_inner();
        match response.update.expect("update") {
            p::get_update_response::Update::Transaction(tx) => tx,
            other => panic!("expected transaction: {other:?}"),
        }
    }
}

/// Allocate each setup party through the public SDK client with a unique hint.
async fn allocate_party(client: &CantonClient, prefix: &str) -> PartyId {
    let hint = PartyId::new(id(prefix).to_string()).unwrap();
    ok(client
        .party_management()
        .allocate_party(AllocatePartyRequest::with_hint(hint)))
    .await
    .party
}

/// Grant the test user permission to act as its owner party.
async fn create_user(channel: &Channel, owner: &PartyId, user: &UserId) {
    ok(
        p::admin::user_management_service_client::UserManagementServiceClient::new(channel.clone())
            .create_user(p::admin::CreateUserRequest {
                user: Some(p::admin::User {
                    id: user.to_string(),
                    ..Default::default()
                }),
                rights: vec![p::admin::Right {
                    kind: Some(p::admin::right::Kind::CanActAs(p::admin::right::CanActAs {
                        party: owner.to_string(),
                    })),
                }],
                ..Default::default()
            }),
    )
    .await;
}
