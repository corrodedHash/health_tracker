use std::io::Cursor;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use uuid::Uuid;

use health_core::{ExerciseKind, NewExerciseSession, RunningSession};
use health_db::{RunningRepository, SessionsRepository, SqlxRepository};

use crate::error::WebError;
use crate::middleware::session::UserId;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/runs/gpx",
    request_body(content = inline(serde_json::Value), description = "Raw GPX file bytes"),
    responses(
        (status = 200, description = "Run session created"),
    ),
    tag = "runs",
)]
pub async fn upload_gpx(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    body: Bytes,
) -> Result<Json<health_core::ExerciseSession>, WebError> {
    let (total_distance, total_duration, moving_distance, moving_duration) = parse_gpx(&body)?;

    let new_session = NewExerciseSession {
        kind: ExerciseKind::Running,
        started_at: health_core::chrono::Utc::now(),
        duration: total_duration,
        notes: None,
    };

    let repo = SqlxRepository::new(state.pool.clone());
    let session = SessionsRepository::insert(&repo, user_id, &new_session).await?;

    let running = RunningSession {
        session_id: session.id,
        distance_m: total_distance,
        quality: None,
        moving_distance_m: Some(moving_distance),
        moving_time: Some(moving_duration.as_secs_f64()),
        gpx_data: Some(body.to_vec()),
    };
    RunningRepository::insert(&repo, session.id, &running).await?;

    Ok(Json(session))
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}/gpx",
    params(
        ("id" = Uuid, Path, description = "Run session UUID"),
    ),
    responses(
        (status = 200, description = "GPX file content", content_type = "application/gpx+xml"),
        (status = 404, description = "GPX data not found"),
    ),
    tag = "runs",
)]
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

fn compute_moving_distance_time(gpx: &gpx::Gpx, kmh_threshold: f64) -> (f64, std::time::Duration) {
    let mut total_distance = 0.0;
    let mut total_secs: f64 = 0.0;

    for track in &gpx.tracks {
        for segment in &track.segments {
            for [a, b] in segment.points.array_windows() {
                let (Some(ta), Some(tb)) = (a.time, b.time) else {
                    continue;
                };

                let delta_time = health_core::time::OffsetDateTime::from(tb)
                    - health_core::time::OffsetDateTime::from(ta);
                let delta_secs = delta_time.as_seconds_f64();
                let delta_distance = haversine_rs::distance(
                    haversine_rs::point::Point::new(b.point().0.y, b.point().0.x),
                    haversine_rs::point::Point::new(a.point().0.y, a.point().0.x),
                    haversine_rs::units::Unit::Meters,
                );

                let hours = delta_secs / 3600.0;
                if hours <= 0.0 {
                    continue;
                }
                let kmh = (delta_distance / 1000.0) / hours;

                if kmh >= kmh_threshold {
                    total_distance += delta_distance;
                    total_secs += delta_secs;
                }
            }
        }
    }

    let total_duration = std::time::Duration::from_secs_f64(total_secs);

    (total_distance, total_duration)
}

fn parse_gpx(
    bytes: &[u8],
) -> Result<(f64, std::time::Duration, f64, std::time::Duration), WebError> {
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
                    let dt =
                        health_core::chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
                    if first_time.is_none() {
                        first_time = Some(dt);
                    }
                    last_time = Some(dt);
                }
            }
        }
    }

    let total_duration = match (first_time, last_time) {
        (Some(first), Some(last)) => last
            .signed_duration_since(first)
            .to_std()
            .map_err(|e| WebError::BadRequest(format!("invalid GPX duration: {e}")))?,
        _ => std::time::Duration::ZERO,
    };

    let (moving_distance, moving_duration) = compute_moving_distance_time(&gpx, 1.5);

    Ok((
        total_distance,
        total_duration,
        moving_distance,
        moving_duration,
    ))
}
