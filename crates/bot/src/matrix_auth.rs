//! Matrix session persistence.
//!
//! Ported from `matrix-running/src/auth.rs` (item 5.24). The
//! `restore_session` / `login` functions are preserved; `get_client`
//! now takes a typed config struct (loaded via the `config` crate from
//! `main.rs`) rather than reading a TOML login file.

use std::path::Path;

use anyhow::Context;
use matrix_sdk::Client;
use matrix_sdk::authentication::matrix::MatrixSession;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct MatrixLoginConfig {
    pub homeserver: String,
    pub user_id: String,
    pub password: String,
}

async fn restore_session(homeserver: &str, session_file: &Path) -> anyhow::Result<Client> {
    let client = Client::builder().homeserver_url(homeserver).build().await?;

    let raw = std::fs::read_to_string(session_file)
        .with_context(|| format!("reading session file {}", session_file.display()))?;
    let session: MatrixSession = toml::from_str(&raw).context("parsing session file")?;
    client.restore_session(session).await?;

    Ok(client)
}

async fn login(session_file: &Path, login_info: &MatrixLoginConfig) -> anyhow::Result<Client> {
    let client = Client::builder()
        .homeserver_url(&login_info.homeserver)
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(&login_info.user_id, &login_info.password)
        .initial_device_display_name("health-bot")
        .await?;

    let matrix_auth = client.matrix_auth();
    let session = matrix_auth
        .session()
        .ok_or_else(|| anyhow::anyhow!("Could not extract session from client"))?;

    std::fs::write(session_file, toml::to_string(&session)?.as_bytes())?;
    Ok(client)
}

/// Get a logged-in Matrix client, restoring a cached session when
/// `session_file` exists and falling back to a fresh password login.
///
/// # Errors
/// Returns an error if the homeserver is unreachable, the cached
/// session file cannot be parsed, or the password login fails.
pub async fn get_client(
    login_info: &MatrixLoginConfig,
    session_file: &Path,
) -> anyhow::Result<Client> {
    if session_file.exists() {
        restore_session(&login_info.homeserver, session_file).await
    } else {
        login(session_file, login_info).await
    }
}