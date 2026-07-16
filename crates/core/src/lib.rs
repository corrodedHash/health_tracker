//! Domain types shared by every crate in the `health_tracker` workspace.
//!
//! ## Design rules
//!
//! * No I/O — no `tokio`, no `reqwest`, no `sqlx`, no `axum`. Pure data
//!   plus validation logic. Tests use only `std` + `serde_json`.
//! * All errors are typed via `thiserror`. Callers convert as needed.
//! * Time is stored as `chrono::DateTime<Utc>` everywhere (matches the
//!   `TIMESTAMPTZ` SQL columns in the design). `time` is re-exported only
//!   because the bot needs it for `gpx` parsing.
//!
//! ## Extensibility
//!
//! Adding a new exercise type only requires:
//!   1. a new variant on [`ExerciseKind`],
//!   2. a new child struct parallel to [`WeightSession`] / [`CoreSession`] /
//!      [`RunningSession`],
//!   3. the new row in the `db` crate.
//!
//! Heartrate, auth and cross-cutting queries work unchanged.

pub mod duration_ext;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub use chrono;
pub use time;
pub use uuid;

// ---------------------------------------------------------------------------
// Exercise kinds
// ---------------------------------------------------------------------------

/// Tag for the kind of exercise a session represents.
///
/// Mirrors the `kind` column on `exercise_sessions` and the child-table
/// layout described in `DESIGN.md`. Order matters only for `Default`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExerciseKind {
    Weight,
    Core,
    Running,
}

impl ExerciseKind {
    /// All known variants — used by tests for exhaustive checks.
    pub const ALL: [Self; 3] = [Self::Weight, Self::Core, Self::Running];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Weight => "weight",
            Self::Core => "core",
            Self::Running => "running",
        }
    }
}

impl std::fmt::Display for ExerciseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExerciseKind {
    type Err = UnknownExerciseKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "weight" => Ok(Self::Weight),
            "core" => Ok(Self::Core),
            "running" => Ok(Self::Running),
            other => Err(UnknownExerciseKind(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown exercise kind: {0}")]
pub struct UnknownExerciseKind(pub String);

// ---------------------------------------------------------------------------
// Parent: ExerciseSession
// ---------------------------------------------------------------------------

/// One row of the `exercise_sessions` parent table — the cross-cutting
/// representation of any workout, regardless of type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciseSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: ExerciseKind,
    pub started_at: DateTime<Utc>,
    pub duration: std::time::Duration,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Payload accepted by `POST /api/exercise-sessions`.
///
/// `id`, `user_id` and `created_at` are server-assigned; they are filled in
/// by the repository on insert and never supplied by the client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewExerciseSession {
    pub kind: ExerciseKind,
    pub started_at: DateTime<Utc>,
    pub duration: std::time::Duration,
    pub notes: Option<String>,
}

// ---------------------------------------------------------------------------
// Child rows: typed per-kind detail columns
// ---------------------------------------------------------------------------

/// `weight_exercises` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // no Eq: f64 fields
pub struct WeightSession {
    pub session_id: Uuid,
    pub exercise_name: String,
    pub weight_kg: f64,
    pub sets: i32,
    pub reps: i32,
    pub quality: Option<i32>,
}

/// `core_exercises` row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreSession {
    pub session_id: Uuid,
    pub exercise_name: String,
    pub duration: std::time::Duration,
    pub quality: Option<i32>,
}

/// `running_sessions` row. `gpx_data` is the raw bytes of the GPX file
/// (stored as `BYTEA` per `DESIGN.md`). No `Eq` because `distance_m` is f64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunningSession {
    pub session_id: Uuid,
    pub distance_m: f64,
    /// Raw GPX blob — present only when the upload came via the bot or an
    /// explicit GPX upload through the API. `serde(skip)` for the API list
    /// view; clients that need the bytes hit `GET /api/runs/:id/gpx`.
    #[serde(skip)]
    pub gpx_data: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// Heartrate time-series
// ---------------------------------------------------------------------------

/// One sample in `heartrate_samples`. `(session_id, offset_secs)` is the PK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartrateSample {
    pub session_id: Uuid,
    /// Seconds from the session start.
    pub offset_secs: i32,
    pub bpm: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewHeartrateSamples {
    pub session_id: Uuid,
    pub samples: Vec<HeartrateSample>,
}

// ---------------------------------------------------------------------------
// Users & API tokens
// ---------------------------------------------------------------------------

/// Minimal user identity. `external_id` is the OIDC `sub` claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub external_id: String,
    pub display_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A long-lived bearer token issued via the web UI for machine clients
/// (e.g. the Matrix bot). The hashed form is what we store; the cleartext
/// form only exists transiently at creation time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    /// Arbitrary label identifying the client ("matrix-bot", "garmin-scraper").
    pub label: String,
    /// SHA-256 hash of the cleartext token — never the cleartext itself.
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Returned once at creation time — `token` is the only moment the
/// cleartext is visible to the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: String,
    /// Random 32-byte token, hex-encoded (64 chars). The DB stores only
    /// `sha256(token)`.
    pub token: String,
}

// ---------------------------------------------------------------------------
/// OIDC PKCE state. Ported from `workout_tracker/src/models.rs` (the
/// `Oidc`/`NewOidc` Diesel models). Lives in `core` because both `auth` and
/// `db` need to agree on its shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OidcState {
    pub csrf: String,
    pub nonce: String,
    pub code_verifier: String,
    pub resume_token: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewOidcState {
    pub csrf: String,
    pub nonce: String,
    pub code_verifier: String,
    pub resume_token: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Errors that [`Session::validate`] and friends can produce.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum ValidationError {
    #[error("duration must be positive (got {0:?})")]
    NonPositiveDuration(std::time::Duration),
    #[error("weight_kg must be positive (got {0})")]
    NonPositiveWeight(f64),
    #[error("sets must be positive (got {0})")]
    NonPositiveSets(i32),
    #[error("reps must be positive (got {0})")]
    NonPositiveReps(i32),
    #[error("bpm must be in 30..=220 (got {0})")]
    BpmOutOfRange(i16),
    #[error("offset_secs must be non-negative (got {0})")]
    NegativeOffset(i32),
    #[error("distance_m must be non-negative (got {0})")]
    NegativeDistance(f64),
    #[error("quality must be in 1..=10 (got {0})")]
    QualityOutOfRange(i32),
}

impl NewExerciseSession {
    /// Validate the cross-cutting fields.
    ///
    /// # Errors
    /// Returns [`ValidationError::NonPositiveDuration`] if `duration` is zero.
    pub const fn validate(&self) -> Result<(), ValidationError> {
        if self.duration.is_zero() {
            return Err(ValidationError::NonPositiveDuration(self.duration));
        }
        Ok(())
    }
}

impl WeightSession {
    /// Validate the per-column constraints the DB would enforce.
    ///
    /// # Errors
    /// See [`ValidationError`] variants.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.weight_kg <= 0.0 {
            return Err(ValidationError::NonPositiveWeight(self.weight_kg));
        }
        if self.sets <= 0 {
            return Err(ValidationError::NonPositiveSets(self.sets));
        }
        if self.reps <= 0 {
            return Err(ValidationError::NonPositiveReps(self.reps));
        }
        if let Some(q) = self.quality
            && !(1..=10).contains(&q)
        {
            return Err(ValidationError::QualityOutOfRange(q));
        }
        Ok(())
    }
}

impl CoreSession {
    /// `core_exercises` only has a `quality` column to check beyond the
    /// duration already validated on the parent.
    ///
    /// # Errors
    /// Returns [`ValidationError::QualityOutOfRange`] when present and outside 1..=10.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if let Some(q) = self.quality
            && !(1..=10).contains(&q)
        {
            return Err(ValidationError::QualityOutOfRange(q));
        }
        Ok(())
    }
}

impl RunningSession {
    /// Distances are non-negative; a zero-distance "run" is allowed
    /// (e.g. treadmill with no GPS lock that still produced a GPX).
    ///
    /// # Errors
    /// Returns [`ValidationError::NegativeDistance`] on negative input.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.distance_m < 0.0 {
            return Err(ValidationError::NegativeDistance(self.distance_m));
        }
        Ok(())
    }
}

impl HeartrateSample {
    /// Per-row sanity checks before bulk insert.
    ///
    /// # Errors
    /// See [`ValidationError`] variants.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.offset_secs < 0 {
            return Err(ValidationError::NegativeOffset(self.offset_secs));
        }
        if !(30..=220).contains(&self.bpm) {
            return Err(ValidationError::BpmOutOfRange(self.bpm));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]
    use super::*;

    fn session() -> NewExerciseSession {
        NewExerciseSession {
            kind: ExerciseKind::Weight,
            started_at: DateTime::parse_from_rfc3339("2026-07-16T08:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            duration: std::time::Duration::from_mins(1),
            notes: None,
        }
    }

    #[test]
    fn exercise_kind_round_trip() {
        for k in ExerciseKind::ALL {
            let s = serde_json::to_string(&k).unwrap();
            let back: ExerciseKind = serde_json::from_str(&s).unwrap();
            assert_eq!(k, back);
            assert_eq!(k.to_string(), k.as_str());
            let parsed: ExerciseKind = k.as_str().parse().unwrap();
            assert_eq!(k, parsed);
        }
    }

    #[test]
    fn exercise_kind_unknown_string() {
        assert!("yoga".parse::<ExerciseKind>().is_err());
    }

    #[test]
    fn positive_session_validates() {
        assert!(session().validate().is_ok());
    }

    #[test]
    fn zero_duration_rejected() {
        let mut s = session();
        s.duration = std::time::Duration::ZERO;
        assert_eq!(
            s.validate(),
            Err(ValidationError::NonPositiveDuration(
                std::time::Duration::ZERO
            ))
        );
    }

    #[test]
    fn weight_validation_catches_negatives() {
        let bad = WeightSession {
            session_id: Uuid::nil(),
            exercise_name: "bench".into(),
            weight_kg: -5.0,
            sets: 3,
            reps: 5,
            quality: None,
        };
        assert_eq!(
            bad.validate(),
            Err(ValidationError::NonPositiveWeight(-5.0))
        );
    }

    #[test]
    fn weight_quality_out_of_range() {
        let bad = WeightSession {
            session_id: Uuid::nil(),
            exercise_name: "bench".into(),
            weight_kg: 60.0,
            sets: 3,
            reps: 5,
            quality: Some(11),
        };
        assert_eq!(bad.validate(), Err(ValidationError::QualityOutOfRange(11)));
    }

    #[test]
    fn heartrate_bounds() {
        let good = HeartrateSample {
            session_id: Uuid::nil(),
            offset_secs: 0,
            bpm: 60,
        };
        assert!(good.validate().is_ok());

        let neg_offset = HeartrateSample {
            session_id: Uuid::nil(),
            offset_secs: -1,
            bpm: 60,
        };
        assert_eq!(
            neg_offset.validate(),
            Err(ValidationError::NegativeOffset(-1))
        );

        let low_bpm = HeartrateSample {
            session_id: Uuid::nil(),
            offset_secs: 0,
            bpm: 20,
        };
        assert_eq!(low_bpm.validate(), Err(ValidationError::BpmOutOfRange(20)));
    }
}
