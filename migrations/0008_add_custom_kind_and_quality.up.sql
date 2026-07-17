ALTER TABLE exercises
  DROP CONSTRAINT exercises_kind_check,
  ADD CONSTRAINT exercises_kind_check CHECK (kind IN ('weight','core','running','custom'));

ALTER TABLE exercises
  ADD COLUMN quality SMALLINT CHECK (quality IS NULL OR (quality >= 1 AND quality <= 10));

COMMENT ON COLUMN exercises.quality IS 'Subjective quality rating 1-10 (optional, across all exercise types)';
