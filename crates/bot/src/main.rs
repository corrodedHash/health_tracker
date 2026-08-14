//! `health-bot` binary entry point (item 5.27).
//!
//! Loads layered config, builds the matrix + API clients, and runs the
//! sync loop. GPX files received via Matrix are uploaded to the web API
//! using the sender's linked API token; a `link` command starts the
//! browser-confirmation flow that issues that token.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use health_bot::api_client::{
    ApiClient, ApiConfig, ApiError, LinkPoll, LinkStatus, ReqwestApiClient,
};
use health_bot::gpx::process_gpx;
use health_bot::links::LinksStore;
use health_bot::matrix_auth::MatrixLoginConfig;
use health_bot::matrix_client::{BotEvent, GpxFileMetadata, MatrixClient, MatrixSdkClient};
use matrix_sdk::ruma::{OwnedEventId, OwnedRoomId};
use serde::Deserialize;
use tokio::sync::mpsc;

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
    sync_token: PathBuf,
}

#[derive(Debug, Deserialize)]
struct ApiSection {
    base_url: String,
    token: String,
    #[serde(default = "default_links_file")]
    links_file: PathBuf,
    /// How long to keep polling a link before giving up (minutes).
    #[serde(default = "default_link_poll_timeout_minutes")]
    link_poll_timeout_minutes: u64,
}

fn default_links_file() -> PathBuf {
    PathBuf::from("links.toml")
}

const fn default_link_poll_timeout_minutes() -> u64 {
    5
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

    let matrix_client = MatrixSdkClient::new(
        &matrix_login,
        &cfg.matrix.session_file,
        &cfg.matrix.sync_token,
    )
    .await?;
    let event_sender = matrix_client.event_sender();

    let api_config = ApiConfig {
        base_url: cfg.api.base_url.clone(),
        token: cfg.api.token.clone(),
    };
    let api_client = ReqwestApiClient::new(api_config);

    let links_store = Arc::new(tokio::sync::Mutex::new(LinksStore::load(
        &cfg.api.links_file,
    )));
    let link_context = LinkContext {
        store: links_store.clone(),
        event_sender: event_sender.clone(),
        poll_timeout: Duration::from_secs(cfg.api.link_poll_timeout_minutes * 60),
    };

    tracing::info!("health-bot started, waiting for GPX files and link commands");

    while let Ok(event) = matrix_client.wait_for_event().await {
        match event {
            BotEvent::GpxFile { bytes, metadata } => {
                handle_gpx_event(&api_client, &links_store, &matrix_client, &bytes, &metadata)
                    .await;
            }
            BotEvent::LinkRequest {
                sender,
                room_id,
                event_id,
            } => {
                handle_link_request(
                    &api_client,
                    &link_context,
                    &matrix_client,
                    sender,
                    room_id,
                    event_id,
                )
                .await;
            }
            BotEvent::Reply {
                room_id,
                event_id,
                text,
            } => {
                let _ = matrix_client
                    .send_text_reply(&room_id, &event_id, &text)
                    .await;
            }
        }
    }

    tracing::info!("Matrix event channel closed, shutting down");
    Ok(())
}

/// Outcome of [`process_gpx_message`]: [`GpxOutcome::Unauthorized`] means
/// the API rejected the token with HTTP 401, so the caller must drop the
/// stale token from its link store. The function itself sends the
/// reply+reaction; token removal lives outside so unit tests only need
/// matrix/api mocks.
enum GpxOutcome {
    /// Upload completed — or was skipped, or failed with a non-auth error.
    Done,
    /// The API rejected the token; caller should drop it from the link store.
    Unauthorized,
}

/// Handle a [`BotEvent::GpxFile`]: look up the sender's linked API token,
/// delegate the upload to [`process_gpx_message`], and remove the token
/// if the API rejects it as unauthorized.
async fn handle_gpx_event(
    api: &ReqwestApiClient,
    links_store: &Arc<tokio::sync::Mutex<LinksStore>>,
    matrix: &dyn MatrixClient,
    bytes: &[u8],
    metadata: &GpxFileMetadata,
) {
    let sender = metadata.sender.to_string();
    let token = {
        let guard = links_store.lock().await;
        guard.token_for(&sender).map(str::to_owned)
    };
    let Some(token) = token else {
        let _ = matrix
            .send_text_reply(
                &metadata.room_id,
                &metadata.event_id,
                "You haven't linked your account yet — reply `link` to authenticate.",
            )
            .await;
        return;
    };

    if matches!(
        process_gpx_message(matrix, api, &token, bytes, metadata).await,
        GpxOutcome::Unauthorized
    ) {
        let _ = links_store.lock().await.remove_token(&sender);
    }
}

/// Testable GPX handler: skip already-acknowledged messages (client-side
/// idempotency — Phase 2), then upload + reply + react. Returns
/// [`GpxOutcome::Unauthorized`] only when the upload is rejected with 401,
/// so the production caller can drop the stale token; token removal
/// itself lives outside this function so unit tests can stub matrix/api
/// without dragging in the link store.
async fn process_gpx_message(
    matrix: &dyn MatrixClient,
    api: &dyn ApiClient,
    token: &str,
    bytes: &[u8],
    metadata: &GpxFileMetadata,
) -> GpxOutcome {
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
            return GpxOutcome::Done;
        }
        Ok(false) => {}
        Err(e) => tracing::warn!(
            "Could not check reactions for {} ({}): {e:#}, proceeding anyway",
            metadata.event_id,
            metadata.filename
        ),
    }

    match handle_gpx(api, token, bytes).await {
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
            GpxOutcome::Done
        }
        Err(e) => {
            tracing::error!("Failed to handle GPX file {}: {e:#}", metadata.filename);
            if e.downcast_ref::<ApiError>()
                .is_some_and(ApiError::is_unauthorized)
            {
                let _ = matrix
                    .send_text_reply(
                        &metadata.room_id,
                        &metadata.event_id,
                        "Your API token is invalid or revoked — reply `link` to re-link.",
                    )
                    .await;
                let _ = matrix
                    .send_reaction(&metadata.room_id, &metadata.event_id, "❌")
                    .await;
                GpxOutcome::Unauthorized
            } else {
                let _ = matrix
                    .send_text_reply(&metadata.room_id, &metadata.event_id, &format!("{e:#}"))
                    .await;
                let _ = matrix
                    .send_reaction(&metadata.room_id, &metadata.event_id, "❌")
                    .await;
                GpxOutcome::Done
            }
        }
    }
}

/// Shared state for a link flow: the token store, the main-loop event
/// channel, and how long to keep polling before giving up.
#[derive(Clone)]
struct LinkContext {
    store: Arc<tokio::sync::Mutex<LinksStore>>,
    event_sender: Arc<tokio::sync::Mutex<mpsc::Sender<BotEvent>>>,
    poll_timeout: Duration,
}

async fn handle_link_request(
    api: &ReqwestApiClient,
    link_ctx: &LinkContext,
    matrix: &dyn MatrixClient,
    sender: matrix_sdk::ruma::OwnedUserId,
    room_id: OwnedRoomId,
    event_id: OwnedEventId,
) {
    let sender_str = sender.to_string();
    let already_linked = {
        let guard = link_ctx.store.lock().await;
        guard.token_for(&sender_str).is_some()
    };
    if already_linked {
        let _ = matrix
            .send_text_reply(
                &room_id,
                &event_id,
                "You're already linked. Revoke the token in the web UI and reply `link` to re-link.",
            )
            .await;
        return;
    }

    match api.create_link().await {
        Ok(link) => {
            let _ = matrix
                .send_text_reply(
                    &room_id,
                    &event_id,
                    &format!("Authenticate here to link your account: {}", link.url),
                )
                .await;

            let api = api.clone();
            let ctx = link_ctx.clone();
            let code = link.code.clone();
            tokio::spawn(async move {
                poll_link_until_done(&api, ctx, &sender, &room_id, &event_id, &code).await;
            });
        }
        Err(e) => {
            tracing::error!("Failed to create link: {e:#}");
            let _ = matrix
                .send_text_reply(
                    &room_id,
                    &event_id,
                    &format!("Failed to start linking: {e:#}"),
                )
                .await;
        }
    }
}

/// Poll a link until the user accepts (or it expires), then store the
/// issued token and route a reply back through the main loop. Gives up
/// after `link_ctx.poll_timeout`, mirroring the server-side link TTL, so a
/// poll task can never outlive the link — e.g. while the web server is
/// unreachable.
async fn poll_link_until_done(
    api: &ReqwestApiClient,
    link_ctx: LinkContext,
    sender: &matrix_sdk::ruma::OwnedUserId,
    room_id: &OwnedRoomId,
    event_id: &OwnedEventId,
    code: &str,
) {
    let deadline = tokio::time::Instant::now() + link_ctx.poll_timeout;
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    loop {
        if tokio::time::Instant::now() >= deadline {
            let _ = link_ctx
                .event_sender
                .lock()
                .await
                .send(BotEvent::Reply {
                    room_id: room_id.clone(),
                    event_id: event_id.clone(),
                    text: "⌛ Link expired — reply `link` to start a new one.".to_owned(),
                })
                .await;
            return;
        }
        interval.tick().await;
        match api.poll_link(code).await {
            Ok(LinkPoll {
                status: LinkStatus::Pending,
                ..
            }) => {}
            Ok(LinkPoll {
                status: LinkStatus::Accepted,
                token: Some(token),
            }) => {
                let sender_str = sender.to_string();
                let store_result = {
                    let mut guard = link_ctx.store.lock().await;
                    guard.set_token(&sender_str, &token)
                };
                let text = match store_result {
                    Ok(()) => {
                        "✅ Linked to your account. Send me a .gpx file and I'll add it.".to_owned()
                    }
                    Err(e) => {
                        tracing::error!("Failed to persist token for {sender_str}: {e:#}");
                        "⚠️ Failed to save your link locally — reply `link` to try again."
                            .to_owned()
                    }
                };
                let _ = link_ctx
                    .event_sender
                    .lock()
                    .await
                    .send(BotEvent::Reply {
                        room_id: room_id.clone(),
                        event_id: event_id.clone(),
                        text,
                    })
                    .await;
                return;
            }
            Ok(LinkPoll {
                status: LinkStatus::Accepted,
                token: None,
            }) => {
                // A concurrent poll already collected the token.
                return;
            }
            Ok(LinkPoll {
                status: LinkStatus::Expired,
                ..
            }) => {
                let _ = link_ctx
                    .event_sender
                    .lock()
                    .await
                    .send(BotEvent::Reply {
                        room_id: room_id.clone(),
                        event_id: event_id.clone(),
                        text: "⌛ Link expired — reply `link` to start a new one.".to_owned(),
                    })
                    .await;
                return;
            }
            Err(e) => {
                tracing::error!("Polling link {code} failed: {e:#}");
            }
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

async fn handle_gpx(api: &dyn ApiClient, token: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let result = process_gpx(bytes)?;
    let id = api
        .post_run_gpx(
            token,
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
    use health_bot::matrix_client::{BotEvent, GpxFileMetadata, MatrixClient};

    struct CountingMatrix {
        has_own_reaction: bool,
        reaction_sent: Arc<AtomicBool>,
    }

    #[async_trait::async_trait]
    impl MatrixClient for CountingMatrix {
        async fn wait_for_event(&self) -> anyhow::Result<BotEvent> {
            anyhow::bail!("wait_for_event must not be called in unit tests")
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
            _token: &str,
            _bytes: &[u8],
            _started_at: chrono::DateTime<chrono::Utc>,
            _distance_m: f64,
            _duration: std::time::Duration,
        ) -> anyhow::Result<uuid::Uuid> {
            self.post_calls.fetch_add(1, Ordering::SeqCst);
            Ok(uuid::Uuid::new_v4())
        }

        async fn create_link(&self) -> anyhow::Result<health_bot::api_client::LinkInfo> {
            anyhow::bail!("create_link must not be called in unit tests")
        }

        async fn poll_link(&self, _code: &str) -> anyhow::Result<health_bot::api_client::LinkPoll> {
            anyhow::bail!("poll_link must not be called in unit tests")
        }
    }

    fn metadata() -> GpxFileMetadata {
        GpxFileMetadata {
            filename: "run.gpx".into(),
            sender: matrix_sdk::ruma::UserId::parse("@test:example.com").unwrap(),
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

        process_gpx_message(&matrix, &api, "any-token", b"<gpx>", &metadata()).await;

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

        process_gpx_message(&matrix, &api, "any-token", MINIMAL_GPX, &metadata()).await;

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
sync_token = "file-sync-token.txt"

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
            env::set_var("HEALTH__MATRIX__SYNC_TOKEN", "env-sync-token.txt");
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
            env::remove_var("HEALTH__MATRIX__SYNC_TOKEN");
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
        assert_eq!(
            config.matrix.sync_token.to_str(),
            Some("env-sync-token.txt")
        );
        assert_eq!(config.api.base_url, "http://web:3000");
        assert_eq!(config.api.token, "env-token");
        assert_eq!(config.api.links_file.to_str(), Some("links.toml"));
        assert_eq!(config.api.link_poll_timeout_minutes, 5);
    }
}
