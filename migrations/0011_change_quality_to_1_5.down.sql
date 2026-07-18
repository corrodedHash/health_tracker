-- Restore quality constraint from 1-5 back to 1-10

ALTER TABLE exercises
  DROP CONSTRAINT IF EXISTS exercises_quality_check,
  ADD CONSTRAINT exercises_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 10));

ALTER TABLE exercise_weight
  DROP CONSTRAINT IF EXISTS exercise_weight_quality_check;

ALTER TABLE exercise_core
  DROP CONSTRAINT IF EXISTS exercise_core_quality_check;

ALTER TABLE exercise_running
  DROP CONSTRAINT IF EXISTS exercise_running_quality_check,
  ADD CONSTRAINT exercise_running_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 10));

COMMENT ON COLUMN exercises.quality IS 'Subjective quality rating 1-10 (optional, across all exercise types)';
COMMENT ON COLUMN exercise_weight.quality IS 'Optional 1-10 subjective feel rating';
COMMENT ON COLUMN exercise_core.quality IS 'Optional 1-10 subjective feel rating';
