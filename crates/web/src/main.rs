use std::sync::Arc;

use anyhow::Context;
use sqlx::PgPool;
use tracing_subscriber::EnvFilter;

mod config;
mod error;
mod middleware;
mod routes;
mod state;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = config::Config::load()?;

    let pool = PgPool::connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    health_db::run_migrations(&pool).await?;

    let cookie_key_bytes =
        hex::decode(&config.cookie_key).context("invalid cookie key hex")?;
    let cookie_key = cookie::Key::from(&cookie_key_bytes[..]);

    let oidc_bundle = if let Some(oidc_config) = &config.oidc {
        let oidc_cfg = health_auth::oidc::OidcConfig {
            issuer_url: oidc_config.issuer_url.clone(),
            client_id: oidc_config.client_id.clone(),
            client_secret: oidc_config.client_secret.clone(),
            redirect_uri: oidc_config.redirect_uri.clone(),
        };
        let bundle = health_auth::oidc::setup_oidc_client(oidc_cfg)
            .await
            .context("OIDC setup failed")?;
        Some(Arc::new(bundle))
    } else {
        None
    };

    let state = state::AppState {
        pool,
        config: config.clone(),
        cookie_key,
        oidc_bundle,
    };

    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(&config.listen_addr)
        .await
        .context("failed to bind")?;

    tracing::info!("listening on {}", config.listen_addr);

    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}
