-- core_exercises: child table of the CTI layout for 'core' sessions
-- (plank, dead bug, etc.). Per DESIGN.md §"Class Table Inheritance".
-- Mirrors health_core::CoreSession.
CREATE TABLE core_exercises (
    session_id    UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    duration      INTERVAL NOT NULL,
    quality       INT
);

COMMENT ON TABLE core_exercises IS 'Child rows for exercise_sessions of kind ''core''';
COMMENT ON COLUMN core_exercises.session_id IS 'FK to exercise_sessions.id (also the PK)';
COMMENT ON COLUMN core_exercises.exercise_name IS 'E.g. ''plank'', ''dead bug''';
COMMENT ON COLUMN core_exercises.duration IS 'Hold duration as PostgreSQL INTERVAL';
COMMENT ON COLUMN core_exercises.quality IS 'Optional 1-10 subjective feel rating';

-- Cross-table kind enforcement (''core'') is handled in the
-- SqlxRepository insert transaction; see weight_exercises/up.sql note.