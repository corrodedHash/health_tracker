//! `MatrixClient` trait — the mock boundary between the bot's sync loop
//! and the matrix-sdk (item 5.25).
//!
//! The real impl wraps `matrix-sdk` and yields GPX file bytes received in
//! any joined room. The test impl returns fixture bytes so the sync loop
//! can be exercised without a live Matrix connection.

use std::path::Path;

use async_trait::async_trait;
use matrix_sdk::ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent};
use matrix_sdk::{
    Client, Room, RoomState,
    config::SyncSettings,
    media::{MediaFormat, MediaRequestParameters},
    ruma::events::room::member::StrippedRoomMemberEvent,
};
use tokio::sync::mpsc;
use tokio::time::sleep;

use crate::matrix_auth::{MatrixLoginConfig, get_client};

#[derive(Debug, Clone)]
pub struct GpxFileMetadata {
    pub filename: String,
}

type GpxSender = mpsc::Sender<(Vec<u8>, GpxFileMetadata)>;
type GpxSenderCtx = matrix_sdk::event_handler::Ctx<std::sync::Arc<tokio::sync::Mutex<GpxSender>>>;

#[async_trait]
pub trait MatrixClient: Send {
    async fn wait_for_gpx_file(&mut self) -> anyhow::Result<(Vec<u8>, GpxFileMetadata)>;
}

pub struct MatrixSdkClient {
    _client: Client,
    rx: mpsc::Receiver<(Vec<u8>, GpxFileMetadata)>,
}

impl MatrixSdkClient {
    /// Build a Matrix client that yields GPX files received in joined rooms.
    ///
    /// # Errors
    /// Returns an error if the Matrix client cannot be built, the login
    /// fails, or the initial sync does not complete.
    pub async fn new(login: &MatrixLoginConfig, session_file: &Path) -> anyhow::Result<Self> {
        let client = get_client(login, session_file).await?;

        client.add_event_handler(on_stripped_state_member);

        let (tx, rx) = mpsc::channel::<(Vec<u8>, GpxFileMetadata)>(16);
        let tx = std::sync::Arc::new(tokio::sync::Mutex::new(tx));
        client.add_event_handler_context(tx);
        client.add_event_handler(on_room_message);

        let sync_token = client.sync_once(SyncSettings::default()).await?.next_batch;
        tracing::info!("Finished initial Matrix sync");

        let sync_client = client.clone();
        tokio::spawn(async move {
            let settings = SyncSettings::default().token(sync_token);
            if let Err(e) = sync_client.sync(settings).await {
                tracing::error!("Matrix sync ended with error: {e}");
            }
        });

        Ok(Self { _client: client, rx })
    }
}

#[async_trait]
impl MatrixClient for MatrixSdkClient {
    async fn wait_for_gpx_file(&mut self) -> anyhow::Result<(Vec<u8>, GpxFileMetadata)> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("matrix event channel closed"))
    }
}

async fn on_stripped_state_member(room_member: StrippedRoomMemberEvent, client: Client, room: Room) {
    let Some(user_id) = client.user_id() else {
        tracing::error!("Could not get user id from client");
        return;
    };
    if room_member.state_key != user_id {
        return;
    }

    tokio::spawn(async move {
        if room.request_encryption_state().await.is_err() {
            tracing::error!("Could not request encryption state");
            return;
        }
        if !matches!(
            room.encryption_state(),
            matrix_sdk::EncryptionState::NotEncrypted
        ) {
            tracing::error!("Encrypted room {}, not joining", room.room_id());
            return;
        }

        tracing::info!("Autojoining room {}", room.room_id());
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
                break;
            }
            if delay > 3600 {
                tracing::error!("Can't join room {} ({err:?})", room.room_id());
                break;
            }
        }
        tracing::info!("Successfully joined room {}", room.room_id());
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
        .send((bytes, GpxFileMetadata { filename }))
        .await;
}