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
        result.distance_m > 6480.0 && result.distance_m < 6490.0,
        "moving distance should be ~6.5km, got {}",
        result.distance_m
    );
    assert!(
        result.duration.as_secs_f64() > 2180.0 && result.duration.as_secs_f64() < 2190.0,
        "moving duration should be ~2185s, got {}",
        result.duration.as_secs_f64()
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
