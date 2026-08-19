use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

use crate::grpc::v2::{
    auth::AuthInterceptor,
    error::ClientBuildError,
    retry::{RetryConfig, RetryHandler},
    services::{
        CommandServiceClient, PackageServiceClient, StateServiceClient, UpdateServiceClient,
        VersionServiceClient,
    },
};

#[cfg(not(feature = "tracing"))]
type InnerChannel = Channel;
#[cfg(feature = "tracing")]
type InnerChannel = crate::tracing_layer::GrpcTracing<Channel>;

pub(crate) type InterceptedService =
    tonic::service::interceptor::InterceptedService<InnerChannel, AuthInterceptor>;

/// Builder for constructing a [`CantonClient`] with TLS and authentication.
///
/// # Example
/// ```no_run
/// # async fn example() -> ledger_api::grpc::error::Result<()> {
/// # use ledger_api::grpc::CantonClientBuilder;
/// let client = CantonClientBuilder::new("https://localhost:5001")
///     .with_token("my-jwt-token")
///     .connect()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct CantonClientBuilder {
    endpoint: String,
    tls_config: Option<ClientTlsConfig>,
    token: Option<String>,
    max_decoding_message_size: Option<usize>,
    retry_config: RetryConfig,
}

impl CantonClientBuilder {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tls_config: None,
            token: None,
            max_decoding_message_size: None,
            retry_config: RetryConfig::default(),
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

    /// Build the client, connected to the API
    pub async fn connect(mut self) -> Result<CantonClient, ClientBuildError> {
        let endpoint = Self::build_endpoint(self.endpoint.clone(), self.tls_config.take())?;
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
        let endpoint = Self::build_endpoint(self.endpoint.clone(), self.tls_config.take())?;
        let channel = endpoint.connect_lazy();
        Ok(self.build_client_with_channel(channel))
    }

    fn build_endpoint(
        endpoint: String,
        tls_config: Option<ClientTlsConfig>,
    ) -> Result<Endpoint, ClientBuildError> {
        let mut endpoint = Endpoint::from_shared(endpoint)?;

        if let Some(tls) = tls_config {
            endpoint = endpoint.tls_config(tls)?;
        }

        Ok(endpoint)
    }

    fn build_client_with_channel(self, channel: Channel) -> CantonClient {
        #[cfg(feature = "tracing")]
        let channel = crate::tracing_layer::GrpcTracing::new(channel);

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
