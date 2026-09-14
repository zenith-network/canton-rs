use canton_proto::com::digitalasset::canton as proto;
use tonic::transport::{Channel, Endpoint};

use crate::grpc::v30::services::{TopologyManagerReadClient, TopologyManagerWriteClient};

use super::{
    auth::{AuthInterceptor, TokenProvider},
    error::ClientBuildError,
};

pub use tonic::transport::ClientTlsConfig;

/// Transport shared by generated clients and Admin API service wrappers.
pub type InterceptedService =
    tonic::service::interceptor::InterceptedService<Channel, AuthInterceptor>;

/// Builder for an [`AdminClient`].
pub struct AdminClientBuilder {
    endpoint: String,
    tls_config: Option<ClientTlsConfig>,
    interceptor: AuthInterceptor,
}

impl AdminClientBuilder {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            tls_config: None,
            interceptor: AuthInterceptor::new(None),
        }
    }

    /// Configure server certificate validation and, if needed, an mTLS identity.
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

    /// Supply externally refreshed tokens; the provider is consulted per RPC
    /// and shared across client clones. Replaces any previous auth configuration.
    pub fn with_token_provider(mut self, provider: impl TokenProvider) -> Self {
        self.interceptor = AuthInterceptor::from_token_provider(provider);
        self
    }

    /// Establish the transport connection immediately.
    pub async fn connect(self) -> Result<AdminClient, ClientBuildError> {
        let channel = self.build_endpoint()?.connect().await?;
        Ok(self.build_client_with_channel(channel))
    }

    /// Connect on the first RPC.
    pub fn connect_lazy(self) -> Result<AdminClient, ClientBuildError> {
        let channel = self.build_endpoint()?.connect_lazy();
        Ok(self.build_client_with_channel(channel))
    }

    fn build_endpoint(&self) -> Result<Endpoint, ClientBuildError> {
        let mut endpoint =
            Endpoint::from_shared(self.endpoint.clone())?.keep_alive_while_idle(true);

        if let Some(tls) = &self.tls_config {
            endpoint = endpoint.tls_config(tls.clone())?;
        }
        Ok(endpoint)
    }

    fn build_client_with_channel(self, channel: Channel) -> AdminClient {
        AdminClient {
            channel,
            interceptor: self.interceptor,
        }
    }
}

/// Canton Admin API client. Clones share the connection and token provider.
#[derive(Clone)]
pub struct AdminClient {
    channel: Channel,
    interceptor: AuthInterceptor,
}

impl AdminClient {
    pub fn builder(endpoint: impl Into<String>) -> AdminClientBuilder {
        AdminClientBuilder::new(endpoint)
    }

    /// Create a topology manager read client
    pub fn topology_manager_read(&self) -> TopologyManagerReadClient {
        use proto::topology::admin::v30::topology_manager_read_service_client::TopologyManagerReadServiceClient;
        TopologyManagerReadClient::new(TopologyManagerReadServiceClient::with_interceptor(
            self.channel.clone(),
            self.interceptor.clone(),
        ))
    }

    pub fn topology_manager_write(&self) -> TopologyManagerWriteClient {
        use proto::topology::admin::v30::topology_manager_write_service_client::TopologyManagerWriteServiceClient;
        TopologyManagerWriteClient::new(TopologyManagerWriteServiceClient::with_interceptor(
            self.channel.clone(),
            self.interceptor.clone(),
        ))
    }
}
