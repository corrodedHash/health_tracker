//! Account-link endpoints for the Matrix bot flow.
//!
//! The bot POSTs `/api/links` (bearer-authed) to mint a single-use code, a
//! user confirms it in the browser (`POST /api/links/{code}/confirm`,
//! session-authed), and the bot polls `GET /api/links/{code}` back to
//! receive a freshly issued API token bound to that user.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use chrono::Utc;
use uuid::Uuid;

use health_db::{ApiTokenRepository, PendingLinkRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

/// How long a bot-initiated link stays valid before the user must confirm.
const LINK_TTL_MINUTES: i64 = 15;

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct NewPendingLink {
    pub code: String,
    pub url: String,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum LinkStatus {
    Pending,
    Accepted,
    Expired,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LinkPollResponse {
    pub status: LinkStatus,
    /// The freshly issued token — present only once, on the first poll
    /// after the user accepts.
    pub token: Option<String>,
}

/// Public origin used to build the browser-facing confirmation URL.
/// Prefers `public_base_url`, falls back to `frontend_url`, then to the
/// request's `Host` header (http assumed; set `public_base_url` behind a
/// TLS-terminating proxy).
fn public_origin(state: &AppState, headers: &HeaderMap) -> Option<String> {
    state
        .config
        .public_base_url
        .clone()
        .or_else(|| state.config.frontend_url.clone())
        .or_else(|| {
            headers
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .map(|host| format!("http://{host}"))
        })
}

#[utoipa::path(
    post,
    path = "/api/links",
    responses(
        (status = 200, description = "Link created; user must confirm in the browser"),
    ),
    tag = "links",
)]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    _user: UserId,
) -> Result<Json<NewPendingLink>, WebError> {
    let code = Uuid::new_v4().simple().to_string();
    let expires_at = Utc::now() + chrono::TimeDelta::minutes(LINK_TTL_MINUTES);

    let repo = SqlxRepository::new(state.pool.clone());
    repo.create(&code, expires_at).await?;

    let url = public_origin(&state, &headers).map_or_else(
        || format!("/link?code={code}"),
        |origin| format!("{}/link?code={code}", origin.trim_end_matches('/')),
    );

    Ok(Json(NewPendingLink { code, url }))
}

#[utoipa::path(
    post,
    path = "/api/links/{code}/confirm",
    params(
        ("code" = String, Path, description = "Link code"),
    ),
    responses(
        (status = 200, description = "Account linked to the bot"),
    ),
    tag = "links",
)]
pub async fn confirm(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    repo.confirm(&code, user_id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[utoipa::path(
    get,
    path = "/api/links/{code}",
    params(
        ("code" = String, Path, description = "Link code"),
    ),
    responses(
        (status = 200, description = "Link status; token present only on the first poll after acceptance"),
    ),
    tag = "links",
)]
pub async fn poll(
    State(state): State<AppState>,
    _user: UserId,
    Path(code): Path<String>,
) -> Result<Json<LinkPollResponse>, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let link = repo.fetch(&code).await?;

    let Some(user_id) = link.user_id else {
        let status = if link.expires_at < Utc::now() {
            LinkStatus::Expired
        } else {
            LinkStatus::Pending
        };
        return Ok(Json(LinkPollResponse {
            status,
            token: None,
        }));
    };

    // Atomically claim the single-use token return before issuing, so two
    // concurrent polls can never both create a token for the same link.
    // Only the poll that wins the `mark_token_returned` race issues one.
    if !repo.mark_token_returned(&code).await? {
        return Ok(Json(LinkPollResponse {
            status: LinkStatus::Accepted,
            token: None,
        }));
    }

    let token = repo.issue(user_id, &format!("matrix-bot:{code}")).await?;

    Ok(Json(LinkPollResponse {
        status: LinkStatus::Accepted,
        token: Some(token.token),
    }))
}
