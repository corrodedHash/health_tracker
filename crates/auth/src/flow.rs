use health_core::{NewOidcState, OidcState};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, OAuth2TokenResponse, PkceCodeVerifier, TokenResponse,
};
use thiserror::Error;

use crate::oidc::OidcClientBundle;

#[derive(Debug, Error)]
pub enum LoginStartError {
    #[error("failed to create PKCE challenge")]
    PkceChallenge,
}

#[derive(Debug, Clone)]
pub struct LoginRequest {
    pub auth_url: String,
    pub state: NewOidcState,
}

/// Start an OIDC login flow by generating a PKCE challenge.
///
/// # Errors
///
/// Returns [`LoginStartError::PkceChallenge`] if the PKCE challenge generation fails.
pub fn start_login(
    bundle: &OidcClientBundle,
    resume_token: Option<String>,
) -> Result<LoginRequest, LoginStartError> {
    let challenge = crate::oidc::start_pkce_flow(&bundle.client);

    let state = NewOidcState {
        csrf: challenge.csrf_token,
        nonce: challenge.nonce,
        code_verifier: challenge.pkce_verifier,
        resume_token,
    };

    Ok(LoginRequest {
        auth_url: challenge.auth_url,
        state,
    })
}

#[derive(Debug, Error)]
pub enum LoginFinishError {
    #[error("CSRF token is missing or invalid")]
    MissingCsrf,

    #[error("failed to exchange authorization code for tokens")]
    TokenExchange(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("server did not return an ID token")]
    MissingIdToken,

    #[error("failed to parse ID token claims")]
    ParseIdToken(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to verify access token hash")]
    VerifyAccessTokenHash(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("access token hash mismatch")]
    AccessTokenHashMismatch,
}

#[derive(Debug, Clone)]
pub struct LoginResult {
    pub sub: String,
    pub resume_location: String,
}

/// Complete an OIDC login flow by exchanging the authorization code for tokens.
///
/// # Errors
///
/// - [`LoginFinishError::MissingCsrf`] if the CSRF token is missing or invalid.
/// - [`LoginFinishError::TokenExchange`] if the token exchange with the OIDC provider fails.
/// - [`LoginFinishError::MissingIdToken`] if the server did not return an ID token.
/// - [`LoginFinishError::ParseIdToken`] if the ID token claims cannot be parsed.
/// - [`LoginFinishError::VerifyAccessTokenHash`] if the access token hash verification fails.
/// - [`LoginFinishError::AccessTokenHashMismatch`] if the access token hash does not match.
pub async fn finish_login(
    bundle: &OidcClientBundle,
    code: &str,
    state: &OidcState,
) -> Result<LoginResult, LoginFinishError> {
    let token_response = bundle
        .client
        .exchange_code(AuthorizationCode::new(code.to_owned()))
        .map_err(|e| LoginFinishError::TokenExchange(Box::new(e)))?
        .set_pkce_verifier(PkceCodeVerifier::new(state.code_verifier.clone()))
        .request_async(&bundle.http_client)
        .await
        .map_err(|e| LoginFinishError::TokenExchange(Box::new(e)))?;

    let id_token = token_response
        .id_token()
        .ok_or(LoginFinishError::MissingIdToken)?;

    let id_token_verifier = bundle.client.id_token_verifier();
    let claims = id_token
        .claims(&id_token_verifier, |_: Option<&_>| Ok(()))
        .map_err(|e| LoginFinishError::ParseIdToken(Box::new(e)))?;

    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_access_token_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token
                .signing_alg()
                .map_err(|e| LoginFinishError::VerifyAccessTokenHash(Box::new(e)))?,
            id_token
                .signing_key(&id_token_verifier)
                .map_err(|e| LoginFinishError::VerifyAccessTokenHash(Box::new(e)))?,
        )
        .map_err(|e| LoginFinishError::VerifyAccessTokenHash(Box::new(e)))?;

        if actual_access_token_hash != *expected_access_token_hash {
            return Err(LoginFinishError::AccessTokenHashMismatch);
        }
    }

    let sub = claims.subject().to_string();

    let resume_location = state
        .resume_token
        .as_ref()
        .map_or_else(|| "/".to_owned(), |t| format!("/?resume_token={t}"));

    Ok(LoginResult {
        sub,
        resume_location,
    })
}
