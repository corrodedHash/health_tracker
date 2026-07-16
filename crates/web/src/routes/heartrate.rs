use axum::Json;
use axum::extract::{Path, State};
use serde::Deserialize;
use uuid::Uuid;

use health_core::{HeartrateSample, NewHeartrateSamples};
use health_db::{HeartrateRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct HeartrateBody {
    pub samples: Vec<HeartrateSamplePayload>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct HeartrateSamplePayload {
    pub offset_secs: i32,
    pub bpm: i16,
}

#[utoipa::path(
    post,
    path = "/api/exercise-sessions/{id}/heartrate",
    params(
        ("id" = Uuid, Path, description = "Session UUID"),
    ),
    responses(
        (status = 200, description = "Heartrate data added",
            body = serde_json::Value),
    ),
    tag = "heartrate",
)]
pub async fn add(
    State(state): State<AppState>,
    UserId(_user_id): UserId,
    Path(session_id): Path<Uuid>,
    Json(body): Json<HeartrateBody>,
) -> Result<Json<serde_json::Value>, WebError> {
    let samples: Vec<HeartrateSample> = body
        .samples
        .into_iter()
        .map(|s| HeartrateSample {
            session_id,
            offset_secs: s.offset_secs,
            bpm: s.bpm,
        })
        .collect();

    let new = NewHeartrateSamples {
        session_id,
        samples,
    };

    for sample in &new.samples {
        sample
            .validate()
            .map_err(|e| WebError::BadRequest(e.to_string()))?;
    }

    let repo = SqlxRepository::new(state.pool.clone());
    let inserted = repo.insert_bulk(&new).await?;

    Ok(Json(serde_json::json!({ "inserted": inserted })))
}
