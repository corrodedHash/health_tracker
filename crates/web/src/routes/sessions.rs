use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use health_core::{ExerciseKind, ExerciseSession, NewExerciseSession};
use health_db::{SessionsRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, Serialize, utoipa::IntoParams, utoipa::ToSchema)]
pub struct ListParams {
    kind: Option<String>,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(
    get,
    path = "/api/exercise-sessions",
    params(ListParams),
    responses(
        (status = 200, description = "List all exercise sessions"),
    ),
    tag = "sessions",
)]
pub async fn list(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ExerciseSession>>, WebError> {
    let kind = params
        .kind
        .map(|k| k.parse::<ExerciseKind>())
        .transpose()
        .map_err(|e| WebError::BadRequest(e.to_string()))?;

    let repo = SqlxRepository::new(state.pool.clone());
    let sessions = repo.list(user_id, kind, params.from, params.to).await?;
    Ok(Json(sessions))
}

#[utoipa::path(
    post,
    path = "/api/exercise-sessions",
    responses(
        (status = 200, description = "Session created"),
    ),
    tag = "sessions",
)]
pub async fn create(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(new): Json<NewExerciseSession>,
) -> Result<Json<ExerciseSession>, WebError> {
    new.validate()
        .map_err(|e| WebError::BadRequest(e.to_string()))?;
    let repo = SqlxRepository::new(state.pool.clone());
    let session = repo.insert(user_id, &new).await?;
    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/api/exercise-sessions/{id}",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Session found"),
        (status = 404, description = "Session not found"),
    ),
    tag = "sessions",
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExerciseSession>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let session = repo.get(id).await?;
    Ok(Json(session))
}

#[utoipa::path(
    delete,
    path = "/api/exercise-sessions/{id}",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Session deleted", body = serde_json::Value),
        (status = 404, description = "Session not found"),
    ),
    tag = "sessions",
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let deleted = repo.delete(id).await?;
    if !deleted {
        return Err(WebError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
