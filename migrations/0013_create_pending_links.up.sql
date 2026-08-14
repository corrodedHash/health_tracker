-- pending_links: transient bot-initiated account links awaiting browser
-- confirmation. The Matrix bot creates a row, the user confirms it while
-- logged in via the web UI, and the bot polls it back to receive a freshly
-- issued API token. Mirrors the transient oidc_state pattern: the row only
-- exists while the link is in flight.
CREATE TABLE pending_links (
    code              TEXT PRIMARY KEY,
    user_id           UUID REFERENCES users(id) ON DELETE CASCADE,
    expires_at        TIMESTAMPTZ NOT NULL,
    accepted_at       TIMESTAMPTZ,
    token_returned_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE pending_links IS 'Bot-issued account-link requests awaiting browser confirmation';
COMMENT ON COLUMN pending_links.code IS 'Single-use random code returned to the bot and embedded in the confirmation URL';
COMMENT ON COLUMN pending_links.user_id IS 'User who confirmed the link in the browser; NULL until accepted';
COMMENT ON COLUMN pending_links.expires_at IS 'Links not confirmed by this time are treated as expired (~15 min)';
COMMENT ON COLUMN pending_links.accepted_at IS 'When the user confirmed the link (NULL until then)';
COMMENT ON COLUMN pending_links.token_returned_at IS 'When the bot polled back a freshly issued API token (single use)';
COMMENT ON COLUMN pending_links.created_at IS 'Row creation timestamp (UTC)';

CREATE INDEX pending_links_user_id_idx ON pending_links (user_id);
