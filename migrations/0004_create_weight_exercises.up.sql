-- weight_exercises: child table of the CTI layout for 'weight' sessions.
-- PK is also FK to exercise_sessions(id) with ON DELETE CASCADE so the
-- parent row's lifetime governs the child's. Per DESIGN.md §"Class
-- Table Inheritance". Mirrors health_core::WeightSession.
CREATE TABLE weight_exercises (
    session_id    UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    weight_kg     DOUBLE PRECISION NOT NULL,
    sets          INT NOT NULL,
    reps          INT NOT NULL,
    quality       INT
);

COMMENT ON TABLE weight_exercises IS 'Child rows for exercise_sessions of kind ''weight''';
COMMENT ON COLUMN weight_exercises.session_id IS 'FK to exercise_sessions.id (also the PK)';
COMMENT ON COLUMN weight_exercises.exercise_name IS 'E.g. ''bench press'', ''squat''';
COMMENT ON COLUMN weight_exercises.weight_kg IS 'Per-set weight in kilograms (must be > 0; enforced by health_core validation)';
COMMENT ON COLUMN weight_exercises.sets IS 'Number of sets performed (positive)';
COMMENT ON COLUMN weight_exercises.reps IS 'Reps per set (positive)';
COMMENT ON COLUMN weight_exercises.quality IS 'Optional 1-10 subjective feel rating';

-- Note: cross-table enforcement that exercise_sessions.kind = 'weight'
-- is not done with a CHECK constraint (Postgres CHECK cannot contain
-- subqueries). The SqlxRepository insert path (Phase 1 item 5.9) inserts
-- the parent and child row in a single transaction and refuses to insert
-- a child row whose kind discriminator doesn't match. A trigger-based
-- guard can be added later if belt-and-braces DB enforcement is wanted.