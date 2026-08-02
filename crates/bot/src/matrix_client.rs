//! `MatrixClient` trait — the mock boundary between the bot's sync loop
//! and the matrix-sdk (item 5.25).
//!
//! The real impl wraps `matrix-sdk` and yields GPX file bytes received in
//! any joined room. The test impl returns fixture bytes so the sync loop
//! can be exercised without a live Matrix connection.

use std::path::Path;

use async_trait::async_trait;
use matrix_sdk::{
    Client, Error, LoopCtrl, Room, RoomState,
    config::SyncSettings,
    media::{MediaFormat, MediaRequestParameters},
    ruma::{
        EventId, OwnedEventId, OwnedRoomId, RoomId,
        api::client::relations::get_relating_events_with_rel_type,
        events::{
            AnyMessageLikeEvent,
            reaction::ReactionEventContent,
            relation::{Annotation, InReplyTo, RelationType},
            room::{
                member::StrippedRoomMemberEvent,
                message::{
                    MessageType, OriginalSyncRoomMessageEvent, Relation, RoomMessageEventContent,
                },
            },
        },
    },
    sync::SyncResponse,
};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::matrix_auth::{MatrixLoginConfig, get_client};

#[derive(Debug, Clone)]
pub struct GpxFileMetadata {
    pub filename: String,
    pub room_id: OwnedRoomId,
    pub event_id: OwnedEventId,
}

type GpxSender = mpsc::Sender<(Vec<u8>, GpxFileMetadata)>;
type GpxSenderCtx = matrix_sdk::event_handler::Ctx<std::sync::Arc<tokio::sync::Mutex<GpxSender>>>;

#[async_trait]
pub trait MatrixClient: Send {
    async fn wait_for_gpx_file(&mut self) -> anyhow::Result<(Vec<u8>, GpxFileMetadata)>;

    async fn send_text_reply(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        text: &str,
    ) -> anyhow::Result<()>;

    async fn send_html_reply(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        plain: &str,
        html: &str,
    ) -> anyhow::Result<()>;

    async fn send_reaction(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        key: &str,
    ) -> anyhow::Result<()>;

    /// Whether the bot has already reacted to `event_id` in `room_id` with
    /// any of the given reaction keys (e.g. ✅ / ❌).
    ///
    /// Used as a client-side idempotency filter: on restart, the bot
    /// re-sees messages it already processed and skipped them so it does
    /// not re-upload the GPX.
    async fn has_own_reaction(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        keys: &[&str],
    ) -> anyhow::Result<bool>;
}

pub struct MatrixSdkClient {
    client: Client,
    rx: mpsc::Receiver<(Vec<u8>, GpxFileMetadata)>,
}

impl MatrixSdkClient {
    /// Build a Matrix client that yields GPX files received in joined rooms.
    ///
    /// The initial sync resumes from a previously persisted sync token in
    /// `sync_token_file` (falling back to a fresh sync when absent), and a
    /// background loop keeps syncing, persisting each response's `next_batch`
    /// so a restart does not re-deliver already-seen events.
    ///
    /// # Errors
    /// Returns an error if the Matrix client cannot be built, the login
    /// fails, or the initial sync does not complete.
    pub async fn new(
        login: &MatrixLoginConfig,
        session_file: &Path,
        sync_token_file: &Path,
    ) -> anyhow::Result<Self> {
        let client = get_client(login, session_file).await?;

        client.add_event_handler(on_stripped_state_member);

        let (tx, rx) = mpsc::channel::<(Vec<u8>, GpxFileMetadata)>(16);
        let tx = std::sync::Arc::new(tokio::sync::Mutex::new(tx));
        client.add_event_handler_context(tx);
        client.add_event_handler(on_room_message);

        let settings = read_sync_token(sync_token_file)
            .map_or_else(SyncSettings::default, |prev| {
                SyncSettings::default().token(prev)
            });
        let initial_token = client.sync_once(settings).await?.next_batch;
        if let Err(e) = persist_sync_token(sync_token_file, &initial_token) {
            tracing::error!(
                "Failed to persist initial sync token to {}: {e:#}",
                sync_token_file.display()
            );
        }
        tracing::info!("Finished initial Matrix sync");

        let sync_client = client.clone();
        let sync_token_file = sync_token_file.to_owned();
        tokio::spawn(async move {
            // `sync_with_result_callback` syncs forever; the callback persists
            // each `next_batch` and swallows errors so the loop survives
            // transient failures (log-and-continue).
            if let Err(e) = sync_client
                .sync_with_result_callback(SyncSettings::default().token(initial_token), |result| {
                    persist_next_batch(&sync_token_file, result)
                })
                .await
            {
                tracing::error!("Matrix sync loop ended with error: {e}");
            }
        });

        Ok(Self { client, rx })
    }
}

/// Read the persisted sync token, if any. A missing file (or an unreadable
/// one, logged) falls back to a fresh sync — the server-side dedup makes a
/// re-sync of recent events safe.
fn read_sync_token(sync_token_file: &Path) -> Option<String> {
    match std::fs::read_to_string(sync_token_file) {
        Ok(token) => {
            let token = token.trim();
            if token.is_empty() {
                None
            } else {
                Some(token.to_owned())
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::error!(
                "Failed to read sync token from {}: {e:#}",
                sync_token_file.display()
            );
            None
        }
    }
}

/// Write `token` to `sync_token_file`, creating any missing parent
/// directories.
///
/// # Errors
/// Returns an error if the parent directory cannot be created or the file
/// cannot be written.
fn persist_sync_token(sync_token_file: &Path, token: &str) -> anyhow::Result<()> {
    if let Some(parent) = sync_token_file.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(sync_token_file, token)?;
    Ok(())
}

/// `sync_with_result_callback` callback: persist each successful response's
/// `next_batch` to disk and keep syncing. Errors are logged and swallowed so
/// the loop keeps going (log-and-continue).
fn persist_next_batch(
    sync_token_file: &Path,
    result: Result<SyncResponse, Error>,
) -> std::future::Ready<Result<LoopCtrl, Error>> {
    match result {
        Ok(response) => {
            if let Err(e) = persist_sync_token(sync_token_file, &response.next_batch) {
                tracing::error!(
                    "Failed to persist sync token to {}: {e:#}",
                    sync_token_file.display()
                );
            }
        }
        Err(e) => tracing::error!("Matrix sync ended with error: {e}"),
    }
    std::future::ready(Ok(LoopCtrl::Continue))
}

#[async_trait]
impl MatrixClient for MatrixSdkClient {
    async fn wait_for_gpx_file(&mut self) -> anyhow::Result<(Vec<u8>, GpxFileMetadata)> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("matrix event channel closed"))
    }

    async fn send_text_reply(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        text: &str,
    ) -> anyhow::Result<()> {
        let room = self
            .client
            .get_room(room_id)
            .ok_or_else(|| anyhow::anyhow!("room {room_id} not found"))?;

        let mut content = RoomMessageEventContent::text_plain(text);
        content.relates_to = Some(Relation::Reply {
            in_reply_to: InReplyTo::new(event_id.to_owned()),
        });
        room.send(content).await?;
        Ok(())
    }

    async fn send_html_reply(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        plain: &str,
        html: &str,
    ) -> anyhow::Result<()> {
        let room = self
            .client
            .get_room(room_id)
            .ok_or_else(|| anyhow::anyhow!("room {room_id} not found"))?;

        let mut content = RoomMessageEventContent::text_html(plain, html);
        content.relates_to = Some(Relation::Reply {
            in_reply_to: InReplyTo::new(event_id.to_owned()),
        });
        room.send(content).await?;
        Ok(())
    }

    async fn send_reaction(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        key: &str,
    ) -> anyhow::Result<()> {
        let room = self
            .client
            .get_room(room_id)
            .ok_or_else(|| anyhow::anyhow!("room {room_id} not found"))?;

        let content =
            ReactionEventContent::new(Annotation::new(event_id.to_owned(), key.to_owned()));
        room.send(content).await?;
        Ok(())
    }

    async fn has_own_reaction(
        &self,
        room_id: &RoomId,
        event_id: &EventId,
        keys: &[&str],
    ) -> anyhow::Result<bool> {
        has_own_reaction(&self.client, room_id, event_id, keys).await
    }
}

/// Check whether `client`'s own user has reacted to `event_id` with any of
/// `keys`. matrix-sdk 0.11 has no `Room::event_relations`, so this uses the
/// raw ruma endpoint `GET /rooms/{roomId}/relations/{eventId}/m.annotation`.
///
/// Returns `Ok(false)` when the client has no logged-in user.
async fn has_own_reaction(
    client: &Client,
    room_id: &RoomId,
    event_id: &EventId,
    keys: &[&str],
) -> anyhow::Result<bool> {
    let Some(user_id) = client.user_id() else {
        return Ok(false);
    };

    let request = get_relating_events_with_rel_type::v1::Request::new(
        room_id.to_owned(),
        event_id.to_owned(),
        RelationType::Annotation,
    );
    let response = client.send(request).await?;

    for raw in response.chunk {
        let Ok(AnyMessageLikeEvent::Reaction(reaction)) = raw.deserialize() else {
            continue;
        };
        if reaction.sender() != user_id {
            continue;
        }
        let Some(original) = reaction.as_original() else {
            continue;
        };
        if keys.contains(&original.content.relates_to.key.as_str()) {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn on_stripped_state_member(
    room_member: StrippedRoomMemberEvent,
    client: Client,
    room: Room,
) {
    let Some(user_id) = client.user_id() else {
        tracing::error!("Could not get user id from client");
        return;
    };
    if room_member.state_key != user_id {
        return;
    }

    tokio::spawn(async move {
        // Join first — request_encryption_state() before joining tends
        // to fail because the server won't serve state to non-members.
        let mut delay = 2u64;

        while let Err(err) = room.join().await {
            tracing::error!(
                "Failed to join room {} ({err:?}), retrying in {delay}s",
                room.room_id()
            );
            sleep(std::time::Duration::from_secs(delay)).await;
            if let Some(x) = delay.checked_mul(2) {
                delay = x;
            } else {
                tracing::error!("Delay got too large, aborting");
                return;
            }
            if delay > 3600 {
                tracing::error!("Can't join room {} ({err:?})", room.room_id());
                return;
            }
        }
        tracing::info!("Successfully joined room {}", room.room_id());

        // Now check whether the room is encrypted; leave if it is.
        if let Err(e) = room.request_encryption_state().await {
            tracing::warn!(
                "Could not request encryption state for room {} ({e}), \
                 staying joined anyway",
                room.room_id()
            );
        } else if !matches!(
            room.encryption_state(),
            matrix_sdk::EncryptionState::NotEncrypted
        ) {
            tracing::error!("Room {} is encrypted, leaving", room.room_id());
            if let Err(e) = room.leave().await {
                tracing::error!("Failed to leave room {}: {e}", room.room_id());
            }
        }
    });
}

async fn on_room_message(
    event: OriginalSyncRoomMessageEvent,
    room: Room,
    client: Client,
    tx: GpxSenderCtx,
) {
    if room.state() != RoomState::Joined {
        return;
    }
    if let Some(user_id) = client.user_id()
        && event.sender == user_id
    {
        return;
    }

    let MessageType::File(file_content) = event.content.msgtype else {
        return;
    };

    let filename = file_content.filename().to_owned();
    if !std::path::Path::new(&filename)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gpx"))
    {
        return;
    }

    // Skip messages the bot already handled (any own ✅/❌ reaction),
    // before paying for the media download. On an error, proceed anyway:
    // the server-side dedup makes a re-upload a no-op.
    if matches!(
        has_own_reaction(&client, room.room_id(), &event.event_id, &["✅", "❌"]).await,
        Ok(true)
    ) {
        tracing::info!(
            "Skipping {filename}: bot already reacted to {}",
            event.event_id
        );
        return;
    }

    let bytes = match client
        .media()
        .get_media_content(
            &MediaRequestParameters {
                source: file_content.source,
                format: MediaFormat::File,
            },
            false,
        )
        .await
    {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to download file {filename}: {e}");
            return;
        }
    };

    tracing::info!("Received GPX file: {filename}");
    let _ = tx
        .lock()
        .await
        .send((
            bytes,
            GpxFileMetadata {
                filename,
                room_id: room.room_id().to_owned(),
                event_id: event.event_id.clone(),
            },
        ))
        .await;
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::{persist_next_batch, persist_sync_token, read_sync_token};
    use matrix_sdk::{LoopCtrl, sync::SyncResponse};

    #[test]
    fn read_sync_token_returns_none_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sync-token.txt");
        assert_eq!(read_sync_token(&path), None);
    }

    #[test]
    fn persist_sync_token_creates_file_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tokens").join("sync-token.txt");
        persist_sync_token(&path, "s7259_1").unwrap();

        assert_eq!(path.metadata().unwrap().len(), "s7259_1".len() as u64);
        assert_eq!(read_sync_token(&path).as_deref(), Some("s7259_1"));
    }

    #[test]
    fn read_sync_token_trims_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sync-token.txt");
        std::fs::write(&path, "s7259_1\n").unwrap();
        assert_eq!(read_sync_token(&path).as_deref(), Some("s7259_1"));
    }

    #[test]
    fn read_sync_token_returns_none_for_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sync-token.txt");
        std::fs::write(&path, "\n").unwrap();
        assert_eq!(read_sync_token(&path), None);
    }

    #[tokio::test]
    async fn persist_next_batch_writes_token_and_continues() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sync-token.txt");
        let response = SyncResponse {
            next_batch: "s7259_2".to_owned(),
            ..SyncResponse::default()
        };

        let ctrl = persist_next_batch(&path, Ok(response)).await.unwrap();
        assert_eq!(ctrl, LoopCtrl::Continue);
        assert_eq!(read_sync_token(&path).as_deref(), Some("s7259_2"));
    }
}
