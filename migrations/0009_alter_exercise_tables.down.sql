-- Revert weight: add back reps and exercise_name, drop defaults
ALTER TABLE exercise_weight
  ADD COLUMN reps integer,
  ADD COLUMN exercise_name text;

UPDATE exercise_weight SET reps = 10, exercise_name = 'unknown';

ALTER TABLE exercise_weight
  ALTER COLUMN reps SET NOT NULL,
  ALTER COLUMN exercise_name SET NOT NULL,
  ALTER COLUMN weight_kg DROP DEFAULT,
  ALTER COLUMN sets DROP DEFAULT;

-- Revert core: add back exercise_name and duration
ALTER TABLE exercise_core
  ADD COLUMN exercise_name text,
  ADD COLUMN duration interval;

UPDATE exercise_core SET exercise_name = 'unknown', duration = '0 seconds'::interval;

ALTER TABLE exercise_core
  ALTER COLUMN exercise_name SET NOT NULL,
  ALTER COLUMN duration SET NOT NULL;

-- Revert running: drop quality and moving columns
ALTER TABLE exercise_running
  DROP COLUMN quality,
  DROP COLUMN moving_distance_m,
  DROP COLUMN moving_time;
