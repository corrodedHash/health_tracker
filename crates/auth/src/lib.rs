pub mod flow;
pub mod oidc;
pub mod session;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claims {
    pub sub: String,
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("authentication failed")]
    Failed(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("token expired")]
    Expired,

    #[error("invalid token")]
    InvalidToken,
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn validate_token(&self, token: &str) -> Result<Claims, AuthError>;
}

pub struct OidcAuthProvider {
    #[expect(dead_code, reason = "will be used when OIDC validation is implemented")]
    bundle: oidc::OidcClientBundle,
}

impl OidcAuthProvider {
    #[must_use]
    pub const fn new(bundle: oidc::OidcClientBundle) -> Self {
        Self { bundle }
    }
}

#[async_trait]
impl AuthProvider for OidcAuthProvider {
    async fn validate_token(&self, _token: &str) -> Result<Claims, AuthError> {
        todo!("OIDC token validation via JWKS")
    }
}

pub struct MockAuthProvider {
    pub claims: Claims,
}

#[async_trait]
impl AuthProvider for MockAuthProvider {
    async fn validate_token(&self, _token: &str) -> Result<Claims, AuthError> {
        Ok(self.claims.clone())
    }
}
