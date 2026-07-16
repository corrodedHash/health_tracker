use std::io::Cursor;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use uuid::Uuid;

use health_core::{ExerciseKind, NewExerciseSession, RunningSession};
use health_db::{RunningRepository, SessionsRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

pub async fn upload_gpx(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    body: Bytes,
) -> Result<Json<health_core::ExerciseSession>, WebError> {
    let (distance_m, duration) = parse_gpx(&body)?;

    let new_session = NewExerciseSession {
        kind: ExerciseKind::Running,
        started_at: health_core::chrono::Utc::now(),
        duration,
        notes: None,
    };

    let repo = SqlxRepository::new(state.pool.clone());
    let session = SessionsRepository::insert(&repo, user_id, &new_session).await?;

    let running = RunningSession {
        session_id: session.id,
        distance_m,
        gpx_data: Some(body.to_vec()),
    };
    RunningRepository::insert(&repo, session.id, &running).await?;

    Ok(Json(session))
}

pub async fn get_gpx(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, WebError> {
    let repo = SqlxRepository::new(state.pool.clone());
    let gpx_data = RunningRepository::get_gpx(&repo, id).await?;

    gpx_data.map_or_else(
        || Err(WebError::NotFound),
        |bytes| {
            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/gpx+xml")],
                bytes,
            )
                .into_response())
        },
    )
}

fn parse_gpx(bytes: &[u8]) -> Result<(f64, std::time::Duration), WebError> {
    let cursor = Cursor::new(bytes);
    let gpx = gpx::read(cursor).map_err(|e| WebError::BadRequest(format!("invalid GPX: {e}")))?;

    let mut total_distance: f64 = 0.0;
    let mut first_time: Option<health_core::chrono::DateTime<health_core::chrono::Utc>> = None;
    let mut last_time: Option<health_core::chrono::DateTime<health_core::chrono::Utc>> = None;
    let mut prev_point: Option<haversine_rs::point::Point> = None;

    for track in &gpx.tracks {
        for segment in &track.segments {
            for point in &segment.points {
                let lat = point.point().y();
                let lon = point.point().x();

                if let Some(prev) = prev_point {
                    let curr = haversine_rs::point::Point::new(lat, lon);
                    total_distance +=
                        haversine_rs::distance(prev, curr, haversine_rs::units::Unit::Meters);
                }

                prev_point = Some(haversine_rs::point::Point::new(lat, lon));

                if let Some(time) = point.time {
                    let odt: health_core::time::OffsetDateTime = time.into();
                    let secs = odt.unix_timestamp();
                    let dt = health_core::chrono::DateTime::from_timestamp(secs, 0)
                        .unwrap_or_default();
                    if first_time.is_none() {
                        first_time = Some(dt);
                    }
                    last_time = Some(dt);
                }
            }
        }
    }

    let duration = match (first_time, last_time) {
        (Some(first), Some(last)) => last
            .signed_duration_since(first)
            .to_std()
            .map_err(|e| WebError::BadRequest(format!("invalid GPX duration: {e}")))?,
        _ => std::time::Duration::ZERO,
    };

    Ok((total_distance, duration))
}
