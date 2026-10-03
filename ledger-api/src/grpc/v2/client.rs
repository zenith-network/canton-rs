use std::{mem, time::Duration};

use ledger_api_proto::com::daml::ledger::api::v2 as proto;
use tonic::transport::{Channel, Endpoint};

use crate::grpc::v2::{
    auth::AuthInterceptor,
    error::ClientBuildError,
    retry::{RetryConfig, RetryHandler},
    services::{
        CommandServiceClient, PackageServiceClient, StateServiceClient, UpdateServiceClient,
        VersionServiceClient,
    },
};

#[cfg(any(
    feature = "tls-ring",
    feature = "tls-aws-lc",
    feature = "tls-native-roots",
    feature = "tls-webpki-roots",
))]
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
    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        feature = "tls-native-roots",
        feature = "tls-webpki-roots",
    ))]
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
            #[cfg(any(
                feature = "tls-ring",
                feature = "tls-aws-lc",
                feature = "tls-native-roots",
                feature = "tls-webpki-roots",
            ))]
            tls_config: None,
            token: None,
            max_decoding_message_size: None,
            retry_config: RetryConfig::default(),
            http2_keep_alive_interval: CantonClient::DEFAULT_HTTP2_KEEP_ALIVE_INTERVAL,
            keep_alive_timeout: CantonClient::DEFAULT_KEEP_ALIVE_TIMEOUT,
        }
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        feature = "tls-native-roots",
        feature = "tls-webpki-roots",
    ))]
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

    pub(crate) fn build_endpoint(&mut self) -> Result<Endpoint, ClientBuildError> {
        let endpoint = Endpoint::from_shared(mem::take(&mut self.endpoint))?
            .http2_keep_alive_interval(self.http2_keep_alive_interval)
            .keep_alive_timeout(self.keep_alive_timeout)
            .keep_alive_while_idle(true);

        #[cfg(any(
            feature = "tls-ring",
            feature = "tls-aws-lc",
            feature = "tls-native-roots",
            feature = "tls-webpki-roots",
        ))]
        let endpoint = if let Some(tls) = self.tls_config.take() {
            endpoint.tls_config(tls)?
        } else {
            endpoint
        };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = CantonClientBuilder::new("http://localhost:5001");
        assert_eq!(builder.endpoint, "http://localhost:5001");
        assert!(builder.token.is_none());
        assert!(builder.max_decoding_message_size.is_none());
        #[cfg(any(
            feature = "tls-ring",
            feature = "tls-aws-lc",
            feature = "tls-native-roots",
            feature = "tls-webpki-roots",
        ))]
        assert!(builder.tls_config.is_none());
    }

    #[test]
    fn test_build_endpoint_without_tls() {
        let mut builder = CantonClientBuilder::new("http://localhost:5001")
            .with_token("test-token")
            .with_max_decoding_message_size(1024 * 1024);
        let endpoint = builder.build_endpoint();
        assert!(endpoint.is_ok());
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        feature = "tls-native-roots",
        feature = "tls-webpki-roots",
    ))]
    #[test]
    fn test_builder_with_tls() {
        let tls = ClientTlsConfig::new();
        let builder = CantonClientBuilder::new("https://localhost:5001").with_tls(tls);
        assert!(builder.tls_config.is_some());
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        feature = "tls-native-roots",
        feature = "tls-webpki-roots",
    ))]
    #[test]
    fn test_build_endpoint_with_tls() {
        let tls = ClientTlsConfig::new();
        let mut builder = CantonClientBuilder::new("https://localhost:5001").with_tls(tls);
        assert!(builder.tls_config.is_some());
        let endpoint = builder.build_endpoint();
        assert!(endpoint.is_ok());
        assert!(
            builder.tls_config.is_none(),
            "tls_config must be consumed and applied to endpoint"
        );
    }

    #[cfg(any(
        feature = "tls-ring",
        feature = "tls-aws-lc",
        feature = "tls-native-roots",
        feature = "tls-webpki-roots",
    ))]
    #[tokio::test]
    async fn test_connect_with_tls_fails_on_plaintext_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            if let Ok((mut socket, _)) = listener.accept().await {
                let _ = socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await;
            }
        });

        let tls = ClientTlsConfig::new().domain_name("localhost");
        let builder = CantonClientBuilder::new(format!("https://{addr}")).with_tls(tls);

        let res = builder.connect().await;
        // Connecting with TLS to a non-TLS server fails at the TLS handshake layer,
        // confirming that TLS transport is actively configured and applied.
        assert!(res.is_err());
    }
}
