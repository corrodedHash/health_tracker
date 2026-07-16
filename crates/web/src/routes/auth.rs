use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;

use health_auth::flow;
use health_db::{OidcStateRepository, SqlxRepository, UsersRepository};

use crate::error::WebError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginParams {
    pub resume_token: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<LoginParams>,
) -> Result<Response, WebError> {
    if state.config.dev_auto_login {
        let repo = SqlxRepository::new(state.pool.clone());
        let user = repo
            .upsert_by_external_id("dev-user", Some("Dev User"))
            .await?;

        let session_data = health_auth::session::SessionData {
            user_id: user.id.to_string(),
        };
        let cookie_str = health_auth::session::create_session_cookie(
            &session_data,
            &state.cookie_key,
            time::Duration::hours(24),
        )
        .map_err(WebError::Session)?;

        let base = headers
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            .and_then(|r| {
                let idx = r.find("://")?;
                let rest = &r[idx + 3..];
                let slash = rest.find('/')?;
                Some(&r[..idx + 3 + slash])
            })
            .unwrap_or("http://localhost:5173");

        let location = params
            .resume_token
            .map_or_else(|| format!("{base}/"), |t| format!("{base}/?resume_token={t}"));

        return Ok((
            StatusCode::FOUND,
            [(header::SET_COOKIE, cookie_str)],
            Redirect::to(&location),
        )
            .into_response());
    }

    let bundle = state
        .oidc_bundle
        .as_deref()
        .ok_or_else(|| WebError::Internal(anyhow::anyhow!("OIDC not configured")))?;

    let request = flow::start_login(bundle, params.resume_token)
        .map_err(|e| WebError::Internal(anyhow::anyhow!("{e}")))?;

    let repo = SqlxRepository::new(state.pool.clone());
    repo.insert(&request.state).await?;

    Ok(Redirect::to(&request.auth_url).into_response())
}

pub async fn callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
) -> Result<Response, WebError> {
    let bundle = state
        .oidc_bundle
        .as_deref()
        .ok_or_else(|| WebError::Internal(anyhow::anyhow!("OIDC not configured")))?;

    let repo = SqlxRepository::new(state.pool.clone());
    let oidc_state = repo.fetch(&params.state).await?;

    let result = flow::finish_login(bundle, &params.code, &oidc_state)
        .await
        .map_err(|e| WebError::Internal(anyhow::anyhow!("{e}")))?;

    let user = repo.upsert_by_external_id(&result.sub, None).await?;

    let session_data = health_auth::session::SessionData {
        user_id: user.id.to_string(),
    };
    let cookie_str = health_auth::session::create_session_cookie(
        &session_data,
        &state.cookie_key,
        time::Duration::hours(24),
    )
    .map_err(WebError::Session)?;

    repo.delete(&params.state).await?;

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_str)],
        Redirect::to(&result.resume_location),
    )
        .into_response())
}

pub async fn logout(State(state): State<AppState>) -> Result<Response, WebError> {
    let cookie_str = health_auth::session::delete_session_cookie(&state.cookie_key);

    Ok((
        StatusCode::OK,
        [(header::SET_COOKIE, cookie_str)],
        Json(serde_json::json!({ "ok": true })),
    )
        .into_response())
}
