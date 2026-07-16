-- api_tokens: long-lived bearer tokens issued via the web UI for
-- machine clients (e.g. the Matrix bot). Only the SHA-256 hash is
-- stored; the cleartext is shown once at creation time. Per
-- MIGRATION.md §3 item 8 and DESIGN.md §"Authentication". Mirrors
-- health_core::ApiToken / NewApiToken.
CREATE TABLE api_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label        TEXT NOT NULL,
    -- SHA-256 hex of the cleartext: 64 lowercase hex chars.
    token_hash   CHAR(64) NOT NULL UNIQUE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

COMMENT ON TABLE api_tokens IS 'Long-lived bearer tokens for machine clients (Matrix bot, scrapers)';
COMMENT ON COLUMN api_tokens.id IS 'Server-assigned row id (gen_random_uuid)';
COMMENT ON COLUMN api_tokens.user_id IS 'Owning user; FK users.id with ON DELETE CASCADE';
COMMENT ON COLUMN api_tokens.label IS 'Human-friendly client identifier (''matrix-bot'', ''garmin-scraper'')';
COMMENT ON COLUMN api_tokens.token_hash IS 'SHA-256 hex digest (64 chars) of the cleartext bearer token; never store cleartext';
COMMENT ON COLUMN api_tokens.created_at IS 'Issuance timestamp (UTC)';
COMMENT ON COLUMN api_tokens.last_used_at IS 'Timestamp of the most recent verify() success; NULL until first use';

-- Listing a user''s tokens (e.g. GET /api/tokens) and looking up a
-- token by hash on every bot request are the hot paths.
CREATE INDEX api_tokens_user_id_idx ON api_tokens (user_id);