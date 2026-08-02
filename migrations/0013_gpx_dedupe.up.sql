-- gpx_sha256: content-hash of the raw GPX blob, used to reject re-uploads
-- of already-stored GPX files (server-side idempotency). Mirrors the
-- api_tokens.token_hash CHAR(64) UNIQUE pattern (0008_create_api_tokens).
--
-- user_id is denormalized from exercises so the unique dedup index is
-- per-user: two different accounts may store the same file. Rows created
-- before this migration get user_id backfilled. Rows with NULL user_id /
-- gpx_sha256 (GPX-less runs, or legacy-style inserts via
-- RunningRepository::insert) never conflict (Postgres NULLS DISTINCT).
ALTER TABLE exercise_running ADD COLUMN gpx_sha256 CHAR(64) NULL;
ALTER TABLE exercise_running ADD COLUMN user_id UUID NULL REFERENCES users(id) ON DELETE CASCADE;

UPDATE exercise_running er
   SET user_id = e.user_id
  FROM exercises e
 WHERE er.session_id = e.id;

CREATE UNIQUE INDEX exercise_running_user_id_gpx_sha256_key
  ON exercise_running (user_id, gpx_sha256);

COMMENT ON COLUMN exercise_running.gpx_sha256 IS 'SHA-256 hex digest (64 chars) of gpx_data, NULL when the run has no GPX blob; unique per user so concurrent duplicate uploads are rejected';
COMMENT ON COLUMN exercise_running.user_id IS 'Denormalized owner id (mirrors exercises.user_id) so the gpx_sha256 dedup index is per-user; NULL only for rows inserted outside the GPX upload path';
