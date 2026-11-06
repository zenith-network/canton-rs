use std::{mem, time::Duration};

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use tonic::transport::{Channel, Endpoint};

use crate::grpc::v2::{
    auth::AuthInterceptor,
    error::ClientBuildError,
    retry::{RetryConfig, RetryHandler},
    services::{
        CommandServiceClient, EventQueryServiceClient, InteractiveSubmissionServiceClient,
        PackageServiceClient, PartyManagementServiceClient, StateServiceClient,
        UpdateServiceClient, VersionServiceClient,
    },
};

pub use tonic::transport::ClientTlsConfig;

#[cfg(not(feature = "tracing"))]
type InnerChannel = Channel;
#[cfg(feature = "tracing")]
type InnerChannel = crate::grpc::v2::tracing_layer::GrpcTracing<Channel>;

pub(crate) type InterceptedService =
    tonic::service::interceptor::InterceptedService<InnerChannel, AuthInterceptor>;

/// Builder for constructing a [`CantonClient`] with TLS and authentication.
///
/// # Example
///
/// ```rust,no_run
/// # async fn example() {
/// # use ledger_api::grpc::v2::client::CantonClientBuilder;
/// let client = CantonClientBuilder::new("https://localhost:5001")
///     .with_token("my-jwt-token")
///     .connect()
///     .await
///     .unwrap();
/// # }
/// ```
pub struct CantonClientBuilder {
    endpoint: String,
    tls_config: Option<ClientTlsConfig>,
    token: Option<String>,
    max_decoding_message_size: Option<usize>,
    retry_config: RetryConfig,
    http2_keep_alive_interval: Duration,
    keep_alive_timeout: Duration,
}

impl CantonClientBuilder {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tls_config: None,
            token: None,
            max_decoding_message_size: None,
            retry_config: RetryConfig::default(),
            http2_keep_alive_interval: CantonClient::DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL,
            keep_alive_timeout: CantonClient::DEFAULT_KEEP_ALIVE_TIMEOUT,
        }
    }

    pub fn with_tls(mut self, config: ClientTlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Max decoding message size will be set for all services returned by the client
    ///
    /// If not set, defaults to [`CantonClient::DEFAULT_MAX_RECV_MESSAGE_SIZE`].
    pub fn with_max_decoding_message_size(mut self, size: usize) -> Self {
        self.max_decoding_message_size = Some(size);
        self
    }

    /// Retry configuration of the client
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Override HTTP/2 keep-alive tuning.
    ///
    /// `interval` is how often PING frames are sent; `timeout` is how long to
    /// wait for the corresponding PONG before considering the connection dead.
    pub fn with_http2_keep_alive(mut self, interval: Duration, timeout: Duration) -> Self {
        self.http2_keep_alive_interval = interval;
        self.keep_alive_timeout = timeout;
        self
    }

    /// Build the client, connected to the API
    pub async fn connect(mut self) -> Result<CantonClient, ClientBuildError> {
        let endpoint = self.build_endpoint()?;
        let channel = endpoint.connect().await?;

        #[cfg(feature = "tracing")]
        tracing::info!(endpoint = %self.endpoint, "connected to canton");

        Ok(self.build_client_with_channel(channel))
    }

    /// Build the client
    ///
    /// This method uses lazy connection. The actual connection will be established on the first
    /// call to some RPC method.
    ///
    /// If you want to establish connection immediately, use [`CantonClientBuilder::connect`].
    pub fn connect_lazy(mut self) -> Result<CantonClient, ClientBuildError> {
        let endpoint = self.build_endpoint()?;
        let channel = endpoint.connect_lazy();
        Ok(self.build_client_with_channel(channel))
    }

    fn build_endpoint(&mut self) -> Result<Endpoint, ClientBuildError> {
        let mut endpoint = Endpoint::from_shared(mem::take(&mut self.endpoint))?
            .http2_keep_alive_interval(self.http2_keep_alive_interval)
            .keep_alive_timeout(self.keep_alive_timeout)
            .keep_alive_while_idle(true);

        if let Some(tls) = self.tls_config.take() {
            endpoint = endpoint.tls_config(tls)?;
        }

        Ok(endpoint)
    }

    fn build_client_with_channel(self, channel: Channel) -> CantonClient {
        #[cfg(feature = "tracing")]
        let channel = crate::grpc::v2::tracing_layer::GrpcTracing::new(channel);

        let interceptor = AuthInterceptor::new(self.token);

        let max_decoding_message_size = self
            .max_decoding_message_size
            .unwrap_or(CantonClient::DEFAULT_MAX_RECV_MESSAGE_SIZE);

        CantonClient {
            channel,
            interceptor,
            max_decoding_message_size,
            retry_handler: self.retry_config.into_handler(),
        }
    }
}

/// A connected Canton Ledger API client.
#[derive(Clone)]
pub struct CantonClient {
    channel: InnerChannel,
    interceptor: AuthInterceptor,
    max_decoding_message_size: usize,
    retry_handler: RetryHandler,
}

impl CantonClient {
    /// 128 MiB (default for Canton client, used in original Scala client code)
    pub const DEFAULT_MAX_RECV_MESSAGE_SIZE: usize = 0x8000000;

    /// Default HTTP/2 PING interval. Without keep-alive, a server-side
    /// participant restart (e.g. Canton losing its Postgres connection) can
    /// leave streaming RPCs like the update service silently hung — the
    /// client re-connects, `next().await` blocks forever, and no error is
    /// ever surfaced. Pinging every 20s bounds detection to ~30s.
    pub const DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL: Duration = Duration::from_secs(20);
    pub const DEFAULT_KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn builder(endpoint: impl Into<String>) -> CantonClientBuilder {
        CantonClientBuilder::new(endpoint)
    }

    /// Set a new retry configuration for the client
    ///
    /// Note that although the underlying channel is shared among the cloned clients, retry configs
    /// are not - each copy of the client will have it's own one. So changing this config here won't
    /// affect other clients created before.
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Reference to the retry configuration of the client
    pub fn retry_config(&self) -> &RetryConfig {
        self.retry_handler.config()
    }

    pub fn command(&self) -> CommandServiceClient {
        CommandServiceClient::new(
            proto::command_service_client::CommandServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn update(&self) -> UpdateServiceClient {
        UpdateServiceClient::new(
            proto::update_service_client::UpdateServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn event_query(&self) -> EventQueryServiceClient {
        EventQueryServiceClient::new(
            proto::event_query_service_client::EventQueryServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn state(&self) -> StateServiceClient {
        StateServiceClient::new(
            proto::state_service_client::StateServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn package(&self) -> PackageServiceClient {
        PackageServiceClient::new(
            proto::package_service_client::PackageServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn interactive_submission(&self) -> InteractiveSubmissionServiceClient {
        InteractiveSubmissionServiceClient::new(
            proto::interactive::interactive_submission_service_client::InteractiveSubmissionServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn party_management(&self) -> PartyManagementServiceClient {
        PartyManagementServiceClient::new(
            proto::admin::party_management_service_client::PartyManagementServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }

    pub fn version(&self) -> VersionServiceClient {
        VersionServiceClient::new(
            proto::version_service_client::VersionServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
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
    use crate::grpc::v2::error::CantonError;

    /// A Ledger API of one service, which answers every call the same and keeps each call.
    #[derive(Clone)]
    struct Ledger {
        answer: Answer,
        seen: Arc<Mutex<Vec<Call>>>,
    }

    /// A call's `authorization` header and encoded request.
    type Call = (Option<String>, Vec<u8>);

    #[derive(Clone)]
    #[allow(clippy::large_enum_variant, reason = "one answer per test")]
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
                    server
                        .serve_with_incoming(EventQueryServiceServer::new(self.clone()), incoming),
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

    #[tokio::test]
    async fn interactive_submission_returns_cantons_refusal_decoded() {
        let mut details = ErrorDetails::new();
        details.set_error_info(
            "NO_PREFERRED_PACKAGES_FOUND",
            "",
            [("category".to_owned(), "9".to_owned())],
        );
        let ledger = Ledger::new(Answer::Packages(Err(Status::with_error_details(
            Code::FailedPrecondition,
            "NO_PREFERRED_PACKAGES_FOUND(9,0): No package is vetted by all hosting participants",
            details,
        ))));
        let (client, _) = ledger.clients().await;

        let error = client
            .interactive_submission()
            .get_preferred_packages(preferred_request())
            .await
            .unwrap_err();
        let CantonError::Decoded(error) = error else {
            panic!("expected a decoded Canton error, got {error:?}");
        };
        assert_eq!(
            error.error_code_id().as_str(),
            "NO_PREFERRED_PACKAGES_FOUND"
        );
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

    #[tokio::test]
    async fn event_query_returns_no_events_or_not_found() {
        let (client, _) = Ledger::new(Answer::Events(Ok(Default::default())))
            .clients()
            .await;
        let answer = client
            .event_query()
            .get_events_by_contract_id_any(contract_id(), EventFormat::new())
            .await
            .unwrap();
        assert!(answer.created.is_none() && answer.archived.is_none());

        let (client, _) = Ledger::new(Answer::Events(Err(Status::not_found("not visible"))))
            .clients()
            .await;
        let error = client
            .event_query()
            .get_events_by_contract_id_any(contract_id(), EventFormat::new())
            .await
            .unwrap_err();
        assert_eq!(error.code(), Some(Code::NotFound));
    }
}
