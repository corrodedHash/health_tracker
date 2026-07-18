-- Change quality constraint from 1-10 to 1-5 across all tables

ALTER TABLE exercises
  DROP CONSTRAINT IF EXISTS exercises_quality_check,
  ADD CONSTRAINT exercises_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 5));

ALTER TABLE exercise_weight
  ADD CONSTRAINT exercise_weight_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 5));

ALTER TABLE exercise_core
  ADD CONSTRAINT exercise_core_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 5));

ALTER TABLE exercise_running
  DROP CONSTRAINT IF EXISTS exercise_running_quality_check,
  ADD CONSTRAINT exercise_running_quality_check CHECK (quality IS NULL OR (quality >= 1 AND quality <= 5));

COMMENT ON COLUMN exercises.quality IS 'Subjective quality rating 1-5 (optional, across all exercise types)';
COMMENT ON COLUMN exercise_weight.quality IS 'Optional 1-5 subjective feel rating';
COMMENT ON COLUMN exercise_core.quality IS 'Optional 1-5 subjective feel rating';
