//! Repository traits — the mock boundary used by `web` / `bot`.
//!
//! Each trait is `async`-fn-shaped via `#[async_trait]` so test doubles
//! (mockall or hand-written) and the real [`crate::repo::SqlxRepository`]
//! share one signature surface. See `DESIGN.md` §"Testability".
//!
//! The single concrete impl lives in [`crate::repo`] and targets
//! Postgres (`sqlx::PgPool`). A SQLite in-memory impl is deferred to
//! Phase 1 item 5.10 (see `MIGRATION.md`'s "SQLite test strategy").

use health_core::{
    ApiToken, CoreSession, ExerciseKind, ExerciseSession, HeartrateSample,
    NewApiToken, NewExerciseSession, NewOidcState, NewHeartrateSamples,
    OidcState, RunningSession, User, WeightSession,
};
use uuid::Uuid;

use crate::error::DbError;

/// Page over [`ExerciseSession`] rows for a user, optionally filtered.
///
/// `kind`, `from`, `to` are all optional; passing `None` skips the
/// filter. Rows come back newest `started_at` first (mirrors the
/// `exercise_sessions_user_kind_started_at_idx` index).
#[async_trait::async_trait]
pub trait SessionsRepository: Send + Sync {
    /// List sessions for `user_id`, optionally filtered.
    async fn list(
        &self,
        user_id: Uuid,
        kind: Option<ExerciseKind>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<ExerciseSession>, DbError>;

    /// Fetch a single session by id. Returns [`DbError::NotFound`] if absent.
    async fn get(&self, id: Uuid) -> Result<ExerciseSession, DbError>;

    /// Insert a parent row, returning the fully-populated session
    /// (server-assigned `id` + `created_at`).
    async fn insert(
        &self,
        user_id: Uuid,
        new: &NewExerciseSession,
    ) -> Result<ExerciseSession, DbError>;

    /// Delete a session by id. Child rows cascade via the FK
    /// `ON DELETE CASCADE` declared in the migration.
    ///
    /// Returns `Ok(true)` if a row was deleted, `Ok(false)` if the id
    /// did not exist.
    async fn delete(&self, id: Uuid) -> Result<bool, DbError>;
}

/// `weight_exercises` rows.
#[async_trait::async_trait]
pub trait WeightRepository: Send + Sync {
    /// Attach a weight-exercise child row to session `id`. The parent
    /// row's `kind` must be `Weight`; otherwise returns
    /// [`DbError::KindMismatch`].
    async fn insert(
        &self,
        session_id: Uuid,
        row: &WeightSession,
    ) -> Result<(), DbError>;

    /// Fetch the child row for `session_id`, or [`DbError::NotFound`].
    async fn get_by_session(&self, session_id: Uuid)
        -> Result<WeightSession, DbError>;
}

/// `core_exercises` rows.
#[async_trait::async_trait]
pub trait CoreRepository: Send + Sync {
    async fn insert(
        &self,
        session_id: Uuid,
        row: &CoreSession,
    ) -> Result<(), DbError>;

    async fn get_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<CoreSession, DbError>;
}

/// `running_sessions` rows.
#[async_trait::async_trait]
pub trait RunningRepository: Send + Sync {
    /// Insert a running child row. Setting `gpx_data` is optional
    /// (some runs are uploaded without a GPX trace).
    async fn insert(
        &self,
        session_id: Uuid,
        row: &RunningSession,
    ) -> Result<(), DbError>;

    /// Child row without the `gpx_data` blob; use [`Self::get_gpx`]
    /// for the bytes.
    async fn get_by_session(
        &self,
        session_id: Uuid,
    ) -> Result<RunningSession, DbError>;

    /// Raw GPX bytes for the session (may be `None`).
    async fn get_gpx(&self, session_id: Uuid) -> Result<Option<Vec<u8>>, DbError>;
}

/// `heartrate_samples` — bulk insert + per-session scan.
#[async_trait::async_trait]
pub trait HeartrateRepository: Send + Sync {
    /// Bulk insert using `INSERT ... ON CONFLICT (session_id,
    /// offset_secs) DO NOTHING` so re-uploading a watch export is
    /// idempotent. Returns the number of rows actually inserted
    /// (i.e. newly seen offsets).
    async fn insert_bulk(
        &self,
        samples: &NewHeartrateSamples,
    ) -> Result<u64, DbError>;

    /// All samples for a session, ordered by `offset_secs`.
    async fn list_for_session(
        &self,
        session_id: Uuid,
    ) -> Result<Vec<HeartrateSample>, DbError>;
}

/// `users` — OIDC `sub` claim upsert.
#[async_trait::async_trait]
pub trait UsersRepository: Send + Sync {
    /// Insert the user if `external_id` is new, otherwise return the
    /// existing row (optionally refreshing `display_name`).
    async fn upsert_by_external_id(
        &self,
        external_id: &str,
        display_name: Option<&str>,
    ) -> Result<User, DbError>;

    async fn get(&self, id: Uuid) -> Result<User, DbError>;
}

/// `oidc_state` — port of `workout_tracker`'s insert/fetch/delete.
#[async_trait::async_trait]
pub trait OidcStateRepository: Send + Sync {
    async fn insert(&self, state: &NewOidcState) -> Result<(), DbError>;

    async fn fetch(&self, csrf: &str) -> Result<OidcState, DbError>;

    /// Remove a consumed (or stale) state row. Idempotent.
    async fn delete(&self, csrf: &str) -> Result<(), DbError>;
}

/// `api_tokens` — issue / verify / revoke for the bot bearer-token flow.
#[async_trait::async_trait]
pub trait ApiTokenRepository: Send + Sync {
    /// Generate a fresh random token, persist its SHA-256 hash, and
    /// return the clear text once.
    async fn issue(&self, user_id: Uuid, label: &str) -> Result<NewApiToken, DbError>;

    /// Look up a cleartext token by hashing it and matching
    /// `token_hash`. Updates `last_used_at` to `NOW()` on success.
    /// Returns the owning `user_id`, or `None` if no token matches.
    async fn verify(&self, cleartext: &str) -> Result<Option<Uuid>, DbError>;

    /// Delete a token by id. Idempotent; returns whether a row was removed.
    async fn revoke(&self, id: Uuid) -> Result<bool, DbError>;

    /// List a user's tokens (never includes cleartext — only hashes).
    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiToken>, DbError>;
}