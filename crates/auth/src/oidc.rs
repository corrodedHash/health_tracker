use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::reqwest;
use openidconnect::{
    ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge, RedirectUrl,
};
use serde::Deserialize;
use thiserror::Error;

pub type OidcClient = openidconnect::Client<
    openidconnect::EmptyAdditionalClaims,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreAuthPrompt,
    openidconnect::StandardErrorResponse<openidconnect::core::CoreErrorResponseType>,
    openidconnect::StandardTokenResponse<
        openidconnect::IdTokenFields<
            openidconnect::EmptyAdditionalClaims,
            openidconnect::EmptyExtraTokenFields,
            openidconnect::core::CoreGenderClaim,
            openidconnect::core::CoreJweContentEncryptionAlgorithm,
            openidconnect::core::CoreJwsSigningAlgorithm,
        >,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::StandardTokenIntrospectionResponse<
        openidconnect::EmptyExtraTokenFields,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::core::CoreRevocableToken,
    openidconnect::StandardErrorResponse<openidconnect::RevocationErrorResponseType>,
    openidconnect::EndpointSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointNotSet,
    openidconnect::EndpointMaybeSet,
    openidconnect::EndpointMaybeSet,
>;

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Error)]
pub enum OidcSetupError {
    #[error("failed to build HTTP client")]
    HttpClientBuild(#[source] reqwest::Error),

    #[error("failed to parse issuer URL: {0}")]
    IssuerUrlParse(#[source] openidconnect::url::ParseError),

    #[error("failed to discover OpenID provider metadata")]
    ProviderMetadataDiscovery(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("failed to parse redirect URI: {0}")]
    RedirectUriParse(String),
}

pub struct OidcClientBundle {
    pub http_client: reqwest::Client,
    pub client: OidcClient,
}

/// Set up an OIDC client by discovering the provider metadata.
///
/// # Errors
///
/// - [`OidcSetupError::HttpClientBuild`] if the HTTP client cannot be built.
/// - [`OidcSetupError::IssuerUrlParse`] if the issuer URL is invalid.
/// - [`OidcSetupError::ProviderMetadataDiscovery`] if the provider metadata cannot be discovered.
/// - [`OidcSetupError::RedirectUriParse`] if the redirect URI is invalid.
pub async fn setup_oidc_client(config: OidcConfig) -> Result<OidcClientBundle, OidcSetupError> {
    let http_client = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(OidcSetupError::HttpClientBuild)?;

    let provider_metadata = CoreProviderMetadata::discover_async(
        IssuerUrl::new(config.issuer_url).map_err(OidcSetupError::IssuerUrlParse)?,
        &http_client,
    )
    .await
    .map_err(|e| OidcSetupError::ProviderMetadataDiscovery(Box::new(e)))?;

    tracing::event!(
        tracing::Level::INFO,
        jwks = %provider_metadata.jwks_uri(),
        authorization = %provider_metadata.authorization_endpoint(),
        issuer = %provider_metadata.issuer(),
        "Loaded OIDC configuration"
    );

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new(config.client_id),
        Some(ClientSecret::new(config.client_secret)),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.redirect_uri.clone())
            .map_err(|_| OidcSetupError::RedirectUriParse(config.redirect_uri))?,
    );

    Ok(OidcClientBundle {
        http_client,
        client,
    })
}

#[derive(Debug, Clone)]
pub struct PkceChallenge {
    pub auth_url: String,
    pub csrf_token: String,
    pub nonce: String,
    pub pkce_verifier: String,
}

pub fn start_pkce_flow(client: &OidcClient) -> PkceChallenge {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .set_pkce_challenge(pkce_challenge)
        .url();

    PkceChallenge {
        auth_url: auth_url.to_string(),
        csrf_token: csrf_token.into_secret(),
        nonce: nonce.secret().clone(),
        pkce_verifier: pkce_verifier.into_secret(),
    }
}
