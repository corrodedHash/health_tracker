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

A parent `exercise_sessions` table holds cross-cutting fields. Each exercise type
gets its own child table whose PK is also a FK (with `ON DELETE CASCADE`) back to
the parent. This gives real column types and constraints while keeping a single FK
target for heartrate data.

```sql
-- Parent — one row per workout, any type
CREATE TABLE exercise_sessions (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL,
    kind        TEXT NOT NULL,       -- 'weight' | 'core' | 'running' | custom
    started_at  TIMESTAMPTZ NOT NULL,
    duration    INTERVAL NOT NULL,
    notes       TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Weight exercises
CREATE TABLE weight_exercises (
    session_id    UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    weight_kg     DOUBLE PRECISION NOT NULL,
    sets          INT NOT NULL,
    reps          INT NOT NULL,
    quality       INT                -- 1–10 subjective feel
);

-- Running sessions with optional GPX blob
CREATE TABLE running_sessions (
    session_id   UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    distance_m   DOUBLE PRECISION NOT NULL,
    gpx_data     BYTEA              -- raw GPX file, stored as blob
);

-- Core exercises (plank, dead bug, etc.)
CREATE TABLE core_exercises (
    session_id    UUID PRIMARY KEY REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    exercise_name TEXT NOT NULL,
    duration      INTERVAL NOT NULL,
    quality       INT
);

-- Time-series data — single FK to parent works for ALL types
CREATE TABLE heartrate_samples (
    session_id   UUID NOT NULL REFERENCES exercise_sessions(id) ON DELETE CASCADE,
    offset_secs  INTEGER NOT NULL,   -- seconds from session start
    bpm          SMALLINT NOT NULL,
    PRIMARY KEY (session_id, offset_secs)
);
```

### Unknown / Custom Exercises

If an exercise type doesn't have a child table yet, insert only into
`exercise_sessions` with a descriptive `kind` value and put details in `notes`.
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

1. Add a migration creating a new child table (FK → `exercise_sessions`).
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
                   real Postgres        ──▶  Testcontainer (CI)

core crate ──────▶  pure fn calls       ──▶  (no I/O at all)
```

### Layer-by-Layer

**`core` crate** — pure unit tests, no I/O. Domain validation (duration must be
positive, weight > 0, etc.), enum exhaustiveness, serde round-trips. Fastest
test tier, run on every `cargo test`.

**`db` crate** — two tiers:
| Tier | Target | Command | Speed |
|---|---|---|---|
| Unit | SQLite (in-memory) | `cargo test` | ~10ms/test |
| Integration | Postgres (Testcontainers) | `cargo test --test '*'` | ~5s spin-up |

SQLite covers query parsing and row mapping for free. The `#[sqlx::test]`
macro creates a fresh DB per test. CI runs the Postgres tier; local dev only
needs SQLite unless you're working on PG-specific features.

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
      ╱ Integration ╲       Axum router + mock repo + real SQLite — `cargo test`
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
| `cargo test` | All tier-1 (core, db SQLite, web mocked, auth mocked) | Nothing |
| `cargo test --test '*'` | Tier-2 (db Postgres integration) | Testcontainers (Docker) |
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
| SQLx compile-time checks need a running DB | CI starts a Postgres Testcontainer; `sqlx prepare` generates offline cache for dev |
| Matrix SDK dependency bloat in bot | Bot is a separate binary — web server never compiles it |
| JSONB would lose type safety | Not using JSONB — concrete columns per type via CTI pattern |
| Time zone bugs | All `started_at` stored as `TIMESTAMPTZ` (UTC); conversion happens only in the frontend |
| Orphan records on partial inserts | Repositories always wrap parent + child insert in a single SQLx transaction |
| config crate secrets management | Use environment variable layers over checked-in defaults |

## Implementation Order

1. `core` — domain types, `ExerciseKind` enum with `Weight`, `Core`, `Running` variants
2. `db` — migrations, repository traits + SQLx implementations, unit tests with SQLite
3. `auth` — OIDC token validation, session management
4. `web` — basic CRUD endpoints behind OIDC, static file serving
5. `frontend` — login, session list, exercise form (shadcn + echarts)
6. `bot` — Matrix listener, GPX → API bridge
7. Heartrate ingest CLI for at least one watch export format
8. Polish: charts, filters, GPX track visualization on map
