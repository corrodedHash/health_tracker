Done. Here's the handoff for the next agent:
Session handoff — 2026-07-16
Completed
Item	Status
5.10 — Postgres integration tests	✅ Committed (crates/db/tests/repo_integration.rs, 13 tests)
5.12 — port setup_oidc_client	✅ Committed (crates/auth/src/oidc.rs)
5.13 — port oidc_init → start_login	✅ Committed (crates/auth/src/flow.rs)
5.14 — port oidc_callback → finish_login	✅ Committed (panic → LoginFinishError::MissingIdToken)
5.15 — AuthProvider trait + mock impl	✅ Committed (crates/auth/src/lib.rs)
5.16 — session token logic	✅ Committed (crates/auth/src/session.rs, cookie crate)
Migration format fix	✅ Converted dir-based → file-based (sqlx 0.8 requires 0001_create_users.up.sql, not 0001_create_users/up.sql)
Workspace deps	✅ Added serial_test, config, cookie, mockall, http-body-util; removed sqlite from sqlx features
MIGRATION.md	✅ Updated with items 5.12-5.16 checked off, status section updated
Clippy fixes	✅ Fixed duration_suboptimal_units in db tests, expect_used in db examples
Next: Phase 3 — web (items 5.17–5.22)
The web crate is still a stub. The full set:
5.17  — web/main.rs: axum + tracing + config crate + run_migrations
5.18  — routes (sessions CRUD, /runs/gpx, heartrate, tokens)
5.19  — OIDC auth middleware stamping UserId
5.20  — Bearer-token middleware
5.21  — Static file serving + SPA fallback
5.22  — Router-level tests (tower::oneshot + mockall)
Then verify: cargo check --workspace && cargo test -p health-db && cargo test -p health-auth && cargo test -p health-web && cargo clippy --workspace --all-targets -- -D warnings
Key non-negotiables still in effect:
- No code comments; no git commits (unless user asks)
- Postgres only, no SQLite
- edition 2024; dep.workspace = true; strict clippy
- Web: POST /api/runs/gpx parses GPX server-side (gpx + haversine-rs), computes distance_m + duration, stores both + raw bytes; Bearer-token auth
- No query! macro (runtime query_as); PgPool only
- config crate for settings (env-over-defaults), cookie crate for signed session cookies
One gotcha
The pg container IP is 172.17.0.2. The db integration tests hardcode postgresql://postgres:password@172.17.0.2/postgres (overridable via DATABASE_URL). The auth/web crates' router tests (5.22) use mockall + tower::ServiceExt::oneshot and should NOT need a live database (they mock the repo traits).
