//! Batch import of exercise sessions via API-token auth.
//!
//! The web UI and the script client both create sessions one at a time, but
//! a CSV import sends many rows in a single request. Each row is applied
//! independently so a single bad row fails loudly without discarding the
//! rest of the batch. `started_at` is taken verbatim from the payload so the
//! original workout dates survive the import.

use axum::Json;
use axum::extract::State;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use health_core::{
    CoreSession, ExerciseKind, ExerciseSession, NewExerciseSession, RunningSession, WeightSession,
};
use health_db::{
    CoreRepository, RunningRepository, SessionsRepository, SqlxRepository, WeightRepository,
};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

/// One CSV row, mirroring `POST /api/exercise-sessions` plus the per-kind
/// child columns. Kind-specific fields are optional at the type level and
/// validated per row.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ImportRow {
    pub kind: ExerciseKind,
    pub started_at: chrono::DateTime<chrono::Utc>,
    #[serde(
        rename = "duration_secs",
        with = "health_core::duration_ext::serde_duration_secs"
    )]
    #[schema(value_type = f64)]
    pub duration: std::time::Duration,
    pub notes: Option<String>,
    pub quality: Option<i32>,
    /// Required when `kind = weight`.
    pub weight_g: Option<i32>,
    /// Required when `kind = weight`.
    pub sets: Option<i32>,
    /// Required when `kind = running`.
    pub distance_m: Option<i32>,
}

#[utoipa::path(
    post,
    path = "/api/import/sessions",
    responses(
        (status = 200, description = "Per-row import results"),
    ),
    tag = "import",
)]
pub async fn import(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(rows): Json<Vec<ImportRow>>,
) -> Result<Json<serde_json::Value>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let mut results = Vec::with_capacity(rows.len());

    for (index, row) in rows.into_iter().enumerate() {
        let result = match import_row(&repo, user_id, row).await {
            Ok(session) => json!({
                "index": index,
                "id": session.id,
                "started_at": session.started_at,
                "created_at": session.created_at,
            }),
            Err(e) => json!({ "index": index, "error": e }),
        };
        results.push(result);
    }

    Ok(Json(json!({ "results": results })))
}

async fn import_row(
    repo: &SqlxRepository,
    user_id: Uuid,
    row: ImportRow,
) -> Result<ExerciseSession, String> {
    let new_session = NewExerciseSession {
        kind: row.kind,
        started_at: row.started_at,
        duration: row.duration,
        notes: row.notes,
        quality: row.quality,
    };
    new_session.validate().map_err(|e| e.to_string())?;

    match new_session.kind {
        ExerciseKind::Weight => {
            let weight_g = row.weight_g.ok_or("kind=weight requires weight_g")?;
            let sets = row.sets.ok_or("kind=weight requires sets")?;
            let weight = WeightSession {
                session_id: Uuid::nil(),
                weight_g,
                sets,
            };
            weight.validate().map_err(|e| e.to_string())?;

            let session = SessionsRepository::insert(repo, user_id, &new_session)
                .await
                .map_err(|e| e.to_string())?;
            let weight = WeightSession {
                session_id: session.id,
                weight_g,
                sets,
            };
            if let Err(e) = WeightRepository::insert(repo, session.id, &weight).await {
                rollback(repo, session.id).await;
                return Err(e.to_string());
            }
            Ok(session)
        }
        ExerciseKind::Core => {
            let session = SessionsRepository::insert(repo, user_id, &new_session)
                .await
                .map_err(|e| e.to_string())?;
            let core = CoreSession {
                session_id: session.id,
            };
            if let Err(e) = CoreRepository::insert(repo, session.id, &core).await {
                rollback(repo, session.id).await;
                return Err(e.to_string());
            }
            Ok(session)
        }
        ExerciseKind::Running => {
            let distance_m = row.distance_m.ok_or("kind=running requires distance_m")?;
            let running = RunningSession {
                session_id: Uuid::nil(),
                distance_m,
                moving_distance_m: None,
                moving_time: None,
                gpx_data: None,
            };
            running.validate().map_err(|e| e.to_string())?;

            let session = SessionsRepository::insert(repo, user_id, &new_session)
                .await
                .map_err(|e| e.to_string())?;
            let running = RunningSession {
                session_id: session.id,
                distance_m,
                moving_distance_m: None,
                moving_time: None,
                gpx_data: None,
            };
            if let Err(e) = RunningRepository::insert(repo, session.id, &running).await {
                rollback(repo, session.id).await;
                return Err(e.to_string());
            }
            Ok(session)
        }
        ExerciseKind::Custom => SessionsRepository::insert(repo, user_id, &new_session)
            .await
            .map_err(|e| e.to_string()),
    }
}

/// Best-effort removal of the just-inserted parent row when the child insert
/// fails, so a failed row leaves no orphan behind (FK cascade removes the
/// child if one was partially written).
async fn rollback(repo: &SqlxRepository, session_id: Uuid) {
    let _ = SessionsRepository::delete(repo, session_id).await;
}
