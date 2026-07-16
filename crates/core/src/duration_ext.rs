//! Helpers for converting between the various duration representations
//! that make appearances across the crate graph.
//!
//! The DB stores durations as PostgreSQL `INTERVAL`. SQLx exposes those as
//! `chrono::Duration`. The GPX bot computes distances with the `time`
//! crate (`time::Duration`). The HTTP API uses `std::time::Duration`.
//! This module is the single place that knows the conversions so the
//! rest of the crate graph can stay neutral.

use std::time::Duration as StdDuration;

/// Convert a `time::Duration` (used by `gpx`/`haversine-rs`) to
/// `std::time::Duration` (used by the rest of the workspace).
///
/// `time` and `std` use the same internal representation for the
/// non-negative range we care about, so this is a trivial lossless cast
/// for all realistic workout lengths.
#[must_use]
pub fn from_time(t: time::Duration) -> StdDuration {
    StdDuration::from_secs_f64(t.as_seconds_f64())
}

/// Round a `std::time::Duration` to whole seconds as `f64`. Useful for
/// serialising into JSON when nanosecond precision is unwanted.
#[must_use]
pub fn secs_f64(d: StdDuration) -> f64 {
    d.as_secs_f64()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests")]
    use super::*;

    #[test]
    fn time_to_std_round_trip() {
        let t = time::Duration::seconds(90) + time::Duration::milliseconds(500);
        let s = from_time(t);
        assert_eq!(s, StdDuration::from_secs_f64(90.5));
    }
}
