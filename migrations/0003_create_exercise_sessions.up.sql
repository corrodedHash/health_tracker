-- exercise_sessions: parent table of the class-table-inheritance layout.
-- One row per workout regardless of type; per-type child tables
-- (weight_exercises, running_sessions, core_exercises) FK back to this
-- table's `id` with ON DELETE CASCADE.
--
-- Per DESIGN.md §"Class Table Inheritance" with a CHECK constraint on
-- `kind` added per MIGRATION.md §3 item 3 to mirror the
-- `ExerciseKind` enum in `health_core` (weight|core|running).
CREATE TABLE exercise_sessions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id),
    kind        TEXT NOT NULL CHECK (kind IN ('weight','core','running')),
    started_at  TIMESTAMPTZ NOT NULL,
    duration    INTERVAL NOT NULL,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE exercise_sessions IS 'Parent row for every exercise session (CTI root)';
COMMENT ON COLUMN exercise_sessions.id IS 'Server-assigned row id (gen_random_uuid)';
COMMENT ON COLUMN exercise_sessions.user_id IS 'Owner of the session (FK users.id)';
COMMENT ON COLUMN exercise_sessions.kind IS 'Discriminator: weight | core | running (matches health_core::ExerciseKind)';
COMMENT ON COLUMN exercise_sessions.started_at IS 'When the session started (UTC, user-supplied)';
COMMENT ON COLUMN exercise_sessions.duration IS 'Session duration as PostgreSQL INTERVAL';
COMMENT ON COLUMN exercise_sessions.notes IS 'Optional free-form notes';
COMMENT ON COLUMN exercise_sessions.created_at IS 'Row creation timestamp (UTC)';

-- Speed up listing/filtering per session within a user; the common access
-- pattern is "sessions for user X between dates of kind Y".
CREATE INDEX exercise_sessions_user_started_at_idx
    ON exercise_sessions (user_id, started_at DESC);
CREATE INDEX exercise_sessions_user_kind_started_at_idx
    ON exercise_sessions (user_id, kind, started_at DESC);