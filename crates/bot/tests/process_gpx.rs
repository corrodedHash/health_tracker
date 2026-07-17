//! Integration tests for `process_gpx` against the copied `run.gpx`
//! fixture (item 5.29).

#![allow(clippy::unwrap_used, reason = "tests")]

use std::fs;

use health_bot::gpx::process_gpx;

#[test]
fn process_gpx_fixture_extracts_started_at_distance_duration() {
    let bytes = fs::read("tests/fixtures/run.gpx").unwrap();
    let result = process_gpx(&bytes).unwrap();

    assert!(
        result.total_distance_m > 6480.0,
        "total distance should be >= ~6.5km, got {}",
        result.total_distance_m
    );
    assert!(
        result.total_duration.as_secs_f64() > 2180.0,
        "total duration should be >= ~2185s, got {}",
        result.total_duration.as_secs_f64()
    );
    assert!(
        result.moving_distance_m > 0.0 && result.moving_distance_m <= result.total_distance_m,
        "moving distance should be positive and <= total"
    );
    assert!(
        result.moving_duration.as_secs_f64() > 0.0
            && result.moving_duration.as_secs_f64() <= result.total_duration.as_secs_f64(),
        "moving duration should be positive and <= total"
    );
    assert_eq!(
        result.started_at,
        chrono::DateTime::parse_from_rfc3339("2025-05-16T11:12:18Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    );
}

#[test]
fn process_gpx_empty_bytes_returns_error() {
    let result = process_gpx(b"");
    assert!(result.is_err());
}

#[test]
fn process_gpx_invalid_xml_returns_error() {
    let result = process_gpx(b"not xml at all <><>");
    assert!(result.is_err());
}
