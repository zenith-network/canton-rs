use std::{
    assert_matches,
    sync::{Arc, Mutex},
};

use ledger_api_proto::com::daml::ledger::api::v2::{
    Created, GetEventsByContractIdRequest, GetEventsByContractIdResponse, PackageReference,
    event_query_service_server::{EventQueryService, EventQueryServiceServer},
    interactive::{
        ExecuteSubmissionAndWaitForTransactionRequest,
        ExecuteSubmissionAndWaitForTransactionResponse, ExecuteSubmissionAndWaitRequest,
        ExecuteSubmissionAndWaitResponse, ExecuteSubmissionRequest, ExecuteSubmissionResponse,
        GetPreferredPackageVersionRequest, GetPreferredPackageVersionResponse,
        GetPreferredPackagesRequest, GetPreferredPackagesResponse, PackageVettingRequirement,
        PrepareSubmissionRequest, PrepareSubmissionResponse,
        interactive_submission_service_server::{
            InteractiveSubmissionService, InteractiveSubmissionServiceServer,
        },
    },
};
use ledger_api_types::{canton_types::ContractId, v2::EventFormat};
use prost::Message as _;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Code, Request, Response, Status, transport::Server};
use tonic_types::{ErrorDetails, StatusExt as _};

use super::CantonClient;
use crate::grpc::v2::{
    error::CantonError,
    retry::{CantonRetryPolicy, RetryConfig},
};

/// A Ledger API of one service, which answers every call the same and keeps each call.
#[derive(Clone)]
struct Ledger {
    answer: Answer,
    seen: Arc<Mutex<Vec<Call>>>,
}

/// A call's `authorization` header and encoded request.
type Call = (Option<String>, Vec<u8>);

#[derive(Clone)]
#[expect(clippy::large_enum_variant, reason = "one answer per test")]
enum Answer {
    Packages(Result<GetPreferredPackagesResponse, Status>),
    Events(Result<GetEventsByContractIdResponse, Status>),
}

impl Ledger {
    fn new(answer: Answer) -> Self {
        Self {
            answer,
            seen: Arc::default(),
        }
    }

    /// Serves on a local port until the test ends. Two clients of it: one with a token, and
    /// one whose decoding limit, a byte, only an empty answer fits in.
    async fn clients(&self) -> (CantonClient, CantonClient) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let incoming = TcpListenerStream::new(listener);
        let server = Server::builder();
        match self.answer {
            Answer::Packages(_) => tokio::spawn(server.serve_with_incoming(
                InteractiveSubmissionServiceServer::new(self.clone()),
                incoming,
            )),
            Answer::Events(_) => tokio::spawn(
                server.serve_with_incoming(EventQueryServiceServer::new(self.clone()), incoming),
            ),
        };
        let client = CantonClient::builder(&url)
            .with_token("tok")
            .connect_lazy()
            .unwrap();
        let limited = CantonClient::builder(&url)
            .with_max_decoding_message_size(1)
            .connect_lazy()
            .unwrap();
        (client, limited)
    }

    fn answer<Req: prost::Message, Resp: Clone>(
        &self,
        request: Request<Req>,
        answer: &Result<Resp, Status>,
    ) -> Result<Response<Resp>, Status> {
        let token = request
            .metadata()
            .get("authorization")
            .map(|value| value.to_str().unwrap().to_owned());
        let request = request.into_inner().encode_to_vec();
        self.seen.lock().unwrap().push((token, request));
        answer.clone().map(Response::new)
    }

    fn seen(&self) -> Vec<Call> {
        self.seen.lock().unwrap().clone()
    }
}

#[tonic::async_trait]
impl EventQueryService for Ledger {
    async fn get_events_by_contract_id(
        &self,
        request: Request<GetEventsByContractIdRequest>,
    ) -> Result<Response<GetEventsByContractIdResponse>, Status> {
        let Answer::Events(answer) = &self.answer else {
            unreachable!("served only with an event query answer")
        };
        self.answer(request, answer)
    }
}

#[tonic::async_trait]
impl InteractiveSubmissionService for Ledger {
    async fn get_preferred_packages(
        &self,
        request: Request<GetPreferredPackagesRequest>,
    ) -> Result<Response<GetPreferredPackagesResponse>, Status> {
        let Answer::Packages(answer) = &self.answer else {
            unreachable!("served only with a preferred packages answer")
        };
        self.answer(request, answer)
    }

    async fn prepare_submission(
        &self,
        _: Request<PrepareSubmissionRequest>,
    ) -> Result<Response<PrepareSubmissionResponse>, Status> {
        Err(Status::unimplemented("not part of the test"))
    }

    async fn execute_submission(
        &self,
        _: Request<ExecuteSubmissionRequest>,
    ) -> Result<Response<ExecuteSubmissionResponse>, Status> {
        Err(Status::unimplemented("not part of the test"))
    }

    async fn execute_submission_and_wait(
        &self,
        _: Request<ExecuteSubmissionAndWaitRequest>,
    ) -> Result<Response<ExecuteSubmissionAndWaitResponse>, Status> {
        Err(Status::unimplemented("not part of the test"))
    }

    async fn execute_submission_and_wait_for_transaction(
        &self,
        _: Request<ExecuteSubmissionAndWaitForTransactionRequest>,
    ) -> Result<Response<ExecuteSubmissionAndWaitForTransactionResponse>, Status> {
        Err(Status::unimplemented("not part of the test"))
    }

    async fn get_preferred_package_version(
        &self,
        _: Request<GetPreferredPackageVersionRequest>,
    ) -> Result<Response<GetPreferredPackageVersionResponse>, Status> {
        Err(Status::unimplemented("not part of the test"))
    }
}

fn preferred_request() -> GetPreferredPackagesRequest {
    GetPreferredPackagesRequest {
        package_vetting_requirements: vec![PackageVettingRequirement {
            parties: vec!["alice::1220ab".into(), "bob::1220cd".into()],
            package_name: "my-package".into(),
        }],
        synchronizer_id: String::new(),
        vetting_valid_at: None,
    }
}

fn contract_id() -> ContractId {
    ContractId::new("00".repeat(33)).unwrap()
}

#[tokio::test]
async fn interactive_submission_is_served_with_the_clients_token_and_decoding_limit() {
    let response = GetPreferredPackagesResponse {
        package_references: vec![PackageReference {
            package_id: "ab".repeat(32),
            package_name: "my-package".into(),
            package_version: "1.0.0".into(),
        }],
        synchronizer_id: "sync::1220ef".into(),
    };
    let ledger = Ledger::new(Answer::Packages(Ok(response.clone())));
    let (client, limited) = ledger.clients().await;

    let answer = client
        .interactive_submission()
        .get_preferred_packages(preferred_request())
        .await
        .unwrap();
    assert_eq!(answer, response);
    assert_eq!(
        ledger.seen(),
        vec![(
            Some("Bearer tok".into()),
            preferred_request().encode_to_vec()
        )]
    );

    let error = limited
        .interactive_submission()
        .get_preferred_packages(preferred_request())
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(Code::OutOfRange), "{error:?}");
}

/// The answer is one Canton never gives, `created` without its event: an error, not "no
/// event". Longer than a byte, it is refused by the limited client.
#[tokio::test]
async fn event_query_is_served_with_the_clients_token_and_decoding_limit() {
    let ledger = Ledger::new(Answer::Events(Ok(GetEventsByContractIdResponse {
        created: Some(Created {
            created_event: None,
            synchronizer_id: "sync::1220ef".into(),
        }),
        archived: None,
    })));
    let (client, limited) = ledger.clients().await;

    let error = client
        .event_query()
        .get_events_by_contract_id_any(contract_id(), EventFormat::new())
        .await
        .unwrap_err();
    assert_matches!(error, CantonError::ValueError(_));
    let request = GetEventsByContractIdRequest {
        contract_id: contract_id().into(),
        event_format: Some(EventFormat::new().into()),
    };
    assert_eq!(
        ledger.seen(),
        vec![(Some("Bearer tok".into()), request.encode_to_vec())]
    );

    let error = limited
        .event_query()
        .get_events_by_contract_id_any(contract_id(), EventFormat::new())
        .await
        .unwrap_err();
    assert_eq!(error.code(), Some(Code::OutOfRange), "{error:?}");
}

/// Canton's transient refusal, to be retried at once.
fn transient() -> Status {
    let mut details = ErrorDetails::new();
    details.set_error_info(
        "SERVICE_NOT_RUNNING",
        "",
        [("category".to_owned(), "1".to_owned())],
    );
    details.set_retry_info(Some(std::time::Duration::ZERO));
    Status::with_error_details(
        Code::Unavailable,
        "SERVICE_NOT_RUNNING(1,0): Service is not running.",
        details,
    )
}

/// A client whose retry configuration allows one retry: each accessor's service retries
/// with it, once.
#[tokio::test]
async fn both_accessors_retry_with_the_clients_retry_configuration() {
    let retry =
        RetryConfig::no_retry().with_canton_policy(Some(CantonRetryPolicy::auto_with_max(1)));

    let ledger = Ledger::new(Answer::Packages(Err(transient())));
    let (mut client, _) = ledger.clients().await;
    client.set_retry_config(retry.clone());
    client
        .interactive_submission()
        .get_preferred_packages(preferred_request())
        .await
        .unwrap_err();
    assert_eq!(ledger.seen().len(), 2);

    let ledger = Ledger::new(Answer::Events(Err(transient())));
    let (mut client, _) = ledger.clients().await;
    client.set_retry_config(retry);
    client
        .event_query()
        .get_events_by_contract_id_any(contract_id(), EventFormat::new())
        .await
        .unwrap_err();
    assert_eq!(ledger.seen().len(), 2);
}
