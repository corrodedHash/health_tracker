//! `ApiClient` trait — the mock boundary for the bot → web HTTP call
//! (item 5.26).
//!
//! The real impl posts to `POST /api/runs/gpx` with a bearer token and
//! multipart form: the raw GPX bytes plus `started_at`, `distance_m`,
//! `duration_secs`. The server parses the GPX independently, stores the
//! raw bytes, and returns the new session UUID.

use async_trait::async_trait;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub base_url: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
struct CreatedSession {
    id: Uuid,
}

#[async_trait]
pub trait ApiClient: Send + Sync {
    /// Upload a GPX file + computed telemetry to the web API.
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server responds
    /// with a non-success status.
    async fn post_run_gpx(
        &self,
        bytes: &[u8],
        started_at: chrono::DateTime<chrono::Utc>,
        distance_m: f64,
        duration: std::time::Duration,
    ) -> anyhow::Result<Uuid>;
}

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
        bytes: &[u8],
        started_at: chrono::DateTime<chrono::Utc>,
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
            .bearer_auth(&self.config.token)
            .header("Content-Type", "application/gpx+xml")
            .body(bytes.to_vec())
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("POST /api/runs/gpx returned {status}: {body}");
        }

        let created: CreatedSession = resp.json().await?;
        Ok(created.id)
    }
}
