//! The concrete repository implementation against Postgres via `SQLx`.
//!
//! All eight traits in [`crate::traits`] are implemented on a single
//! [`SqlxRepository`] struct that owns a [`sqlx::PgPool`]. Queries use
//! the **compile-time** `query!` / `query_as!` macros which are verified
//! against the offline cache in `sqlx-data.json` at compile time.
//!
//! Row mapping goes through db-local structs deriving
//! [`sqlx::FromRow`] — the orphan rule forbids implementing
//! `FromRow` on `health_core` types here. Each row then converts into
//! the corresponding `health_core` type via a `From` impl.

use std::time::Duration;

use health_core::{
    ApiToken, CoreSession, ExerciseKind, ExerciseSession, HeartrateSample, NewApiToken,
    NewExerciseSession, NewHeartrateSamples, NewOidcState, OidcState, PendingLink, RunningSession,
    User, WeightSession,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::postgres::types::PgInterval;
use uuid::Uuid;

use crate::error::DbError;
use crate::traits::{
    ApiTokenRepository, CoreRepository, HeartrateRepository, OidcStateRepository,
    PendingLinkRepository, RunningRepository, SessionsRepository, UsersRepository,
    WeightRepository,
};

// ===========================================================================
// SqlxRepository
// ===========================================================================

/// The Postgres-backed repository. Cheaply cloneable (`PgPool` is
/// `Arc`-backed) so handlers can hold a copy per request.
#[derive(Debug, Clone)]
pub struct SqlxRepository {
    pool: PgPool,
}

impl SqlxRepository {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool — useful for callers that need to
    /// run raw queries (e.g. the migration runner at startup).
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }

    async fn list_all(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 \
             ORDER BY started_at DESC \
             LIMIT $2 OFFSET $3",
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_by_kind(
        &self,
        user_id: Uuid,
        kind: &str,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND kind = $2 \
             ORDER BY started_at DESC \
             LIMIT $3 OFFSET $4",
            user_id,
            kind,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_from(
        &self,
        user_id: Uuid,
        from: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND started_at >= $2 \
             ORDER BY started_at DESC \
             LIMIT $3 OFFSET $4",
            user_id,
            from,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_to(
        &self,
        user_id: Uuid,
        to: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND started_at <= $2 \
             ORDER BY started_at DESC \
             LIMIT $3 OFFSET $4",
            user_id,
            to,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_by_kind_from(
        &self,
        user_id: Uuid,
        kind: &str,
        from: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND kind = $2 AND started_at >= $3 \
             ORDER BY started_at DESC \
             LIMIT $4 OFFSET $5",
            user_id,
            kind,
            from,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_by_kind_to(
        &self,
        user_id: Uuid,
        kind: &str,
        to: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND kind = $2 AND started_at <= $3 \
             ORDER BY started_at DESC \
             LIMIT $4 OFFSET $5",
            user_id,
            kind,
            to,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_between(
        &self,
        user_id: Uuid,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE user_id = $1 AND started_at BETWEEN $2 AND $3 \
             ORDER BY started_at DESC \
             LIMIT $4 OFFSET $5",
            user_id,
            from,
            to,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    async fn list_by_kind_between(
        &self,
        user_id: Uuid,
        kind: &str,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<SessionRow>, sqlx::Error> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises \
             WHERE user_id = $1 AND kind = $2 AND started_at BETWEEN $3 AND $4 \
             ORDER BY started_at DESC \
             LIMIT $5 OFFSET $6",
            user_id,
            kind,
            from,
            to,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Insert a session and its kind-specific child row atomically.
    ///
    /// Both rows go in one transaction, so a failed child insert rolls
    /// back the parent — no orphan is ever visible, and the manual
    /// delete-on-failure dance is unnecessary. The `session_id` inside
    /// `child` is ignored; the parent's server-assigned id is used.
    ///
    /// # Errors
    ///
    /// Returns [`DbError::KindMismatch`] if `child` does not match
    /// `new.kind`, and [`DbError::Sqlx`] on any database failure (the
    /// transaction is rolled back, so nothing is persisted).
    pub async fn insert_session_with_child(
        &self,
        user_id: Uuid,
        new: &NewExerciseSession,
        child: Option<SessionChild>,
    ) -> Result<ExerciseSession, DbError> {
        let mut tx = self.pool.begin().await?;
        let session = insert_session_exec(&mut *tx, user_id, new)
            .await
            .map_err(map_err)?;

        match child {
            None => {}
            Some(SessionChild::Weight(w)) => {
                enforce_kind(&mut tx, session.id, ExerciseKind::Weight).await?;
                sqlx::query!(
                    "INSERT INTO exercise_weight (session_id, weight_g, sets) \
                      VALUES ($1, $2, $3)",
                    session.id,
                    w.weight_g,
                    w.sets
                )
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
            Some(SessionChild::Core) => {
                enforce_kind(&mut tx, session.id, ExerciseKind::Core).await?;
                sqlx::query!(
                    "INSERT INTO exercise_core (session_id) VALUES ($1)",
                    session.id
                )
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
            Some(SessionChild::Running(r)) => {
                enforce_kind(&mut tx, session.id, ExerciseKind::Running).await?;
                sqlx::query!(
                    "INSERT INTO exercise_running \
                      (session_id, distance_m, moving_distance_m, moving_time, gpx_data) \
                      VALUES ($1, $2, $3, $4, $5)",
                    session.id,
                    r.distance_m,
                    r.moving_distance_m,
                    r.moving_time,
                    r.gpx_data.as_deref()
                )
                .execute(&mut *tx)
                .await
                .map_err(map_err)?;
            }
        }

        tx.commit().await?;
        session.try_into()
    }
}
// Row structs — `FromRow` derive + `From` conversion to core types.
// ===========================================================================

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    kind: String,
    started_at: chrono::DateTime<chrono::Utc>,
    duration: PgInterval,
    notes: Option<String>,
    quality: Option<i32>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<SessionRow> for ExerciseSession {
    type Error = DbError;
    fn try_from(r: SessionRow) -> Result<Self, Self::Error> {
        let kind: ExerciseKind = r
            .kind
            .parse()
            .map_err(|_| DbError::Invalid(format!("unknown kind in DB: {}", r.kind)))?;
        Ok(Self {
            id: r.id,
            user_id: r.user_id,
            kind,
            started_at: r.started_at,
            duration: interval_to_std(r.duration)?,
            notes: r.notes,
            quality: r.quality,
            created_at: r.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct WeightRow {
    session_id: Uuid,
    weight_g: i32,
    sets: i32,
}

impl From<WeightRow> for WeightSession {
    fn from(r: WeightRow) -> Self {
        Self {
            session_id: r.session_id,
            weight_g: r.weight_g,
            sets: r.sets,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CoreRow {
    session_id: Uuid,
}

impl From<CoreRow> for CoreSession {
    fn from(r: CoreRow) -> Self {
        Self {
            session_id: r.session_id,
        }
    }
}

#[derive(sqlx::FromRow)]
struct RunningRow {
    session_id: Uuid,
    distance_m: i32,
    moving_distance_m: Option<i32>,
    moving_time: Option<f64>,
    gpx_data: Option<Vec<u8>>,
}

impl From<RunningRow> for RunningSession {
    fn from(r: RunningRow) -> Self {
        Self {
            session_id: r.session_id,
            distance_m: r.distance_m,
            moving_distance_m: r.moving_distance_m,
            moving_time: r.moving_time,
            gpx_data: r.gpx_data,
        }
    }
}

#[derive(sqlx::FromRow)]
struct HeartrateRow {
    session_id: Uuid,
    offset_secs: i32,
    bpm: i16,
}

impl From<HeartrateRow> for HeartrateSample {
    fn from(r: HeartrateRow) -> Self {
        Self {
            session_id: r.session_id,
            offset_secs: r.offset_secs,
            bpm: r.bpm,
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    external_id: String,
    display_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        Self {
            id: r.id,
            external_id: r.external_id,
            display_name: r.display_name,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct OidcStateRow {
    csrf: String,
    code_verifier: String,
    nonce: String,
    resume_token: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<OidcStateRow> for OidcState {
    fn from(r: OidcStateRow) -> Self {
        Self {
            csrf: r.csrf,
            nonce: r.nonce,
            code_verifier: r.code_verifier,
            resume_token: r.resume_token,
            created_at: r.created_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ApiTokenRow {
    id: Uuid,
    user_id: Uuid,
    label: String,
    token_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
    last_used_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ApiTokenRow> for ApiToken {
    fn from(r: ApiTokenRow) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            label: r.label,
            token_hash: r.token_hash,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
        }
    }
}

#[derive(sqlx::FromRow)]
struct PendingLinkRow {
    code: String,
    user_id: Option<Uuid>,
    expires_at: chrono::DateTime<chrono::Utc>,
    accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    token_returned_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<PendingLinkRow> for PendingLink {
    fn from(r: PendingLinkRow) -> Self {
        Self {
            code: r.code,
            user_id: r.user_id,
            expires_at: r.expires_at,
            accepted_at: r.accepted_at,
            token_returned_at: r.token_returned_at,
            created_at: r.created_at,
        }
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Convert a `std::time::Duration` into the `PgInterval` `SQLx` expects
/// for an `INTERVAL` bind parameter.
fn std_to_interval(d: Duration) -> PgInterval {
    PgInterval {
        months: 0,
        days: 0,
        microseconds: d.as_micros().try_into().unwrap_or(i64::MAX),
    }
}

/// Convert a `PgInterval` back into `std::time::Duration`. Only the
/// `microseconds` field is used (we never store months/days — a workout
/// duration never spans calendar units).
fn interval_to_std(i: PgInterval) -> Result<Duration, DbError> {
    if i.months != 0 || i.days != 0 {
        return Err(DbError::Invalid(format!(
            "interval has non-zero months/days ({i:?}) — cannot represent as std::time::Duration"
        )));
    }
    let micros: u128 = i.microseconds.try_into().map_err(|_| {
        DbError::Invalid(format!(
            "interval microseconds negative: {}",
            i.microseconds
        ))
    })?;
    Ok(Duration::from_micros(micros.try_into().unwrap_or(u64::MAX)))
}

/// Map a `SQLx` error to a [`DbError`], recognising unique/FK/CHECK
/// violations so callers can branch on them.
fn map_err(e: sqlx::Error) -> DbError {
    if let Some(db) = e.as_database_error() {
        if db.is_unique_violation() {
            return DbError::Conflict(db.to_string());
        }
        if db.is_foreign_key_violation() {
            return DbError::Invalid(format!("foreign key violation: {db}"));
        }
        if db.is_check_violation() {
            return DbError::Invalid(format!("check violation: {db}"));
        }
    }
    DbError::Sqlx(e)
}

// ===========================================================================
// SessionsRepository
// ===========================================================================

#[async_trait::async_trait]
impl SessionsRepository for SqlxRepository {
    async fn list(
        &self,
        user_id: Uuid,
        kind: Option<ExerciseKind>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<ExerciseSession>, DbError> {
        let rows: Vec<SessionRow> = match (kind, from, to) {
            (None, None, None) => self.list_all(user_id, limit, offset).await?,
            (Some(k), None, None) => {
                self.list_by_kind(user_id, k.as_str(), limit, offset)
                    .await?
            }
            (None, Some(f), None) => self.list_from(user_id, f, limit, offset).await?,
            (None, None, Some(t)) => self.list_to(user_id, t, limit, offset).await?,
            (Some(k), Some(f), None) => {
                self.list_by_kind_from(user_id, k.as_str(), f, limit, offset)
                    .await?
            }
            (Some(k), None, Some(t)) => {
                self.list_by_kind_to(user_id, k.as_str(), t, limit, offset)
                    .await?
            }
            (None, Some(f), Some(t)) => self.list_between(user_id, f, t, limit, offset).await?,
            (Some(k), Some(f), Some(t)) => {
                self.list_by_kind_between(user_id, k.as_str(), f, t, limit, offset)
                    .await?
            }
        };
        rows.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get(&self, id: Uuid) -> Result<ExerciseSession, DbError> {
        let row = sqlx::query_as!(
            SessionRow,
            "SELECT id, user_id, kind, started_at, duration, notes, quality, created_at \
             FROM exercises WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        row.try_into()
    }

    async fn insert(
        &self,
        user_id: Uuid,
        new: &NewExerciseSession,
    ) -> Result<ExerciseSession, DbError> {
        let row = insert_session_exec(&self.pool, user_id, new)
            .await
            .map_err(map_err)?;
        row.try_into()
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let res = sqlx::query!("DELETE FROM exercises WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

// ===========================================================================
// WeightRepository
// ===========================================================================

#[async_trait::async_trait]
impl WeightRepository for SqlxRepository {
    async fn insert(&self, session_id: Uuid, row: &WeightSession) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        enforce_kind(&mut tx, session_id, ExerciseKind::Weight).await?;
        sqlx::query!(
            "INSERT INTO exercise_weight \
              (session_id, weight_g, sets) \
              VALUES ($1, $2, $3)",
            session_id,
            row.weight_g,
            row.sets
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<WeightSession, DbError> {
        let row = sqlx::query_as!(
            WeightRow,
            "SELECT session_id, weight_g, sets \
              FROM exercise_weight WHERE session_id = $1",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }
}

// ===========================================================================
// CoreRepository
// ===========================================================================

#[async_trait::async_trait]
impl CoreRepository for SqlxRepository {
    async fn insert(&self, session_id: Uuid, _row: &CoreSession) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        enforce_kind(&mut tx, session_id, ExerciseKind::Core).await?;
        sqlx::query!(
            "INSERT INTO exercise_core \
              (session_id) \
              VALUES ($1)",
            session_id
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<CoreSession, DbError> {
        let row = sqlx::query_as!(
            CoreRow,
            "SELECT session_id \
              FROM exercise_core WHERE session_id = $1",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }
}

// ===========================================================================
// RunningRepository
// ===========================================================================

#[async_trait::async_trait]
impl RunningRepository for SqlxRepository {
    async fn insert(&self, session_id: Uuid, row: &RunningSession) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        enforce_kind(&mut tx, session_id, ExerciseKind::Running).await?;
        sqlx::query!(
            "INSERT INTO exercise_running (session_id, distance_m, moving_distance_m, moving_time, gpx_data) \
              VALUES ($1, $2, $3, $4, $5)",
            session_id,
            row.distance_m,
            row.moving_distance_m,
            row.moving_time,
            row.gpx_data.as_deref()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<RunningSession, DbError> {
        let row = sqlx::query_as!(
            RunningRow,
            "SELECT session_id, distance_m, moving_distance_m, moving_time, NULL::bytea AS gpx_data \
              FROM exercise_running WHERE session_id = $1",
            session_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }

    async fn get_gpx(&self, session_id: Uuid) -> Result<Option<Vec<u8>>, DbError> {
        let row = sqlx::query!(
            "SELECT gpx_data FROM exercise_running WHERE session_id = $1",
            session_id
        )
        .fetch_optional(&self.pool)
        .await?;
        match row {
            None => Err(DbError::NotFound),
            Some(r) => Ok(r.gpx_data),
        }
    }
}

// ===========================================================================
// HeartrateRepository
// ===========================================================================

#[async_trait::async_trait]
impl HeartrateRepository for SqlxRepository {
    async fn insert_bulk(&self, samples: &NewHeartrateSamples) -> Result<u64, DbError> {
        if samples.samples.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;
        let mut inserted: u64 = 0;
        for s in &samples.samples {
            let res = sqlx::query!(
                "INSERT INTO heartrate_samples (session_id, offset_secs, bpm) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (session_id, offset_secs) DO NOTHING",
                s.session_id,
                s.offset_secs,
                s.bpm
            )
            .execute(&mut *tx)
            .await?;
            inserted += res.rows_affected();
        }
        tx.commit().await?;
        Ok(inserted)
    }

    async fn list_for_session(&self, session_id: Uuid) -> Result<Vec<HeartrateSample>, DbError> {
        let rows = sqlx::query_as!(
            HeartrateRow,
            "SELECT session_id, offset_secs, bpm FROM heartrate_samples \
             WHERE session_id = $1 ORDER BY offset_secs ASC",
            session_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ===========================================================================
// UsersRepository
// ===========================================================================

#[async_trait::async_trait]
impl UsersRepository for SqlxRepository {
    async fn upsert_by_external_id(
        &self,
        external_id: &str,
        display_name: Option<&str>,
    ) -> Result<User, DbError> {
        let row = sqlx::query_as!(
            UserRow,
            "INSERT INTO users (external_id, display_name) VALUES ($1, $2) \
             ON CONFLICT (external_id) DO UPDATE \
                 SET display_name = EXCLUDED.display_name \
             RETURNING id, external_id, display_name, created_at",
            external_id,
            display_name
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.into())
    }

    async fn get(&self, id: Uuid) -> Result<User, DbError> {
        let row = sqlx::query_as!(
            UserRow,
            "SELECT id, external_id, display_name, created_at FROM users WHERE id = $1",
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }
}

// ===========================================================================
// OidcStateRepository
// ===========================================================================

#[async_trait::async_trait]
impl OidcStateRepository for SqlxRepository {
    async fn insert(&self, state: &NewOidcState) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO oidc_state (csrf, code_verifier, nonce, resume_token) \
             VALUES ($1, $2, $3, $4)",
            state.csrf,
            state.code_verifier,
            state.nonce,
            state.resume_token.as_deref()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn fetch(&self, csrf: &str) -> Result<OidcState, DbError> {
        let row = sqlx::query_as!(
            OidcStateRow,
            "SELECT csrf, code_verifier, nonce, resume_token, created_at \
             FROM oidc_state WHERE csrf = $1",
            csrf
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }

    async fn delete(&self, csrf: &str) -> Result<(), DbError> {
        sqlx::query!("DELETE FROM oidc_state WHERE csrf = $1", csrf)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ===========================================================================
// ApiTokenRepository
// ===========================================================================

#[async_trait::async_trait]
impl ApiTokenRepository for SqlxRepository {
    async fn issue(&self, user_id: Uuid, label: &str) -> Result<NewApiToken, DbError> {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let cleartext = hex::encode(bytes);
        let hash = sha256_hex(&cleartext);

        let row = sqlx::query_as!(
            ApiTokenRow,
            "INSERT INTO api_tokens (user_id, label, token_hash) \
             VALUES ($1, $2, $3) \
             RETURNING id, user_id, label, token_hash, created_at, last_used_at",
            user_id,
            label,
            &hash
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;

        Ok(NewApiToken {
            id: row.id,
            user_id: row.user_id,
            label: row.label,
            token: cleartext,
        })
    }

    async fn verify(&self, cleartext: &str) -> Result<Option<Uuid>, DbError> {
        let hash = sha256_hex(cleartext);
        let row = sqlx::query!(
            "SELECT user_id FROM api_tokens WHERE token_hash = $1",
            &hash
        )
        .fetch_optional(&self.pool)
        .await?;
        match row {
            None => Ok(None),
            Some(r) => {
                sqlx::query!(
                    "UPDATE api_tokens SET last_used_at = NOW() WHERE token_hash = $1",
                    &hash
                )
                .execute(&self.pool)
                .await?;
                Ok(Some(r.user_id))
            }
        }
    }

    async fn revoke(&self, id: Uuid) -> Result<bool, DbError> {
        let res = sqlx::query!("DELETE FROM api_tokens WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiToken>, DbError> {
        let rows = sqlx::query_as!(
            ApiTokenRow,
            "SELECT id, user_id, label, token_hash, created_at, last_used_at \
             FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ===========================================================================
// PendingLinkRepository
// ===========================================================================

#[async_trait::async_trait]
impl PendingLinkRepository for SqlxRepository {
    async fn create(
        &self,
        code: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), DbError> {
        sqlx::query!(
            "INSERT INTO pending_links (code, expires_at) VALUES ($1, $2)",
            code,
            expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn confirm(&self, code: &str, user_id: Uuid) -> Result<(), DbError> {
        let link = PendingLinkRepository::fetch(self, code).await?;
        if link.accepted_at.is_some() {
            return Err(DbError::Conflict("link already accepted".to_owned()));
        }
        if link.expires_at < chrono::Utc::now() {
            return Err(DbError::Conflict("link expired".to_owned()));
        }
        sqlx::query!(
            "UPDATE pending_links SET user_id = $2, accepted_at = NOW() WHERE code = $1",
            code,
            user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch(&self, code: &str) -> Result<PendingLink, DbError> {
        let row = sqlx::query_as!(
            PendingLinkRow,
            "SELECT code, user_id, expires_at, accepted_at, token_returned_at, created_at \
             FROM pending_links WHERE code = $1",
            code
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }

    async fn mark_token_returned(&self, code: &str) -> Result<bool, DbError> {
        let res = sqlx::query!(
            "UPDATE pending_links SET token_returned_at = NOW() \
             WHERE code = $1 AND token_returned_at IS NULL",
            code
        )
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected() > 0)
    }
}

// ===========================================================================
// Shared: enforce_kind — guards the CTI child inserts inside the tx,
// since Postgres CHECK cannot reference other tables.
// ===========================================================================

/// Kind-specific child row inserted atomically with its parent.
///
/// The parent's server-assigned id is used regardless of any
/// `session_id` set on the carried struct.
#[derive(Debug)]
pub enum SessionChild {
    Weight(WeightSession),
    Core,
    Running(RunningSession),
}

/// Insert the parent `exercises` row via `exec` (either the pool or a
/// transaction), returning the fully-populated row. Shared by
/// [`SessionsRepository::insert`] and
/// [`SqlxRepository::insert_session_with_child`].
async fn insert_session_exec(
    exec: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    user_id: Uuid,
    new: &NewExerciseSession,
) -> Result<SessionRow, sqlx::Error> {
    sqlx::query_as!(
        SessionRow,
        "INSERT INTO exercises (user_id, kind, started_at, duration, notes, quality) \
          VALUES ($1, $2, $3, $4, $5, $6) \
          RETURNING id, user_id, kind, started_at, duration, notes, quality, created_at",
        user_id,
        new.kind.as_str(),
        new.started_at,
        std_to_interval(new.duration),
        new.notes.as_deref(),
        new.quality
    )
    .fetch_one(exec)
    .await
}

async fn enforce_kind(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session_id: Uuid,
    expected: ExerciseKind,
) -> Result<(), DbError> {
    let row = sqlx::query!("SELECT kind FROM exercises WHERE id = $1", session_id)
        .fetch_optional(&mut **tx)
        .await?;
    match row {
        None => Err(DbError::NotFound),
        Some(r) if r.kind == expected.as_str() => Ok(()),
        Some(r) => Err(DbError::KindMismatch {
            parent: r.kind,
            child: expected.as_str().to_owned(),
        }),
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
