use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::ApiToken;
use health_db::{ApiTokenRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct IssueTokenBody {
    pub label: String,
}

#[utoipa::path(
    post,
    path = "/api/tokens",
    responses(
        (status = 200, description = "Token issued"),
    ),
    tag = "tokens",
)]
pub async fn issue(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(body): Json<IssueTokenBody>,
) -> Result<Json<health_core::NewApiToken>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let token = repo.issue(user_id, &body.label).await?;
    Ok(Json(token))
}

#[utoipa::path(
    get,
    path = "/api/tokens",
    responses(
        (status = 200, description = "List of API tokens"),
    ),
    tag = "tokens",
)]
pub async fn list(
    State(state): State<AppState>,
    UserId(user_id): UserId,
) -> Result<Json<Vec<ApiToken>>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let tokens = repo.list_for_user(user_id).await?;
    Ok(Json(tokens))
}

#[utoipa::path(
    delete,
    path = "/api/tokens/{id}",
    params(
        ("id" = Uuid, Path, description = "Token UUID"),
    ),
    responses(
        (status = 200, description = "Token revoked"),
        (status = 404, description = "Token not found"),
    ),
    tag = "tokens",
)]
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
