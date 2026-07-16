-- running_sessions: child table of the CTI layout for 'running' sessions.
-- Per DESIGN.md §"Why BLOBs, not filesystem for GPX" the raw GPX file is
-- stored inline as BYTEA so deletes are atomic and pg_dump captures
-- everything. Mirrors health_core::RunningSession.
CREATE TABLE running_sessions (
    session_id   UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    distance_m   DOUBLE PRECISION NOT NULL,
    gpx_data     BYTEA
);

COMMENT ON TABLE running_sessions IS 'Child rows for exercise_sessions of kind ''running''';
COMMENT ON COLUMN running_sessions.session_id IS 'FK to exercise_sessions.id (also the PK)';
COMMENT ON COLUMN running_sessions.distance_m IS 'Total distance covered, in meters';
COMMENT ON COLUMN running_sessions.gpx_data IS 'Raw GPX file as BYTEA; served verbatim via GET /api/runs/:id/gpx';

-- Cross-table kind enforcement (''running'') is handled in the
-- SqlxRepository insert transaction; see weight_exercises/up.sql note.