//! GPX parsing + moving-distance computation.
//!
//! Ported verbatim from `matrix-running/src/routes.rs` (item 5.23).
//! The `get_track_moving_distance_time` / `get_segment_distance_time`
//! functions are unchanged. The [`process_gpx`] pure function wraps them
//! so the sync loop can stay thin and tests can hit the logic without
//! going through matrix-sdk.

use std::time::Duration as StdDuration;

use anyhow::Context;
use gpx::{Track, TrackSegment};
use time::OffsetDateTime;

use health_core::duration_ext;

fn get_segment_distance_time(s: &TrackSegment) -> impl Iterator<Item = (f64, time::Duration)> {
    s.points.array_windows().filter_map(|[a, b]| {
        let (Some(ta), Some(tb)) = (a.time, b.time) else {
            return None;
        };
        let delta_time = OffsetDateTime::from(tb) - OffsetDateTime::from(ta);
        let delta_distance = haversine_rs::distance(
            haversine_rs::point::Point::new(b.point().0.y, b.point().0.x),
            haversine_rs::point::Point::new(a.point().0.y, a.point().0.x),
            haversine_rs::units::Unit::Meters,
        );
        Some((delta_distance, delta_time))
    })
}

#[must_use]
pub fn get_track_moving_distance_time(t: &Track, kmh_threshold: f64) -> (f64, time::Duration) {
    t.segments
        .iter()
        .flat_map(|v| {
            get_segment_distance_time(v).filter(|(distance, time)| {
                let hours = time.as_seconds_f64() / (60. * 60.);
                let kilometers = distance / 1000.;
                let kmh = kilometers / hours;
                kmh >= kmh_threshold
            })
        })
        .fold((0.0, time::Duration::default()), |(ia, ib), (na, nb)| {
            (ia + na, ib + nb)
        })
}

pub struct GpxResult {
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub total_distance_m: f64,
    pub total_duration: StdDuration,
    pub moving_distance_m: f64,
    pub moving_duration: StdDuration,
}

/// Parse GPX bytes and compute the run's total and moving distance + duration.
///
/// Moving values are computed with a speed threshold of 1.5 km/h (walking
/// speed), filtering out stationary segments.
///
/// # Errors
/// Returns an error if the bytes cannot be parsed as GPX, the file
/// contains no tracks, or the metadata lacks a `<time>` element.
pub fn process_gpx(bytes: &[u8]) -> anyhow::Result<GpxResult> {
    let reader = std::io::BufReader::new(bytes);
    let gpx = gpx::read(reader).context("Parsing gpx file")?;
    let t = gpx
        .tracks
        .first()
        .ok_or_else(|| anyhow::anyhow!("GPX file contains no tracks"))?;

    // Moving: filter by speed threshold.
    let (moving_distance, moving_duration_time) = get_track_moving_distance_time(t, 1.5);

    // Total: include all points (threshold 0).
    let (total_distance, total_duration_time) = get_track_moving_distance_time(t, 0.0);

    let started_time = gpx
        .metadata
        .as_ref()
        .and_then(|md| md.time)
        .ok_or_else(|| anyhow::anyhow!("GPX file did not contain time metadata"))?;
    let started_at = time_odt_to_chrono(started_time.into())?;

    Ok(GpxResult {
        started_at,
        total_distance_m: total_distance,
        total_duration: duration_ext::from_time(total_duration_time),
        moving_distance_m: moving_distance,
        moving_duration: duration_ext::from_time(moving_duration_time),
    })
}

fn time_odt_to_chrono(t: time::OffsetDateTime) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp(t.unix_timestamp(), t.nanosecond())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp in GPX metadata"))
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "We are in test module")]

    use std::{fs::File, io::BufReader};

    use crate::gpx::get_track_moving_distance_time;

    #[test]
    fn test_distance() {
        let file = File::open("tests/fixtures/run.gpx").unwrap();
        let reader = BufReader::new(file);
        let gpx = gpx::read(reader).unwrap();
        let d = get_track_moving_distance_time(&gpx.tracks[0], 0.0).0;
        assert!(d > 6480.);
        assert!(d < 6490.);
    }

    #[test]
    fn test_duration() {
        let file = File::open("tests/fixtures/run.gpx").unwrap();
        let reader = BufReader::new(file);
        let gpx = gpx::read(reader).unwrap();
        let t = get_track_moving_distance_time(&gpx.tracks[0], 0.0)
            .1
            .as_seconds_f64();
        assert!(t > 2180.);
        assert!(t < 2190.);
    }
}
