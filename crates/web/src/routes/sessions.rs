use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::{ExerciseKind, ExerciseSession, NewExerciseSession};
use health_db::{SessionsRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    kind: Option<String>,
    from: Option<chrono::DateTime<chrono::Utc>>,
    to: Option<chrono::DateTime<chrono::Utc>>,
}

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

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExerciseSession>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let session = repo.get(id).await?;
    Ok(Json(session))
}

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
