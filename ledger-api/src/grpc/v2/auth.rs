use std::{io::Read as _, path::PathBuf, sync::Arc};

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
    token_file: Option<PathBuf>,
}

impl AuthInterceptor {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            token_file: None,
        }
    }

    /// Reload a host-managed bearer file for each new RPC. Missing, empty or
    /// malformed files fail closed; no old token or anonymous fallback is used.
    /// Mount the containing directory when tokens are replaced atomically.
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            token: None,
            token_file: Some(path.into()),
        }
    }

    fn current_token(&self) -> Result<Option<String>, tonic::Status> {
        let Some(path) = &self.token_file else {
            return Ok(self.token.clone());
        };
        let fail = || tonic::Status::unauthenticated("ledger token file unavailable or invalid");
        let file = std::fs::File::open(path).map_err(|_| fail())?;
        let mut text = String::new();
        file.take(16_385)
            .read_to_string(&mut text)
            .map_err(|_| fail())?;
        let token = text.trim();
        if text.len() > 16_384
            || token.is_empty()
            || token
                .bytes()
                .any(|b| b.is_ascii_whitespace() || !b.is_ascii())
        {
            return Err(fail());
        }
        Ok(Some(token.to_owned()))
    }
}

impl Interceptor for AuthInterceptor {
    fn call(
        &mut self,
        mut request: tonic::Request<()>,
    ) -> Result<tonic::Request<()>, tonic::Status> {
        if let Some(token) = self.current_token()? {
            let mut value: tonic::metadata::MetadataValue<_> =
                format!("Bearer {token}").parse().map_err(|parse_error| {
                    let mut err = tonic::Status::internal("invalid auth token");
                    err.set_source(Arc::new(parse_error));
                    err
                })?;
            value.set_sensitive(true);
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
    #[test]
    fn token_file_rotation_is_seen_by_existing_clones() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        std::fs::write(&path, "first\n").unwrap();
        let mut original = AuthInterceptor::from_file(&path);
        let mut clone = original.clone();
        let first = original.call(tonic::Request::new(())).unwrap();
        assert_eq!(
            first.metadata().get("authorization").unwrap(),
            "Bearer first"
        );
        assert!(
            first
                .metadata()
                .get("authorization")
                .unwrap()
                .is_sensitive()
        );
        let next = dir.path().join("next");
        std::fs::write(&next, "second\n").unwrap();
        std::fs::rename(next, &path).unwrap();
        let second = clone.call(tonic::Request::new(())).unwrap();
        assert_eq!(
            second.metadata().get("authorization").unwrap(),
            "Bearer second"
        );
        std::fs::remove_file(path).unwrap();
        assert_eq!(
            original.call(tonic::Request::new(())).unwrap_err().code(),
            tonic::Code::Unauthenticated
        );
    }

    #[test]
    fn bad_file_never_uses_anonymous_or_previous_credentials() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("token");
        let mut auth = AuthInterceptor::from_file(&path);
        for bad in [
            String::new(),
            " \n".into(),
            "secret\r\ninjected".into(),
            "x".repeat(16_385),
            "\u{ff}".into(),
        ] {
            std::fs::write(&path, &bad).unwrap();
            let err = auth.call(tonic::Request::new(())).unwrap_err();
            assert_eq!(err.code(), tonic::Code::Unauthenticated);
            assert_eq!(err.message(), "ledger token file unavailable or invalid");
        }
        std::fs::write(&path, "recovered").unwrap();
        assert_eq!(
            auth.call(tonic::Request::new(()))
                .unwrap()
                .metadata()
                .get("authorization")
                .unwrap(),
            "Bearer recovered"
        );
    }
}
