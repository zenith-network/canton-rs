use std::sync::Arc;

use tonic::service::Interceptor;

/// Trait for types that can provide a JWT bearer token.
///
/// Implement this for custom token refresh logic (e.g. rotating tokens).
pub trait TokenProvider: Send + Sync + 'static {
    fn token(&self) -> Option<String>;
}

/// A fixed, non-refreshable token.
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

/// Interceptor that injects a JWT bearer token into gRPC metadata.
#[derive(Clone)]
pub struct AuthInterceptor {
    token: Option<String>,
}

impl AuthInterceptor {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

impl Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(ref token) = self.token {
            let value = format!("Bearer {token}").parse().map_err(|parse_error| {
                let mut err = tonic::Status::internal("invalid auth token");
                err.set_source(Arc::new(parse_error));
                err
            })?;
            request.metadata_mut().insert("authorization", value);
        }
        Ok(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_token() {
        let token = StaticToken::new("my-jwt");
        assert_eq!(token.token(), Some("my-jwt".to_string()));
    }

    #[test]
    fn test_interceptor_adds_bearer_header() {
        let mut interceptor = AuthInterceptor::new(Some("tok123".to_string()));
        let request = tonic::Request::new(());
        let result = interceptor.call(request).unwrap();
        let auth = result.metadata().get("authorization").unwrap();
        assert_eq!(auth, "Bearer tok123");
    }

    #[test]
    fn test_interceptor_no_token_passes_through() {
        let mut interceptor = AuthInterceptor::new(None);
        let request = tonic::Request::new(());
        let result = interceptor.call(request).unwrap();
        assert!(result.metadata().get("authorization").is_none());
    }
}
