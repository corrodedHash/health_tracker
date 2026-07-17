-- exercise_running: child table of the CTI layout for 'running' sessions.
-- Per DESIGN.md §"Why BLOBs, not filesystem for GPX" the raw GPX file is
-- stored inline as BYTEA so deletes are atomic and pg_dump captures
-- everything. Mirrors health_core::RunningSession.
CREATE TABLE exercise_running (
    session_id   UUID PRIMARY KEY REFERENCES exercises(id) ON DELETE CASCADE,
    distance_m   DOUBLE PRECISION NOT NULL,
    gpx_data     BYTEA
);

COMMENT ON TABLE exercise_running IS 'Child rows for exercises of kind ''running''';
COMMENT ON COLUMN exercise_running.session_id IS 'FK to exercises.id (also the PK)';
COMMENT ON COLUMN exercise_running.distance_m IS 'Total distance covered, in meters';
COMMENT ON COLUMN exercise_running.gpx_data IS 'Raw GPX file as BYTEA; served verbatim via GET /api/runs/:id/gpx';

-- Cross-table kind enforcement (''running'') is handled in the
-- SqlxRepository insert transaction; see exercise_weight/up.sql note.
