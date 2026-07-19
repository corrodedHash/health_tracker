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

- `core` and `db` have zero web/matrix dependencies.
- `bot` does not depend on `web`.
- Both `web` and `bot` depend on `core` + `db`.

## Database Schema

### Class Table Inheritance

A parent `exercises` table holds cross-cutting fields. Each exercise type
gets its own child table whose PK is also a FK (with `ON DELETE CASCADE`) back to
the parent. This gives real column types and constraints while keeping a single FK
target for heartrate data.

### Migration conventions

- **Migrations are Postgres-only** (`UUID`, `TIMESTAMPTZ`,
  `INTERVAL`, `BYTEA`, `gen_random_uuid()`, `ON CONFLICT`). PG >= 13
  is required (no `pgcrypto` extension).
- **Cross-table `kind` validation is enforced in the repository layer,
  not in DB `CHECK`.** Postgres `CHECK` cannot reference other tables.
  Insert methods wrap parent + child row in a single transaction and refuse the
  insert when the discriminator doesn't match.
- **Down migrations are idempotent** (`DROP ... IF EXISTS`) and
  ordered so rollback is the exact reverse of apply.

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

### Endpoints

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
for domain types and inserts directly, or calls the HTTP API like any
other client. This keeps it fully decoupled.

## Extensibility

### Adding a new exercise type

1. Add a migration creating a new child table (FK → `exercises`).
2. Add a variant to the `ExerciseKind` enum in `core`.
3. Done — heartrate, auth, and cross-cutting queries work without changes.

### Smartwatch / Heartrate ingestion

The `heartrate_samples` table already handles any exercise type. Each watch
brand (Fitbit, Garmin, Apple Health) needs a small scraper/CLI that converts
its export format into API calls. These can live as standalone scripts and
don't need to be part of the server binary.

## Testability

```
         ╱─────╲
        ╱  E2E  ╲          Playwright (browser → real server → PG) — CI only
       ╱─────────╲
      ╱ Integration ╲       Axum router + mock repo + real Postgres (testcontainer)
     ╱───────────────╲
    ╱  Unit (domain)   ╲    Core types, validation, serde
   ╱─────────────────────╲
  ╱  SQLx compile-check    ╲   Every build catches bad queries
 ╱───────────────────────────╲
```

## Key Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Postgres testcontainer is required for `cargo test` | Local dev must have Docker running; CI starts the testcontainer as part of the job |
| Matrix SDK dependency bloat in bot | Bot is a separate binary — web server never compiles it |
| JSONB would lose type safety | Not using JSONB — concrete columns per type via CTI pattern |
| Time zone bugs | All `started_at` stored as `TIMESTAMPTZ` (UTC); conversion happens only in the frontend |
| Orphan records on partial inserts | Repositories always wrap parent + child insert in a single SQLx transaction |
| config crate secrets management | Use environment variable layers over checked-in defaults |
