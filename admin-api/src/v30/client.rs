use admin_api_types::ApiName;
use canton_base::{
    auth::interceptor::AuthInterceptor,
    retry::{RetryConfig, RetryHandler},
};
use canton_proto::com::digitalasset::canton as proto;
use protobuf_utils::InvalidProtoField as _;
use tonic::transport::{Channel, Endpoint};

use crate::v30::error::AdminApiError;

use super::error::ClientBuildError;

#[cfg(feature = "_tls-any")]
pub use tonic::transport::ClientTlsConfig;

/// Transport shared by generated clients and Admin API service wrappers.
pub type InterceptedService =
    tonic::service::interceptor::InterceptedService<Channel, AuthInterceptor>;

/// Builder for an [`AdminApiClient`].
pub struct AdminApiClientBuilder {
    endpoint: String,
    #[cfg(feature = "_tls-any")]
    tls_config: Option<ClientTlsConfig>,
    interceptor: AuthInterceptor,
    retry_config: RetryConfig,
}

impl AdminApiClientBuilder {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            #[cfg(feature = "_tls-any")]
            tls_config: None,
            interceptor: AuthInterceptor::new(None),
            retry_config: RetryConfig::default(),
        }
    }

    /// Configure server certificate validation and, if needed, an mTLS identity.
    ///
    /// If not specified explicitly, defaults to `ClientTlsConfig::new().with_enabled_roots()`.
    #[cfg(feature = "_tls-any")]
    pub fn with_tls(mut self, config: ClientTlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    /// Supply a raw JWT or opaque admin token, without the `Bearer ` prefix.
    ///
    /// Replaces any previously configured token provider. The token must be
    /// accepted by the server and authorize admin access.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.interceptor = AuthInterceptor::new(Some(token.into()));
        self
    }

    /// Retry configuration of the client
    pub fn with_retry_config(mut self, retry_config: RetryConfig) -> Self {
        self.retry_config = retry_config;
        self
    }

    /// Establish the transport connection immediately.
    pub async fn connect(self) -> Result<AdminApiClient, ClientBuildError> {
        let channel = self.build_endpoint()?.connect().await?;
        Ok(self.build_client_with_channel(channel))
    }

    /// Connect on the first RPC.
    pub fn connect_lazy(self) -> Result<AdminApiClient, ClientBuildError> {
        let channel = self.build_endpoint()?.connect_lazy();
        Ok(self.build_client_with_channel(channel))
    }

    fn build_endpoint(&self) -> Result<Endpoint, ClientBuildError> {
        let endpoint = Endpoint::from_shared(self.endpoint.clone())?.keep_alive_while_idle(true);

        // We check this to prevent accidentally sending data in plaintext
        #[cfg(not(feature = "_tls-any"))]
        if endpoint.uri().scheme() == Some(&http::uri::Scheme::HTTPS) {
            return Err(ClientBuildError::TlsNotSupported);
        }

        #[cfg(feature = "_tls-any")]
        let endpoint = {
            // ClientTlsConfig::new().with_enabled_roots() looks like a reasonable default if
            // user haven't provided any configuration.
            // Without this, all TLS calls fail unless with_tls() was explicitly called.
            let tls_config = self
                .tls_config
                .clone()
                .unwrap_or_else(|| ClientTlsConfig::new().with_enabled_roots());
            endpoint.tls_config(tls_config)?
        };

        Ok(endpoint)
    }

    fn build_client_with_channel(self, channel: Channel) -> AdminApiClient {
        AdminApiClient {
            channel,
            interceptor: self.interceptor,
            retry_handler: self.retry_config.into_handler(),
        }
    }
}

/// Canton Admin API client.
///
/// Cloning this client will re-use the same connection, authentication token and retry budget.
#[derive(Clone)]
pub struct AdminApiClient {
    channel: Channel,
    interceptor: AuthInterceptor,
    retry_handler: RetryHandler,
}

impl AdminApiClient {
    /// Get builder of the [`AdminApiClient`]
    pub fn builder(endpoint: impl Into<String>) -> AdminApiClientBuilder {
        AdminApiClientBuilder::new(endpoint)
    }

    /// Reference to the retry configuration of the client
    pub fn retry_config(&self) -> &RetryConfig {
        self.retry_handler.config()
    }

    /// Set a new retry configuration for the client
    ///
    /// Note that although the underlying channel is shared among the cloned clients, retry configs
    /// are not - each copy of the client will have it's own one. So changing this config here won't
    /// affect other clients created before.
    pub fn set_retry_config(&mut self, retry_config: RetryConfig) {
        self.retry_handler = retry_config.into_handler();
    }

    /// Provides basic information about the API (if it is an Admin API or public Sequencer API).
    ///
    /// This is useful to detect port misconfigurations and provide a user-friendly error message.
    pub async fn get_api_info(&self) -> Result<ApiName, AdminApiError> {
        // We do not extract this one call to a separate service, because it's too primitive.
        // There are also no other methods on this service. So we implement this directly here.
        use proto::connection::v30::{
            GetApiInfoRequest, GetApiInfoResponse, api_info_service_client::ApiInfoServiceClient,
        };

        let service =
            ApiInfoServiceClient::with_interceptor(self.channel.clone(), self.interceptor.clone());

        let request = GetApiInfoRequest::default();
        let response = self
            .retry_handler
            .call(&service, &request, |mut svc, req| async move {
                svc.get_api_info(req)
                    .await
                    .map_err(AdminApiError::from_status)
            })
            .await?;
        ApiName::parse(&response.name)
            .validated_of::<GetApiInfoResponse>("name")
            .map_err(AdminApiError::value_error)
    }
}
