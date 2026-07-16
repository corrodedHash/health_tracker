//! `health-bot` binary entry point (item 5.27).
//!
//! Loads layered config, builds the matrix + API clients, and runs the
//! sync loop. Each GPX file received via Matrix is parsed, computed,
//! and uploaded to the web API.

use std::path::PathBuf;

use health_bot::api_client::{ApiConfig, ApiClient, ReqwestApiClient};
use health_bot::gpx::process_gpx;
use health_bot::matrix_auth::MatrixLoginConfig;
use health_bot::matrix_client::{MatrixClient, MatrixSdkClient};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct BotConfig {
    matrix: MatrixSection,
    api: ApiSection,
}

#[derive(Debug, Deserialize)]
struct MatrixSection {
    homeserver: String,
    user_id: String,
    password: String,
    session_file: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ApiSection {
    base_url: String,
    token: String,
}

fn load_config() -> anyhow::Result<BotConfig> {
    let config = config::Config::builder()
        .add_source(config::File::with_name("config/default").required(false))
        .add_source(config::Environment::with_prefix("HEALTH"))
        .build()?;

    let bot_config: BotConfig = config.try_deserialize()?;
    Ok(bot_config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = load_config()?;
    if cfg.api.token.is_empty() {
        anyhow::bail!("API token is empty — set HEALTH_API__TOKEN");
    }
    if cfg.matrix.user_id.is_empty() || cfg.matrix.password.is_empty() {
        anyhow::bail!(
            "Matrix credentials missing — set HEALTH_MATRIX__USER_ID and HEALTH_MATRIX__PASSWORD"
        );
    }

    let matrix_login = MatrixLoginConfig {
        homeserver: cfg.matrix.homeserver.clone(),
        user_id: cfg.matrix.user_id.clone(),
        password: cfg.matrix.password.clone(),
    };

    let mut matrix_client = MatrixSdkClient::new(&matrix_login, &cfg.matrix.session_file).await?;

    let api_config = ApiConfig {
        base_url: cfg.api.base_url.clone(),
        token: cfg.api.token.clone(),
    };
    let api_client = ReqwestApiClient::new(api_config);

    tracing::info!("health-bot started, waiting for GPX files");

    while let Ok((bytes, metadata)) = matrix_client.wait_for_gpx_file().await {
        tracing::info!("Processing GPX file: {}", metadata.filename);
        if let Err(e) = handle_gpx(&api_client, &bytes).await {
            tracing::error!("Failed to handle GPX file {}: {e:#}", metadata.filename);
        }
    }

    tracing::info!("Matrix event channel closed, shutting down");
    Ok(())
}

async fn handle_gpx<A: ApiClient>(api: &A, bytes: &[u8]) -> anyhow::Result<()> {
    let result = process_gpx(bytes)?;
    let id = api
        .post_run_gpx(bytes, result.started_at, result.distance_m, result.duration)
        .await?;
    tracing::info!(
        "Uploaded run: {}m in {:.1}s on {} -> session {id}",
        result.distance_m,
        result.duration.as_secs_f64(),
        result.started_at
    );
    Ok(())
}