//! `ApiClient` trait — the mock boundary for the bot → web HTTP call.
//!
//! GPX uploads now authenticate with the *sender's* API token (bound to
//! the confirming user), while the account-link endpoints use the bot's
//! own service token from config.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
    pub base_url: String,
    pub token: String,
}

/// A freshly minted account link, returned by `POST /api/links`.
#[derive(Debug, Clone, Deserialize)]
pub struct LinkInfo {
    pub code: String,
    /// Browser-facing confirmation URL, as sent by the server.
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStatus {
    Pending,
    Accepted,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPoll {
    pub status: LinkStatus,
    /// Present only on the first poll after the user accepts.
    pub token: Option<String>,
}

/// Typed error so the sync loop can distinguish an unauthorized upload
/// (stale/revoked token) from other failures.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API returned {status}: {body}")]
    Status {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl ApiError {
    #[must_use]
    pub fn is_unauthorized(&self) -> bool {
        matches!(
            self,
            Self::Status {
                status,
                ..
            } if *status == reqwest::StatusCode::UNAUTHORIZED
        )
    }
}

#[derive(Debug, Deserialize)]
struct CreatedSession {
    id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawLinkStatus {
    Pending,
    Accepted,
    Expired,
}

#[derive(Debug, Deserialize)]
struct LinkPollDto {
    status: RawLinkStatus,
    token: Option<String>,
}

#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Upload a GPX file + computed telemetry to the web API, using the
    /// given per-user token.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server responds
    /// with a non-success status.
    async fn post_run_gpx(
        &self,
        token: &str,
        bytes: &[u8],
        started_at: DateTime<Utc>,
        distance_m: f64,
        duration: std::time::Duration,
    ) -> anyhow::Result<Uuid>;

    /// Mint an account link. Authenticates with the bot's service token.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server responds
    /// with a non-success status.
    async fn create_link(&self) -> anyhow::Result<LinkInfo>;

    /// Poll a link's status. Authenticates with the bot's service token.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server responds
    /// with a non-success status.
    async fn poll_link(&self, code: &str) -> anyhow::Result<LinkPoll>;
}

#[derive(Debug, Clone)]
pub struct ReqwestApiClient {
    client: reqwest::Client,
    config: ApiConfig,
}

impl ReqwestApiClient {
    #[must_use]
    pub fn new(config: ApiConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    #[must_use]
    pub const fn with_client(config: ApiConfig, client: reqwest::Client) -> Self {
        Self { client, config }
    }
}

#[async_trait]
impl ApiClient for ReqwestApiClient {
    #[allow(clippy::unused_self, unused_variables)]
    async fn post_run_gpx(
        &self,
        token: &str,
        bytes: &[u8],
        started_at: DateTime<Utc>,
        distance_m: f64,
        duration: std::time::Duration,
    ) -> anyhow::Result<Uuid> {
        let url = format!(
            "{}/api/runs/gpx",
            self.config.base_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/gpx+xml")
            .body(bytes.to_vec())
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Status { status, body }.into());
        }

        let created: CreatedSession = resp.json().await?;
        Ok(created.id)
    }

    async fn create_link(&self) -> anyhow::Result<LinkInfo> {
        let url = format!("{}/api/links", self.config.base_url.trim_end_matches('/'));

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.config.token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Status { status, body }.into());
        }

        Ok(resp.json().await?)
    }

    async fn poll_link(&self, code: &str) -> anyhow::Result<LinkPoll> {
        let url = format!(
            "{}/api/links/{code}",
            self.config.base_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.config.token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ApiError::Status { status, body }.into());
        }

        let dto: LinkPollDto = resp.json().await?;
        let link_status = match dto.status {
            RawLinkStatus::Pending => LinkStatus::Pending,
            RawLinkStatus::Accepted => LinkStatus::Accepted,
            RawLinkStatus::Expired => LinkStatus::Expired,
        };
        Ok(LinkPoll {
            status: link_status,
            token: dto.token,
        })
    }
}
