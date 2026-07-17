ALTER TABLE exercises
  DROP CONSTRAINT exercises_kind_check,
  ADD CONSTRAINT exercises_kind_check CHECK (kind IN ('weight','core','running'));

ALTER TABLE exercises
  DROP COLUMN quality;
