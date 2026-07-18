ALTER TABLE exercise_weight
  ADD COLUMN quality INT;

ALTER TABLE exercise_weight
  RENAME COLUMN weight_g TO weight_kg;

ALTER TABLE exercise_weight
  ALTER COLUMN weight_kg TYPE DOUBLE PRECISION USING (weight_kg::double precision / 1000.0);

ALTER TABLE exercise_weight
  ALTER COLUMN weight_kg SET DEFAULT 12,
  ALTER COLUMN sets SET DEFAULT 3;

ALTER TABLE exercise_core
  ADD COLUMN quality INT;

ALTER TABLE exercise_running
  ADD COLUMN quality INTEGER CHECK (quality IS NULL OR (quality >= 1 AND quality <= 10)),
  ALTER COLUMN distance_m TYPE DOUBLE PRECISION USING (distance_m::double precision),
  ALTER COLUMN moving_distance_m TYPE DOUBLE PRECISION USING (moving_distance_m::double precision);
