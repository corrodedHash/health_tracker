-- heartrate_samples: time-series heartrate data, one row per sample.
-- Composite PK (session_id, offset_secs) per DESIGN.md. The single FK
-- to exercise_sessions(id) means ANY session kind can carry heartrate
-- samples — that's the point of CTI: weight/core/running all share this
-- table without per-type fan-out. Mirrors health_core::HeartrateSample.
CREATE TABLE heartrate_samples (
    session_id   UUID NOT NULL REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    offset_secs  INTEGER NOT NULL,
    bpm          SMALLINT NOT NULL,
    PRIMARY KEY (session_id, offset_secs),
    CHECK (bpm > 0),
    CHECK (offset_secs >= 0)
);

COMMENT ON TABLE heartrate_samples IS 'Time-series heartrate samples; keyed by (session, offset)';
COMMENT ON COLUMN heartrate_samples.session_id IS 'FK to exercise_sessions.id (any kind)';
COMMENT ON COLUMN heartrate_samples.offset_secs IS 'Seconds from session start (monotonic per session)';
COMMENT ON COLUMN heartrate_samples.bpm IS 'Beats per minute (must be positive)';

-- Bulk insert idempotency: HeartrateRepository uses
-- INSERT ... ON CONFLICT (session_id, offset_secs) DO NOTHING so watch
-- exports can be replayed without duplicate-key errors.

-- Hot path is ''list samples for a session in offset order''; the PK
-- already serves that as a covering index. No extra index needed.