-- users: minimal user identity. `external_id` is the OIDC `sub` claim,
-- mapping a verified provider subject to a local row. Replaces the
-- literal `"user"` identity used by the parent `workout_tracker`.
CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    external_id  TEXT NOT NULL UNIQUE,
    display_name TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE users IS 'Application users; external_id is the OIDC sub claim';
COMMENT ON COLUMN users.id IS 'Server-assigned identity (gen_random_uuid)';
COMMENT ON COLUMN users.external_id IS 'OIDC provider subject identifier (unique per issuer)';
COMMENT ON COLUMN users.display_name IS 'Optional human-friendly name sourced from provider claims';
COMMENT ON COLUMN users.created_at IS 'Row creation timestamp (UTC)';