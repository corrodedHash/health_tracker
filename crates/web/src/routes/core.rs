use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::CoreSession;
use health_db::{CoreRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewCorePayload {
    pub quality: Option<i32>,
}

#[utoipa::path(
    post,
    path = "/api/exercise-sessions/{id}/core",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Core exercise data added", body = serde_json::Value),
    ),
    tag = "core",
)]
pub async fn create(
    State(state): State<AppState>,
    UserId(_user_id): UserId,
    Path(session_id): Path<Uuid>,
    Json(body): Json<NewCorePayload>,
) -> Result<Json<serde_json::Value>, WebError> {
    let core = CoreSession {
        session_id,
        quality: body.quality,
    };
    core.validate()
        .map_err(|e| WebError::BadRequest(e.to_string()))?;
    let repo = SqlxRepository::new(state.pool.clone());
    CoreRepository::insert(&repo, session_id, &core).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/exercise-sessions/{id}/core",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Core exercise data"),
        (status = 404, description = "Session or core data not found"),
    ),
    tag = "core",
)]
pub async fn get(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<CoreSession>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let core = CoreRepository::get_by_session(&repo, session_id).await?;
    Ok(Json(core))
}
