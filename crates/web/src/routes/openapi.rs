use utoipa::OpenApi;

use health_core::{ApiToken, ExerciseKind, ExerciseSession, NewApiToken, NewExerciseSession};

use super::auth;
use super::heartrate;
use super::runs;
use super::sessions;
use super::tokens;

/// Shared error body returned by all API endpoints on failure.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
pub(super) struct ErrorResponse {
    pub error: String,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Health Tracker API",
        version = "0.1.0",
        description = "Workout and exercise session tracker"
    ),
    paths(
        sessions::list,
        sessions::create,
        sessions::get,
        sessions::delete,
        heartrate::add,
        runs::upload_gpx,
        runs::get_gpx,
        tokens::issue,
        tokens::list,
        tokens::revoke,
        auth::login,
        auth::callback,
        auth::logout,
        auth::status,
    ),
    components(schemas(
        ExerciseKind,
        ExerciseSession,
        NewExerciseSession,
        ApiToken,
        NewApiToken,
        sessions::ListParams,
        heartrate::HeartrateBody,
        heartrate::HeartrateSamplePayload,
        tokens::IssueTokenBody,
        ErrorResponse,
    )),
    tags(
        (name = "sessions", description = "Exercise session CRUD"),
        (name = "heartrate", description = "Heartrate time-series data"),
        (name = "runs", description = "GPX upload and retrieval"),
        (name = "tokens", description = "API token management"),
        (name = "auth", description = "Authentication (OIDC login/logout)"),
    ),
)]
pub(super) struct ApiDoc;

pub async fn serve() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}
