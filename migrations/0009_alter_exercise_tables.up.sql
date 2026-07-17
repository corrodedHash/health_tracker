-- Weight: remove reps and exercise_name (implied by type), add defaults for weight/sets
ALTER TABLE exercise_weight
  DROP COLUMN reps,
  DROP COLUMN exercise_name,
  ALTER COLUMN weight_kg SET DEFAULT 12,
  ALTER COLUMN sets SET DEFAULT 3;

-- Core: remove exercise_name (implied by type) and duration (use parent)
ALTER TABLE exercise_core
  DROP COLUMN exercise_name,
  DROP COLUMN duration;

-- Running: add quality (for all types), add moving distance/time (computed from GPX)
ALTER TABLE exercise_running
  ADD COLUMN quality integer CHECK (quality IS NULL OR (quality >= 1 AND quality <= 10)),
  ADD COLUMN moving_distance_m double precision,
  ADD COLUMN moving_time double precision;
