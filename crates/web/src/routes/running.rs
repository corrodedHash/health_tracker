use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::RunningSession;
use health_db::{RunningRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewRunningPayload {
    pub distance_m: f64,
}

#[utoipa::path(
    post,
    path = "/api/exercise-sessions/{id}/running",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Running session data added", body = serde_json::Value),
    ),
    tag = "running",
)]
pub async fn create(
    State(state): State<AppState>,
    UserId(_user_id): UserId,
    Path(session_id): Path<Uuid>,
    Json(body): Json<NewRunningPayload>,
) -> Result<Json<serde_json::Value>, WebError> {
    let running = RunningSession {
        session_id,
        distance_m: body.distance_m,
        gpx_data: None,
    };
    running
        .validate()
        .map_err(|e| WebError::BadRequest(e.to_string()))?;
    let repo = SqlxRepository::new(state.pool.clone());
    RunningRepository::insert(&repo, session_id, &running).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/exercise-sessions/{id}/running",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Running session data"),
        (status = 404, description = "Session or running data not found"),
    ),
    tag = "running",
)]
pub async fn get(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<RunningSession>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let running = RunningRepository::get_by_session(&repo, session_id).await?;
    Ok(Json(running))
}
