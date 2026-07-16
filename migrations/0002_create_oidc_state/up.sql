-- oidc_state: ephemeral PKCE/nonce state for in-flight OIDC logins.
-- Ported from
-- workout_tracker/migrations/2025-12-07-202324-0000_add_oidc_table/up.sql
-- with the table renamed `oidc` -> `oidc_state` to avoid confusing the
-- transient login state with the OIDC *provider*.
CREATE TABLE oidc_state (
    csrf          VARCHAR(255) PRIMARY KEY,
    code_verifier VARCHAR(255) NOT NULL,
    nonce         VARCHAR(255) NOT NULL,
    resume_token  VARCHAR(36),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE oidc_state IS 'Ephemeral information for OIDC logins';
COMMENT ON COLUMN oidc_state.csrf IS 'PKCE CSRF token (sent to IdP and stored here)';
COMMENT ON COLUMN oidc_state.code_verifier IS 'PKCE code verifier kept server-side';
COMMENT ON COLUMN oidc_state.nonce IS 'OIDC nonce used to bind ID token to login';
COMMENT ON COLUMN oidc_state.resume_token IS 'Opaque token echoed client-side to resume the original request after callback';
COMMENT ON COLUMN oidc_state.created_at IS 'Row creation timestamp (UTC); used to expire stale logins';