//! The concrete repository implementation against Postgres via SQLx.
//!
//! All eight traits in [`crate::traits`] are implemented on a single
//! [`SqlxRepository`] struct that owns a [`sqlx::PgPool`]. Queries use
//! the **runtime** `sqlx::query_*` forms (not the `query!` macro), so the
//! crate compiles without a live `DATABASE_URL` and without an offline
//! `.sqlx/` cache. The cache is produced in Phase 6 item 5.38 via
//! `cargo sqlx prepare --workspace`.
//!
//! Row mapping goes through db-local structs deriving
//! [`sqlx::FromRow`] — the orphan rule forbids implementing
//! `FromRow` on `health_core` types here. Each row then converts into
//! the corresponding `health_core` type via a `From` impl.

use std::time::Duration;

use health_core::{
    ApiToken, CoreSession, ExerciseKind, ExerciseSession, HeartrateSample, NewApiToken,
    NewExerciseSession, NewHeartrateSamples, NewOidcState, OidcState, RunningSession, User,
    WeightSession,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::postgres::types::PgInterval;
use uuid::Uuid;

use crate::error::DbError;
use crate::traits::{
    ApiTokenRepository, CoreRepository, HeartrateRepository, OidcStateRepository,
    RunningRepository, SessionsRepository, UsersRepository, WeightRepository,
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
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Borrow the underlying pool — useful for callers that need to
    /// run raw queries (e.g. the migration runner at startup).
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

// ===========================================================================
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
            created_at: r.created_at,
        })
    }
}

#[derive(sqlx::FromRow)]
struct WeightRow {
    session_id: Uuid,
    exercise_name: String,
    weight_kg: f64,
    sets: i32,
    reps: i32,
    quality: Option<i32>,
}

impl From<WeightRow> for WeightSession {
    fn from(r: WeightRow) -> Self {
        Self {
            session_id: r.session_id,
            exercise_name: r.exercise_name,
            weight_kg: r.weight_kg,
            sets: r.sets,
            reps: r.reps,
            quality: r.quality,
        }
    }
}

#[derive(sqlx::FromRow)]
struct CoreRow {
    session_id: Uuid,
    exercise_name: String,
    duration: PgInterval,
    quality: Option<i32>,
}

impl TryFrom<CoreRow> for CoreSession {
    type Error = DbError;
    fn try_from(r: CoreRow) -> Result<Self, Self::Error> {
        Ok(Self {
            session_id: r.session_id,
            exercise_name: r.exercise_name,
            duration: interval_to_std(r.duration)?,
            quality: r.quality,
        })
    }
}

#[derive(sqlx::FromRow)]
struct RunningRow {
    session_id: Uuid,
    distance_m: f64,
    gpx_data: Option<Vec<u8>>,
}

impl From<RunningRow> for RunningSession {
    fn from(r: RunningRow) -> Self {
        Self {
            session_id: r.session_id,
            distance_m: r.distance_m,
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

// ===========================================================================
// Helpers
// ===========================================================================

/// Convert a `std::time::Duration` into the `PgInterval` SQLx expects
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

/// Map a SQLx error to a [`DbError`], recognising unique/FK/CHECK
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
    ) -> Result<Vec<ExerciseSession>, DbError> {
        // Build the query dynamically. Four optional predicates; we
        // branch on the four combos to keep the bind order correct
        // rather than emit a string-frog of optional clauses.
        let rows: Vec<SessionRow> = match (kind, from, to) {
            (None, None, None) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(k), None, None) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND kind = $2 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(k.as_str())
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(f), None) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND started_at >= $2 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(f)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None, Some(t)) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND started_at <= $2 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(t)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(k), Some(f), None) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND kind = $2 AND started_at >= $3 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(k.as_str())
                .bind(f)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(k), None, Some(t)) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND kind = $2 AND started_at <= $3 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(k.as_str())
                .bind(t)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(f), Some(t)) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions WHERE user_id = $1 AND started_at BETWEEN $2 AND $3 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(f)
                .bind(t)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(k), Some(f), Some(t)) => {
                sqlx::query_as::<_, SessionRow>(
                    "SELECT id, user_id, kind, started_at, duration, notes, created_at \
                 FROM exercise_sessions \
                 WHERE user_id = $1 AND kind = $2 AND started_at BETWEEN $3 AND $4 \
                 ORDER BY started_at DESC",
                )
                .bind(user_id)
                .bind(k.as_str())
                .bind(f)
                .bind(t)
                .fetch_all(&self.pool)
                .await?
            }
        };
        rows.into_iter().map(TryFrom::try_from).collect()
    }

    async fn get(&self, id: Uuid) -> Result<ExerciseSession, DbError> {
        let row: SessionRow = sqlx::query_as::<_, SessionRow>(
            "SELECT id, user_id, kind, started_at, duration, notes, created_at \
             FROM exercise_sessions WHERE id = $1",
        )
        .bind(id)
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
        let row: SessionRow = sqlx::query_as::<_, SessionRow>(
            "INSERT INTO exercise_sessions (user_id, kind, started_at, duration, notes) \
             VALUES ($1, $2, $3, $4, $5) \
             RETURNING id, user_id, kind, started_at, duration, notes, created_at",
        )
        .bind(user_id)
        .bind(new.kind.as_str())
        .bind(new.started_at)
        .bind(std_to_interval(new.duration))
        .bind(new.notes.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        row.try_into()
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DbError> {
        let res = sqlx::query("DELETE FROM exercise_sessions WHERE id = $1")
            .bind(id)
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
        sqlx::query(
            "INSERT INTO weight_exercises \
             (session_id, exercise_name, weight_kg, sets, reps, quality) \
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(session_id)
        .bind(&row.exercise_name)
        .bind(row.weight_kg)
        .bind(row.sets)
        .bind(row.reps)
        .bind(row.quality)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<WeightSession, DbError> {
        let row: WeightRow = sqlx::query_as::<_, WeightRow>(
            "SELECT session_id, exercise_name, weight_kg, sets, reps, quality \
             FROM weight_exercises WHERE session_id = $1",
        )
        .bind(session_id)
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
    async fn insert(&self, session_id: Uuid, row: &CoreSession) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        enforce_kind(&mut tx, session_id, ExerciseKind::Core).await?;
        sqlx::query(
            "INSERT INTO core_exercises \
             (session_id, exercise_name, duration, quality) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(&row.exercise_name)
        .bind(std_to_interval(row.duration))
        .bind(row.quality)
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<CoreSession, DbError> {
        let row: CoreRow = sqlx::query_as::<_, CoreRow>(
            "SELECT session_id, exercise_name, duration, quality \
             FROM core_exercises WHERE session_id = $1",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        row.try_into()
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
        sqlx::query(
            "INSERT INTO running_sessions (session_id, distance_m, gpx_data) \
             VALUES ($1, $2, $3)",
        )
        .bind(session_id)
        .bind(row.distance_m)
        .bind(row.gpx_data.as_deref())
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await?;
        Ok(())
    }

    async fn get_by_session(&self, session_id: Uuid) -> Result<RunningSession, DbError> {
        let row: RunningRow = sqlx::query_as::<_, RunningRow>(
            "SELECT session_id, distance_m, NULL::bytea AS gpx_data \
             FROM running_sessions WHERE session_id = $1",
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }

    async fn get_gpx(&self, session_id: Uuid) -> Result<Option<Vec<u8>>, DbError> {
        let row: Option<(Option<Vec<u8>>,)> =
            sqlx::query_as("SELECT gpx_data FROM running_sessions WHERE session_id = $1")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            None => Err(DbError::NotFound),
            Some((gpx,)) => Ok(gpx),
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
            let res = sqlx::query(
                "INSERT INTO heartrate_samples (session_id, offset_secs, bpm) \
                 VALUES ($1, $2, $3) \
                 ON CONFLICT (session_id, offset_secs) DO NOTHING",
            )
            .bind(s.session_id)
            .bind(s.offset_secs)
            .bind(s.bpm)
            .execute(&mut *tx)
            .await?;
            inserted += res.rows_affected();
        }
        tx.commit().await?;
        Ok(inserted)
    }

    async fn list_for_session(&self, session_id: Uuid) -> Result<Vec<HeartrateSample>, DbError> {
        let rows: Vec<HeartrateRow> = sqlx::query_as::<_, HeartrateRow>(
            "SELECT session_id, offset_secs, bpm FROM heartrate_samples \
             WHERE session_id = $1 ORDER BY offset_secs ASC",
        )
        .bind(session_id)
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
        let row: UserRow = sqlx::query_as::<_, UserRow>(
            "INSERT INTO users (external_id, display_name) VALUES ($1, $2) \
             ON CONFLICT (external_id) DO UPDATE \
                 SET display_name = EXCLUDED.display_name \
             RETURNING id, external_id, display_name, created_at",
        )
        .bind(external_id)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.into())
    }

    async fn get(&self, id: Uuid) -> Result<User, DbError> {
        let row: UserRow = sqlx::query_as::<_, UserRow>(
            "SELECT id, external_id, display_name, created_at FROM users WHERE id = $1",
        )
        .bind(id)
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
        sqlx::query(
            "INSERT INTO oidc_state (csrf, code_verifier, nonce, resume_token) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&state.csrf)
        .bind(&state.code_verifier)
        .bind(&state.nonce)
        .bind(state.resume_token.as_deref())
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn fetch(&self, csrf: &str) -> Result<OidcState, DbError> {
        let row: OidcStateRow = sqlx::query_as::<_, OidcStateRow>(
            "SELECT csrf, code_verifier, nonce, resume_token, created_at \
             FROM oidc_state WHERE csrf = $1",
        )
        .bind(csrf)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DbError::NotFound,
            other => map_err(other),
        })?;
        Ok(row.into())
    }

    async fn delete(&self, csrf: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM oidc_state WHERE csrf = $1")
            .bind(csrf)
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
        // 32 random bytes -> hex-encoded -> 64-char cleartext token.
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let cleartext = hex::encode(bytes);
        let hash = sha256_hex(&cleartext);

        let row: ApiTokenRow = sqlx::query_as::<_, ApiTokenRow>(
            "INSERT INTO api_tokens (user_id, label, token_hash) \
             VALUES ($1, $2, $3) \
             RETURNING id, user_id, label, token_hash, created_at, last_used_at",
        )
        .bind(user_id)
        .bind(label)
        .bind(&hash)
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
        let row: Option<(Uuid,)> =
            sqlx::query_as("SELECT user_id FROM api_tokens WHERE token_hash = $1")
                .bind(&hash)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            None => Ok(None),
            Some((user_id,)) => {
                sqlx::query("UPDATE api_tokens SET last_used_at = NOW() WHERE token_hash = $1")
                    .bind(&hash)
                    .execute(&self.pool)
                    .await?;
                Ok(Some(user_id))
            }
        }
    }

    async fn revoke(&self, id: Uuid) -> Result<bool, DbError> {
        let res = sqlx::query("DELETE FROM api_tokens WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_for_user(&self, user_id: Uuid) -> Result<Vec<ApiToken>, DbError> {
        let rows: Vec<ApiTokenRow> = sqlx::query_as::<_, ApiTokenRow>(
            "SELECT id, user_id, label, token_hash, created_at, last_used_at \
             FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ===========================================================================
// Shared: enforce_kind — guards the CTI child inserts inside the tx,
// since Postgres CHECK cannot reference other tables.
// ===========================================================================

async fn enforce_kind(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session_id: Uuid,
    expected: ExerciseKind,
) -> Result<(), DbError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT kind FROM exercise_sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(&mut **tx)
        .await?;
    match row {
        None => Err(DbError::NotFound),
        Some((kind,)) if kind == expected.as_str() => Ok(()),
        Some((kind,)) => Err(DbError::KindMismatch {
            parent: kind,
            child: expected.as_str().to_owned(),
        }),
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}
