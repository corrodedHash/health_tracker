-- exercise_core: child table of the CTI layout for 'core' sessions
-- (plank, dead bug, etc.). Per DESIGN.md §"Class Table Inheritance".
-- Mirrors health_core::CoreSession.
CREATE TABLE exercise_core (
    session_id    UUID PRIMARY KEY REFERENCES exercises(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    duration      INTERVAL NOT NULL,
    quality       INT
);

COMMENT ON TABLE exercise_core IS 'Child rows for exercises of kind ''core''';
COMMENT ON COLUMN exercise_core.session_id IS 'FK to exercises.id (also the PK)';
COMMENT ON COLUMN exercise_core.exercise_name IS 'E.g. ''plank'', ''dead bug''';
COMMENT ON COLUMN exercise_core.duration IS 'Hold duration as PostgreSQL INTERVAL';
COMMENT ON COLUMN exercise_core.quality IS 'Optional 1-10 subjective feel rating';

-- Cross-table kind enforcement (''core'') is handled in the
-- SqlxRepository insert transaction; see exercise_weight/up.sql note.
