use std::sync::Arc;

use tonic::{Request, Status, service::Interceptor};

/// Supplies a raw bearer token, without the `Bearer ` prefix.
///
/// Canton accepts JWTs from configured auth services as well as opaque Canton
/// admin tokens. The server validates the token and its admin authorization.
/// JWT authentication must resolve to an admin claim; Ledger API user-based
/// authentication is not accepted by the Admin API.
/// See Canton's `GrpcAuthInterceptorFactory` and `CantonAdminTokenAuthService`.
///
/// Implement this for externally refreshed credentials. Called synchronously on
/// every RPC, so return a cached token rather than performing blocking I/O.
/// Returning `None` rejects the request locally as unauthenticated.
pub trait TokenProvider: Send + Sync + 'static {
    fn token(&self) -> Option<String>;
}

/// A fixed bearer token (either a JWT or an opaque admin token).
#[derive(Clone)]
pub struct StaticToken(String);

impl StaticToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }
}

impl TokenProvider for StaticToken {
    fn token(&self) -> Option<String> {
        Some(self.0.clone())
    }
}

/// Injects `authorization: Bearer <token>` into each RPC's metadata.
///
/// With no provider configured, request metadata is passed through. This also
/// supports Canton admin endpoints with no auth services configured.
#[derive(Clone)]
pub struct AuthInterceptor {
    provider: Option<Arc<dyn TokenProvider>>,
}

impl AuthInterceptor {
    pub fn new(token: Option<String>) -> Self {
        Self {
            provider: token
                .map(|token| Arc::new(StaticToken::new(token)) as Arc<dyn TokenProvider>),
        }
    }

    /// The provider is shared by all clones of the interceptor.
    pub fn from_token_provider(provider: impl TokenProvider) -> Self {
        Self {
            provider: Some(Arc::new(provider)),
        }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        if let Some(provider) = &self.provider {
            let token = provider
                .token()
                .filter(|token| !token.is_empty())
                .ok_or_else(|| Status::unauthenticated("authentication token unavailable"))?;
            // gRPC ASCII metadata permits printable ASCII only. HTTP header
            // parsing alone also accepts tabs and non-ASCII bytes.
            if !token.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
                return Err(Status::unauthenticated("invalid authentication token"));
            }
            let mut value = format!("Bearer {token}")
                .parse::<tonic::metadata::MetadataValue<_>>()
                .map_err(|_| Status::unauthenticated("invalid authentication token"))?;
            value.set_sensitive(true);
            request.metadata_mut().insert("authorization", value);
        }
        Ok(request)
    }
}
