-- exercises: parent table of the class-table-inheritance layout.
-- One row per workout regardless of type; per-type child tables
-- (exercise_weight, exercise_running, exercise_core) FK back to this
-- table's `id` with ON DELETE CASCADE.
--
-- Per DESIGN.md §"Class Table Inheritance" with a CHECK constraint on
-- `kind` added per MIGRATION.md §3 item 3 to mirror the
-- `ExerciseKind` enum in `health_core` (weight|core|running).
CREATE TABLE exercises (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id),
    kind        TEXT NOT NULL CHECK (kind IN ('weight','core','running')),
    started_at  TIMESTAMPTZ NOT NULL,
    duration    INTERVAL NOT NULL,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE exercises IS 'Parent row for every exercise session (CTI root)';
COMMENT ON COLUMN exercises.id IS 'Server-assigned row id (gen_random_uuid)';
COMMENT ON COLUMN exercises.user_id IS 'Owner of the session (FK users.id)';
COMMENT ON COLUMN exercises.kind IS 'Discriminator: weight | core | running (matches health_core::ExerciseKind)';
COMMENT ON COLUMN exercises.started_at IS 'When the session started (UTC, user-supplied)';
COMMENT ON COLUMN exercises.duration IS 'Session duration as PostgreSQL INTERVAL';
COMMENT ON COLUMN exercises.notes IS 'Optional free-form notes';
COMMENT ON COLUMN exercises.created_at IS 'Row creation timestamp (UTC)';

-- Speed up listing/filtering per session within a user; the common access
-- pattern is "sessions for user X between dates of kind Y".
CREATE INDEX exercises_user_started_at_idx
    ON exercises (user_id, started_at DESC);
CREATE INDEX exercises_user_kind_started_at_idx
    ON exercises (user_id, kind, started_at DESC);
