use axum::Json;
use axum::extract::{Path, State};
use uuid::Uuid;

use health_core::ApiToken;
use health_db::{ApiTokenRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct IssueTokenBody {
    pub label: String,
}

pub async fn issue(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(body): Json<IssueTokenBody>,
) -> Result<Json<health_core::NewApiToken>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let token = repo.issue(user_id, &body.label).await?;
    Ok(Json(token))
}

pub async fn list(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<ApiToken>>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let tokens = repo.list_for_user(user_id).await?;
    Ok(Json(tokens))
}

pub async fn revoke(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(token_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let tokens = repo.list_for_user(user_id).await?;
    if !tokens.iter().any(|t| t.id == token_id) {
        return Err(WebError::NotFound);
    }
    repo.revoke(token_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
