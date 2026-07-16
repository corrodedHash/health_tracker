use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("unauthorized")]
    #[allow(dead_code)]
    Unauthorized,

    #[error("not found")]
    NotFound,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    #[allow(dead_code)]
    Conflict(String),

    #[error("internal error")]
    Internal(#[from] anyhow::Error),

    #[error("database error")]
    Database(#[from] health_db::DbError),

    #[error("session error")]
    Session(#[from] health_auth::session::SessionError),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Internal(_) | Self::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Self::Unauthorized | Self::Session(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            #[allow(dead_code)]
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
        };

        let body = json!({ "error": message });
        (status, axum::Json(body)).into_response()
    }
}
