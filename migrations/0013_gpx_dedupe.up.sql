-- gpx_sha256: content-hash of the raw GPX blob, used to reject re-uploads
-- of already-stored GPX files (server-side idempotency). Mirrors the
-- api_tokens.token_hash CHAR(64) UNIQUE pattern (0008_create_api_tokens).
--
-- NULLs never conflict (Postgres NULLS DISTINCT), so GPX-less runs are
-- unaffected. Legacy rows keep gpx_sha256 = NULL; the repository's dedup
-- pre-check falls back to byte comparison for them.
ALTER TABLE exercise_running ADD COLUMN gpx_sha256 CHAR(64) NULL;
CREATE UNIQUE INDEX exercise_running_gpx_sha256_key
  ON exercise_running (gpx_sha256);

COMMENT ON COLUMN exercise_running.gpx_sha256 IS 'SHA-256 hex digest (64 chars) of gpx_data, NULL when the run has no GPX blob; unique so concurrent duplicate uploads are rejected';
