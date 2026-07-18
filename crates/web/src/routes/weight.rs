use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::WeightSession;
use health_db::{SqlxRepository, WeightRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewWeightPayload {
    pub weight_g: i32,
    pub sets: i32,
}

#[utoipa::path(
    post,
    path = "/api/exercise-sessions/{id}/weight",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Weight exercise data added", body = serde_json::Value),
    ),
    tag = "weight",
)]
pub async fn create(
    State(state): State<AppState>,
    UserId(_user_id): UserId,
    Path(session_id): Path<Uuid>,
    Json(body): Json<NewWeightPayload>,
) -> Result<Json<serde_json::Value>, WebError> {
    let weight = WeightSession {
        session_id,
        weight_g: body.weight_g,
        sets: body.sets,
    };
    weight
        .validate()
        .map_err(|e| WebError::BadRequest(e.to_string()))?;
    let repo = SqlxRepository::new(state.pool.clone());
    WeightRepository::insert(&repo, session_id, &weight).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/exercise-sessions/{id}/weight",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Weight exercise data"),
        (status = 404, description = "Session or weight data not found"),
    ),
    tag = "weight",
)]
pub async fn get(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<WeightSession>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let weight = WeightRepository::get_by_session(&repo, session_id).await?;
    Ok(Json(weight))
}
