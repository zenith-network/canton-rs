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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_token() {
        let token = StaticToken::new("my-jwt");
        assert_eq!(token.token(), Some("my-jwt".to_string()));
    }
}
