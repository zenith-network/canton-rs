use std::{mem, sync::Arc, time::Duration};

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};

use crate::grpc::v2::{
    auth::AuthInterceptor,
    error::CantonError,
    error::ClientBuildError,
    retry::{RetryConfig, RetryHandler},
    services::{
        ApiVersion, CommandServiceClient, PackageServiceClient, StateServiceClient,
        UpdateServiceClient, VersionServiceClient, admin::PartyManagementServiceClient,
    },
};

#[cfg(feature = "_tls-any")]
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
    #[cfg(feature = "_tls-any")]
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
            #[cfg(feature = "_tls-any")]
            tls_config: None,
            token: None,
            max_decoding_message_size: None,
            retry_config: RetryConfig::default(),
            http2_keep_alive_interval: CantonClient::DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL,
            keep_alive_timeout: CantonClient::DEFAULT_KEEP_ALIVE_TIMEOUT,
        }
    }

    #[cfg(feature = "_tls-any")]
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
        #[allow(unused_mut, reason = "Avoid warning when TLS is not enabled")]
        let mut endpoint = Endpoint::from_shared(mem::take(&mut self.endpoint))?
            .http2_keep_alive_interval(self.http2_keep_alive_interval)
            .keep_alive_timeout(self.keep_alive_timeout)
            .keep_alive_while_idle(true);

        #[cfg(feature = "_tls-any")]
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
            server_caps: Arc::new(RwLock::new(None)),
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
    server_caps: Arc<RwLock<Option<ApiVersion>>>,
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

    /// Resolve server version and enabled features through [`VersionService`] and cache these
    /// results inside the client.
    ///
    /// An error will be returned, if the server version is not compatible with client version
    /// (see [`supported_versions()`][crate::supported_versions()]).
    ///
    /// Cached results are shared among all copies of the client. Client will remember these values
    /// and respect them in the future.
    ///
    /// By default, if the version wasn't resolved, client treats all services as available.
    ///
    /// Note that during client lifetime the server may change its capabilities (e.g. it may be
    /// upgraded to a newer version). The client cannot detect these situations automatically,
    /// because the underlying channel could be silently re-created. Given that, in your application
    /// it might be a good idea to run this resolution periodically, thus making error messages more
    /// precise.
    pub async fn resolve_server_capabilities(&self) -> Result<(), CantonError> {
        let api_version = self.version().get_ledger_api_version().await?;

        if !crate::is_supported_version(&api_version.version) {
            return Err(CantonError::UnsupportedApiVersion {
                server_version: api_version.version.clone(),
            });
        }

        #[cfg(feature = "tracing")]
        {
            tracing::debug!(version = %api_version.version, "Resolved Ledger API version");
            tracing::debug!(features = ?api_version.features, "Resolved Ledger API features");
        }

        let mut server_caps = self.server_caps.write().await;
        *server_caps = Some(api_version);
        Ok(())
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

    pub async fn package(&self) -> PackageServiceClient {
        let max_vetted_packages_page_size =
            if let Some(api_version) = self.server_caps.read().await.as_ref() {
                Some(
                    api_version
                        .features
                        .package_feature
                        .max_vetted_packages_page_size,
                )
            } else {
                None
            };
        PackageServiceClient::new(
            proto::package_service_client::PackageServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
            max_vetted_packages_page_size,
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

    pub async fn party_management(&self) -> PartyManagementServiceClient {
        let max_parties_page_size =
            if let Some(api_version) = self.server_caps.read().await.as_ref() {
                Some(api_version.features.party_management.max_parties_page_size)
            } else {
                None
            };
        PartyManagementServiceClient::new(
            proto::admin::party_management_service_client::PartyManagementServiceClient::with_interceptor(
                self.channel.clone(),
                self.interceptor.clone(),
            )
            .max_decoding_message_size(self.max_decoding_message_size),
            self.retry_handler.clone(),
            max_parties_page_size,
        )
    }
}
