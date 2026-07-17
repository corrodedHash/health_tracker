# Health Tracker — Design Document

## Architecture Overview

Rust workspace with modular crates sharing domain types and database access.
A separate frontend project (Vite + React + TypeScript + shadcn + echarts) talks
to the web API. A Matrix bot binary provides an alternative ingest path for GPX
runs.

```
┌──────────────┐     ┌──────────────────┐     ┌───────────────┐
│  React SPA   │────▶│  axum (web)      │────▶│  PostgreSQL   │
│  shadcn/ech. │     │  OIDC-protected  │     │               │
└──────────────┘     └──────────────────┘     │  sessions     │
                                               │  weights      │
┌──────────────┐     ┌──────────────────┐     │  runs         │
│  Matrix bot  │────▶│  API token auth   │────▶│  core         │
│  (separate   │     │  (reqwest → web)  │     │  heartrate    │
│   binary)    │     └──────────────────┘     └───────────────┘
└──────────────┘
```

## Cargo Workspace

```
health_tracker/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── core/                     # domain types, enums, traits — zero framework deps
│   ├── db/                       # SQLx queries, migrations, repository traits
│   ├── auth/                     # OIDC logic, session tokens
│   ├── web/                      # Axum server, routes, middleware (binary 1)
│   └── bot/                      # Matrix bot (binary 2, optional compile)
├── migrations/                   # SQLx migrations (shared)
└── frontend/                     # Vite + React + TS project (separate package)
```

Key rules:
- `core` and `db` have zero web/matrix dependencies.
- `bot` does not depend on `web`.
- Both `web` and `bot` depend on `core` + `db`.
- Error handling: `thiserror` in `core` / `db`, `anyhow` in `web` / `bot`.

## Database Schema

### Class Table Inheritance

A parent `exercises` table holds cross-cutting fields. Each exercise type
gets its own child table whose PK is also a FK (with `ON DELETE CASCADE`) back to
the parent. This gives real column types and constraints while keeping a single FK
target for heartrate data.

The full migration set lives in `migrations/` (eight directories,
`0001`..`0008`, each with `up.sql` and `down.sql`). The sketch below
mirrors those files; authoritative SQL is the migration files
themselves.

```sql
-- 0001_create_users
CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    external_id  TEXT NOT NULL UNIQUE,           -- OIDC `sub` claim
    display_name TEXT,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 0002_create_oidc_state — PKCE/nonce state for in-flight OIDC logins.
-- Renamed from `workout_tracker`'s `oidc` -> `oidc_state` to avoid
-- confusion with the OIDC *provider*.
CREATE TABLE oidc_state (
    csrf           VARCHAR(255) PRIMARY KEY,
    code_verifier  VARCHAR(255) NOT NULL,
    nonce          VARCHAR(255) NOT NULL,
    resume_token   VARCHAR(36),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 0003_create_exercise_sessions — parent CTI table. Adds an explicit
-- FK to users(id) and a CHECK on the kind discriminator mirroring
-- health_core::ExerciseKind.
CREATE TABLE exercises (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id),
    kind        TEXT NOT NULL CHECK (kind IN ('weight','core','running')),
    started_at  TIMESTAMPTZ NOT NULL,
    duration    INTERVAL NOT NULL,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX exercises_user_started_at_idx
    ON exercises (user_id, started_at DESC);
CREATE INDEX exercises_user_kind_started_at_idx
    ON exercises (user_id, kind, started_at DESC);

-- 0004_create_weight_exercises — child PK+FK with ON DELETE CASCADE.
CREATE TABLE exercise_weight (
    session_id    UUID PRIMARY KEY REFERENCES exercises(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    weight_kg     DOUBLE PRECISION NOT NULL,
    sets          INT NOT NULL,
    reps          INT NOT NULL,
    quality       INT                -- 1–10 subjective feel
);

-- 0005_create_running_sessions — GPX blob stored inline as BYTEA.
CREATE TABLE exercise_running (
    session_id   UUID PRIMARY KEY REFERENCES exercises(id) ON DELETE CASCADE,
    distance_m   DOUBLE PRECISION NOT NULL,
    gpx_data     BYTEA              -- raw GPX file, stored as blob
);

-- 0006_create_core_exercises — child PK+FK with ON DELETE CASCADE.
CREATE TABLE exercise_core (
    session_id    UUID PRIMARY KEY REFERENCES exercises(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    duration      INTERVAL NOT NULL,
    quality       INT
);

-- 0007_create_heartrate_samples — time-series, composite PK, any kind.
CREATE TABLE heartrate_samples (
    session_id   UUID NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    offset_secs  INTEGER NOT NULL,   -- seconds from session start
    bpm          SMALLINT NOT NULL,
    PRIMARY KEY (session_id, offset_secs),
    CHECK (bpm > 0),
    CHECK (offset_secs >= 0)
);

-- 0008_create_api_tokens — bot bearer tokens; only SHA-256 hex stored.
CREATE TABLE api_tokens (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label        TEXT NOT NULL,
    token_hash   CHAR(64) NOT NULL UNIQUE,        -- 64 lowercase hex chars
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX api_tokens_user_id_idx ON api_tokens (user_id);
```

### Migration conventions

- **Migrations are Postgres-only** (`UUID`, `TIMESTAMPTZ`,
  `INTERVAL`, `BYTEA`, `gen_random_uuid()`, `ON CONFLICT`). PG >= 13
  is required (no `pgcrypto` extension).
- **Cross-table `kind` validation is enforced in the repository layer,
  not in DB `CHECK`.** Postgres `CHECK` cannot reference other tables.
  Each child `up.sql` documents this; `SqlxRepository::insert_*` wraps
  parent + child row in a single transaction and refuses the insert
  when the discriminator doesn't match. A training-wheels trigger can
  be added later.
- **Down migrations are idempotent** (`DROP ... IF EXISTS`) and
  ordered so rollback is the exact reverse of apply.
- **SQLite in-memory unit tests** (item 5.10) need a parallel
  migration set with portable types — see `MIGRATION.md`'s "SQLite
  test strategy" sidebar for the two options still on the table.

### Unknown / Custom Exercises

If an exercise type doesn't have a child table yet, insert only into
`exercises` with a descriptive `kind` value and put details in `notes`.
Later, when the type gets its own table, rows can be migrated or simply left as-
is. The parent row always exists as a fallback.

### Why BLOBs, not filesystem for GPX

- No orphan files when a session is deleted (DB handles it atomically).
- Single backup command (`pg_dump`) captures everything.
- GPX files are small (100 KB–2 MB) — Postgres handles millions easily.
- The app serves the file directly when the frontend needs to render a map,
  which is negligible overhead.

## API

### Authentication

| Endpoint | Auth | Purpose |
|---|---|---|
| Web UI routes | OIDC (via `openidconnect`) | Browser login, HttpOnly session cookie |
| `POST /api/exercise-sessions` | OIDC session | Insert workout |
| `POST /api/runs/gpx` | Bearer token | Bot uploads GPX |

The bot authenticates with a long-lived API token stored in an `api_tokens` table
linked to a user. Tokens are generated from the web UI.

### Endpoints (sketch)

```
GET    /api/exercise-sessions?kind=&from=&to=
POST   /api/exercise-sessions
GET    /api/exercise-sessions/:id
DELETE /api/exercise-sessions/:id
POST   /api/exercise-sessions/:id/heartrate

POST   /api/runs/gpx               (token auth, parses GPX server-side)
GET    /api/runs/:id/gpx            (serves raw gpx_data for map rendering)
```

## Frontend

Separate Vite + React + TypeScript project in `frontend/`.

- **shadcn/ui** for consistent component design.
- **echarts** for time-series graphs (weight over time, distance per week, heartrate zones).
- Communicates with the Axum API via `fetch` or React Query.
- Development: Vite proxy → Axum on `:3000`. Production: Axum serves the
  built `dist/` as static files.

## Matrix Bot (Separate Binary)

Crate `crates/bot` compiles to its own binary. It uses `matrix-sdk` to listen
for messages in a room. When a user sends a GPX file:

1. Download the file from Matrix
2. Parse with the `gpx` crate
3. Extract distance + duration
4. POST to `web` API with a bearer token

The bot does not import the web server crate — it only depends on `core` + `db`
for domain types and inserts directly, or (better) calls the HTTP API like any
other client. This keeps it fully decoupled.

To enable/disable the bot at build time:
```toml
# workspace Cargo.toml
[workspace]
members = ["crates/core", "crates/db", "crates/auth", "crates/web"]
# crates/bot added only when developing the bot
```

Or use a feature flag.

## Extensibility

### Adding a new exercise type

1. Add a migration creating a new child table (FK → `exercises`).
2. Add a variant to the `ExerciseKind` enum in `core`.
3. Add a repository method in `db` for the new type.
4. Done — heartrate, auth, and cross-cutting queries work without changes.

### Smartwatch / Heartrate ingestion

The `heartrate_samples` table already handles any exercise type. Each watch
brand (Fitbit, Garmin, Apple Health) needs a small scraper/CLI that converts
its export format into `POST /api/.../heartrate` calls. These live in
`crates/scrapers/` or as standalone scripts — they don't need to be part of
the server binary.

## Testability

### Mock Boundaries

```
                    Mock boundary (trait)          Real impl
                    ──────────────────────    ────────────────
web crate ───────▶  Repository trait    ──▶  SQLx + Postgres
                   AuthProvider trait   ──▶  OIDC client
                   GpxParser trait      ──▶  gpx crate

bot crate ───────▶  MatrixClient trait  ──▶  matrix-sdk
                   ApiClient trait      ──▶  reqwest → web

db crate  ───────▶  real SQLx against   ──▶  SQLite (unit)
                   real Postgres        ──▶  Testcontainer (unit + CI)

core crate ──────▶  pure fn calls       ──▶  (no I/O at all)
```

### Layer-by-Layer

**`core` crate** — pure unit tests, no I/O. Domain validation (duration must be
positive, weight > 0, etc.), enum exhaustiveness, serde round-trips. Fastest
test tier, run on every `cargo test`.

**`db` crate** — Postgres-only tier (decision: drop SQLite):
| Target | Command | Speed |
|---|---|---|
| Postgres (Testcontainer) | `cargo test -p health-db` | ~5s spin-up per binary |

SQLite was considered (parallel `migrations_sqlite/` + a `SqliteRepository`)
but rejected — maintaining a second migration set and a second impl of all
eight repository traits to paper over `INTERVAL`/`BYTEA`/`gen_random_uuid`
friction is more pain than it's worth. Instead, `#[sqlx::test]` is wired
against a Postgres testcontainer: each `#[sqlx::test]` test gets a fresh
database inside a transient container. Local dev needs Docker running; CI
starts the testcontainer as part of the test job. The `SqlxRepository`
(`PgPool`) impl is the only impl — no trait duplication, no type-mapping
bugs hiding behind a parallel migration set.

**`web` crate** — Axum makes handlers testable via `tower::ServiceExt::oneshot`:
- Repositories are trait objects → inject `MockRepository` (via `mockall`
  or handwritten impls).
- Auth middleware is replaceable → a test middleware that stamps a fake `UserId`
  into request extensions.
- GPX upload tests use fixture files in `tests/fixtures/`.
- No server process needed — the router is just a `tower::Service`.

Example structure:
```rust
// In tests:
let mut mock_repo = MockRepository::new();
mock_repo.expect_list_sessions()
    .returning(|| Ok(vec![session_fixture()]));

let app = create_router(mock_repo, test_auth_layer());
let response = app
    .oneshot(Request::builder()
        .uri("/api/exercise-sessions")
        .body(Body::empty())
        .unwrap())
    .await;
assert_eq!(response.status(), 200);
```

**`auth` crate** — boundary is the `AuthProvider` trait:
- Token validation and JWKS fetching are behind the trait.
- Test impl returns a canned `Claims` struct.
- OIDC discovery (the `.well-known/openid-configuration` fetch) is the only
  real HTTP call — mock with `wiremock` in a `#[tokio::test]`.

**`bot` crate** — the Matrix SDK is wrapped in a trait:
```rust
#[cfg_attr(test, mockall::automock)]
trait MatrixClient: Send {
    async fn wait_for_gpx_file(&self) -> Result<(Vec<u8>, Metadata)>;
}
```
Real impl uses `matrix-sdk`. Test impl yields fixture GPX bytes.
The HTTP call to the web API is behind `ApiClient` (mocked with `wiremock`).

### What's Hard to Test

| Hard thing | Mitigation |
|---|---|
| OIDC redirect → callback | End-to-end with Playwright in CI only; else mocked |
| Matrix sync loop (long-lived stream) | Extract message handler as pure fn; test that separately |
| Heartrate time-series insert perf | Benchmark with `criterion`, not a pass/fail test |
| Frontend components | Jest/Vitest + MSW (mock service worker) for API calls |
| Concurrent session creation | Property-based testing with `loom` or just tokio `#[test]` |

### Test Pyramid

```
         ╱─────╲
        ╱  E2E  ╲          Playwright (browser → real server → PG) — CI only
       ╱─────────╲
      ╱ Integration ╲       Axum router + mock repo + real Postgres (testcontainer) — `cargo test`
     ╱───────────────╲
    ╱  Unit (domain)   ╲    Core types, validation, serde — `cargo test`
   ╱─────────────────────╲
  ╱  SQLx compile-check    ╲   Every build catches bad queries
 ╱───────────────────────────╲
```

### CI Pipeline

| Step | What | Depends on |
|---|---|---|
| `cargo check` | SQLx query checking + Rust compilation | Postgres via `sqlx prepare` cached |
| `cargo test` | All (core, db Postgres testcontainer, web mocked, auth mocked) | Docker (testcontainer) |
| `cargo test --test '*'` | (Reserved for future heavier integration suites) | — |
| `playwright test` | E2E browser flow | Deployed server stub |
| `cargo clippy --all-targets` | Lint | — |
| `cargo build --features bot` | Bot compiles separately | — |

### Fixtures

Static test data lives in `crates/*/tests/fixtures/`:
- `valid_running.gpx` — minimal GPX with one track
- `empty_gpx.gpx` — edge case
- `malformed_gpx.bin` — not valid XML
- `session_create.json` — valid POST body

## Key Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Postgres testcontainer is required for `cargo test` | Local dev must have Docker running; CI starts the testcontainer as part of the job. `sqlx prepare` generates an offline cache for `cargo check` without a live DB (5.38) |
| Matrix SDK dependency bloat in bot | Bot is a separate binary — web server never compiles it |
| JSONB would lose type safety | Not using JSONB — concrete columns per type via CTI pattern |
| Time zone bugs | All `started_at` stored as `TIMESTAMPTZ` (UTC); conversion happens only in the frontend |
| Orphan records on partial inserts | Repositories always wrap parent + child insert in a single SQLx transaction |
| config crate secrets management | Use environment variable layers over checked-in defaults |

## Resolved Decisions

These pin down selections left open in §6 of `MIGRATION.md`:

- **Config lib → `config` crate.** Both `crates/web` (5.17) and
  `crates/bot` (5.27) load layered config via the `config` crate:
  checked-in `config/default.toml` defaults overridden by environment
  variables (and optionally `config/<env>.toml`). Secrets never land in
  checked-in defaults — they come from env. Add `config` to
  `[workspace.dependencies]` and to web + bot `Cargo.toml`.
- **Bot always in the workspace.** No feature gate. `crates/bot` is a
  workspace member unconditionally; we accept the `matrix-sdk` compile
  cost. (Matches the recommendation in `MIGRATION.md` §6 item 2.)
- **Keep `vite-plugin-pwa`.** The frontend carries the PWA plugin so
  installs work on mobile. Small cost, useful offline UX.
- **Frontend GPX rendering deferred.** The server still parses GPX
  server-side on `POST /api/runs/gpx`, extracts distance + duration at
  ingest time, and stores the raw bytes in `exercise_running.gpx_data`
  plus exposes `GET /api/runs/:id/gpx`. The frontend, however, does
  **not** render a map in the first cut — it shows only the numeric
  distance and pace. Map selection (leaflet vs maplibre-gl) is deferred
  to a later phase. `POST /api/runs/gpx` is **bearer-token** auth
  (machine/bot-facing), not OIDC session.
- **Map lib selection (leaflet vs maplibre-gl) deferred.** Decide when
  the map view is actually built; pin the dep then.


