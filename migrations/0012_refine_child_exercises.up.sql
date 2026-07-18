-- Remove quality from child tables (now on parent `exercises`).
-- Store weight in grams as integer, distance in meters as integer.
-- Drop database defaults for weight sets (frontend/backend provide them).

ALTER TABLE exercise_weight
  DROP COLUMN quality,
  ALTER COLUMN weight_kg DROP DEFAULT,
  ALTER COLUMN sets DROP DEFAULT;

ALTER TABLE exercise_weight
  RENAME COLUMN weight_kg TO weight_g;

ALTER TABLE exercise_weight
  ALTER COLUMN weight_g TYPE INTEGER USING (weight_g * 1000)::integer;

ALTER TABLE exercise_core
  DROP COLUMN quality;

ALTER TABLE exercise_running
  DROP COLUMN quality,
  ALTER COLUMN distance_m TYPE INTEGER USING (distance_m::integer),
  ALTER COLUMN moving_distance_m TYPE INTEGER USING (moving_distance_m::integer);
