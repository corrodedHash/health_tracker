DROP INDEX exercise_running_user_id_gpx_sha256_key;
ALTER TABLE exercise_running DROP COLUMN gpx_sha256;
ALTER TABLE exercise_running DROP COLUMN user_id;
