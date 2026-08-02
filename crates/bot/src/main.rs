//! `health-bot` binary entry point (item 5.27).
//!
//! Loads layered config, builds the matrix + API clients, and runs the
//! sync loop. Each GPX file received via Matrix is parsed, computed,
//! and uploaded to the web API.

use std::path::PathBuf;
use std::time::Duration;

use health_bot::api_client::{ApiClient, ApiConfig, ReqwestApiClient};
use health_bot::gpx::process_gpx;
use health_bot::matrix_auth::MatrixLoginConfig;
use health_bot::matrix_client::{GpxFileMetadata, MatrixClient, MatrixSdkClient};
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
        .add_source(config::File::with_name("config/local").required(false))
        .add_source(config::Environment::with_prefix("HEALTH").separator("__"))
        .build()?;

    let bot_config: BotConfig = config.try_deserialize()?;
    Ok(bot_config)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cfg = load_config()?;
    if cfg.api.token.is_empty() {
        anyhow::bail!("API token is empty — set HEALTH__API__TOKEN");
    }
    if cfg.matrix.user_id.is_empty() || cfg.matrix.password.is_empty() {
        anyhow::bail!(
            "Matrix credentials missing — set HEALTH__MATRIX__USER_ID and HEALTH__MATRIX__PASSWORD"
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
        process_gpx_message(&matrix_client, &api_client, &bytes, &metadata).await;
    }

    tracing::info!("Matrix event channel closed, shutting down");
    Ok(())
}

/// Handle one GPX message: skip it if the bot already reacted (✅/❌),
/// otherwise upload + reply + react. Extracted from the `main()` loop so the
/// reaction-skip logic is testable through the mockable traits.
async fn process_gpx_message<M: MatrixClient + Sync, A: ApiClient + Sync>(
    matrix: &M,
    api: &A,
    bytes: &[u8],
    metadata: &GpxFileMetadata,
) {
    tracing::info!("Processing GPX file: {}", metadata.filename);

    match matrix
        .has_own_reaction(&metadata.room_id, &metadata.event_id, &["✅", "❌"])
        .await
    {
        Ok(true) => {
            tracing::info!(
                "Skipping {}: bot already reacted to {}",
                metadata.filename,
                metadata.event_id
            );
            return;
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(
            "Could not check reactions for {} ({}): {e:#}, proceeding anyway",
            metadata.event_id,
            metadata.filename
        ),
    }

    match handle_gpx(api, bytes).await {
        Ok(()) => {
            if let Ok(result) = process_gpx(bytes) {
                let (plain, html) = format_gpx_result(&result);
                let _ = matrix
                    .send_html_reply(&metadata.room_id, &metadata.event_id, &plain, &html)
                    .await;
            }
            let _ = matrix
                .send_reaction(&metadata.room_id, &metadata.event_id, "✅")
                .await;
        }
        Err(e) => {
            tracing::error!("Failed to handle GPX file {}: {e:#}", metadata.filename);
            let msg = format!("{e:#}");
            let _ = matrix
                .send_text_reply(&metadata.room_id, &metadata.event_id, &msg)
                .await;
            let _ = matrix
                .send_reaction(&metadata.room_id, &metadata.event_id, "❌")
                .await;
        }
    }
}

fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn format_pace(d: Duration, dist_m: f64) -> String {
    let secs_per_km = if dist_m > 0.0 {
        d.as_secs_f64() / (dist_m / 1000.0)
    } else {
        0.0
    };
    let m = secs_per_km as u64 / 60;
    let s = secs_per_km as u64 % 60;
    format!("{m}:{s:02}/km")
}

fn format_gpx_result(r: &health_bot::gpx::GpxResult) -> (String, String) {
    let dist_km = |m: f64| m / 1000.0;

    let plain = format!(
        concat!(
            "Run uploaded ✅\n",
            "Total: {:.2} km in {} ({})\n",
            "Moving: {:.2} km in {} ({})",
        ),
        dist_km(r.total_distance_m),
        format_duration(r.total_duration),
        format_pace(r.total_duration, r.total_distance_m),
        dist_km(r.moving_distance_m),
        format_duration(r.moving_duration),
        format_pace(r.moving_duration, r.moving_distance_m),
    );

    let html = format!(
        concat!(
            "<strong>Run uploaded</strong> ✅<br>",
            "Total: <strong>{:.2} km</strong> in {} (<em>{}</em>)<br>",
            "Moving: <strong>{:.2} km</strong> in {} (<em>{}</em>)",
        ),
        dist_km(r.total_distance_m),
        format_duration(r.total_duration),
        format_pace(r.total_duration, r.total_distance_m),
        dist_km(r.moving_distance_m),
        format_duration(r.moving_duration),
        format_pace(r.moving_duration, r.moving_distance_m),
    );

    (plain, html)
}

async fn handle_gpx<A: ApiClient>(api: &A, bytes: &[u8]) -> anyhow::Result<()> {
    let result = process_gpx(bytes)?;
    let id = api
        .post_run_gpx(
            bytes,
            result.started_at,
            result.total_distance_m,
            result.total_duration,
        )
        .await?;
    tracing::info!(
        "Uploaded run: total {}m / moving {}m in {:.1}s on {} -> session {id}",
        result.total_distance_m,
        result.moving_distance_m,
        result.total_duration.as_secs_f64(),
        result.started_at
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, unsafe_code)]

    use std::env;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    use super::{load_config, process_gpx_message};
    use health_bot::api_client::ApiClient;
    use health_bot::matrix_client::{GpxFileMetadata, MatrixClient};

    struct CountingMatrix {
        has_own_reaction: bool,
        reaction_sent: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl MatrixClient for CountingMatrix {
        async fn wait_for_gpx_file(&mut self) -> anyhow::Result<(Vec<u8>, GpxFileMetadata)> {
            anyhow::bail!("wait_for_gpx_file must not be called in unit tests")
        }

        async fn send_text_reply(
            &self,
            _room_id: &matrix_sdk::ruma::RoomId,
            _event_id: &matrix_sdk::ruma::EventId,
            _text: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn send_html_reply(
            &self,
            _room_id: &matrix_sdk::ruma::RoomId,
            _event_id: &matrix_sdk::ruma::EventId,
            _plain: &str,
            _html: &str,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        async fn send_reaction(
            &self,
            _room_id: &matrix_sdk::ruma::RoomId,
            _event_id: &matrix_sdk::ruma::EventId,
            _key: &str,
        ) -> anyhow::Result<()> {
            self.reaction_sent.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn has_own_reaction(
            &self,
            _room_id: &matrix_sdk::ruma::RoomId,
            _event_id: &matrix_sdk::ruma::EventId,
            _keys: &[&str],
        ) -> anyhow::Result<bool> {
            Ok(self.has_own_reaction)
        }
    }

    struct CountingApi {
        post_calls: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl ApiClient for CountingApi {
        async fn post_run_gpx(
            &self,
            _bytes: &[u8],
            _started_at: chrono::DateTime<chrono::Utc>,
            _distance_m: f64,
            _duration: std::time::Duration,
        ) -> anyhow::Result<uuid::Uuid> {
            self.post_calls.fetch_add(1, Ordering::SeqCst);
            Ok(uuid::Uuid::new_v4())
        }
    }

    fn metadata() -> GpxFileMetadata {
        GpxFileMetadata {
            filename: "run.gpx".into(),
            room_id: matrix_sdk::ruma::RoomId::parse("!room:example.com").unwrap(),
            event_id: matrix_sdk::ruma::EventId::parse("$event:example.com").unwrap(),
        }
    }

    #[tokio::test]
    async fn already_reacted_message_is_skipped_before_upload() {
        let post_calls = Arc::new(AtomicUsize::new(0));
        let reaction_sent = Arc::new(AtomicBool::new(false));
        let matrix = CountingMatrix {
            has_own_reaction: true,
            reaction_sent: reaction_sent.clone(),
        };
        let api = CountingApi {
            post_calls: post_calls.clone(),
        };

        process_gpx_message(&matrix, &api, b"<gpx>", &metadata()).await;

        assert_eq!(
            post_calls.load(Ordering::SeqCst),
            0,
            "a message the bot already reacted to must not be re-uploaded"
        );
        assert!(
            !reaction_sent.load(Ordering::SeqCst),
            "no reply/reaction should be sent for a skipped message"
        );
    }

    #[tokio::test]
    async fn unreacted_message_is_uploaded() {
        let post_calls = Arc::new(AtomicUsize::new(0));
        let reaction_sent = Arc::new(AtomicBool::new(false));
        let matrix = CountingMatrix {
            has_own_reaction: false,
            reaction_sent: reaction_sent.clone(),
        };
        let api = CountingApi {
            post_calls: post_calls.clone(),
        };

        process_gpx_message(&matrix, &api, MINIMAL_GPX, &metadata()).await;

        assert_eq!(
            post_calls.load(Ordering::SeqCst),
            1,
            "an unreacted GPX message must be uploaded"
        );
        assert!(
            reaction_sent.load(Ordering::SeqCst),
            "success reaction should be sent"
        );
    }

    const MINIMAL_GPX: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="health-tracker-test" xmlns="http://www.topografix.com/GPX/1/1">
  <metadata><time>2026-07-16T08:00:00Z</time></metadata>
  <trk>
    <trkseg>
      <trkpt lat="50.0" lon="6.0"><time>2026-07-16T08:00:00Z</time></trkpt>
      <trkpt lat="50.009" lon="6.009"><time>2026-07-16T08:05:00Z</time></trkpt>
    </trkseg>
  </trk>
</gpx>
"#;

    #[test]
    fn env_vars_override_file_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config");
        std::fs::create_dir_all(&config_path).unwrap();

        std::fs::write(
            config_path.join("default.toml"),
            r#"
[matrix]
homeserver = "https://file.example.com"
user_id = "@file:example.com"
password = "file-password"
session_file = "file-session.toml"

[api]
base_url = "http://file:3000"
token = "file-token"
"#,
        )
        .unwrap();

        let original_cwd = env::current_dir().unwrap();
        env::set_current_dir(dir.path()).unwrap();

        // SAFETY: single-threaded test — no concurrent env access
        unsafe {
            env::set_var("HEALTH__MATRIX__HOMESERVER", "https://matrix.example.com");
            env::set_var("HEALTH__MATRIX__USER_ID", "@bot:matrix.example.com");
            env::set_var("HEALTH__MATRIX__PASSWORD", "env-password");
            env::set_var("HEALTH__MATRIX__SESSION_FILE", "env-session.toml");
            env::set_var("HEALTH__API__BASE_URL", "http://web:3000");
            env::set_var("HEALTH__API__TOKEN", "env-token");
        }

        let result = load_config();

        // SAFETY: single-threaded test — no concurrent env access
        unsafe {
            env::remove_var("HEALTH__MATRIX__HOMESERVER");
            env::remove_var("HEALTH__MATRIX__USER_ID");
            env::remove_var("HEALTH__MATRIX__PASSWORD");
            env::remove_var("HEALTH__MATRIX__SESSION_FILE");
            env::remove_var("HEALTH__API__BASE_URL");
            env::remove_var("HEALTH__API__TOKEN");
        }
        env::set_current_dir(original_cwd).unwrap();

        let config = result.unwrap();
        assert_eq!(config.matrix.homeserver, "https://matrix.example.com");
        assert_eq!(config.matrix.user_id, "@bot:matrix.example.com");
        assert_eq!(config.matrix.password, "env-password");
        assert_eq!(
            config.matrix.session_file.to_str(),
            Some("env-session.toml")
        );
        assert_eq!(config.api.base_url, "http://web:3000");
        assert_eq!(config.api.token, "env-token");
    }
}
