# health_tracker

A Rust workspace for tracking workouts (weight, core, running) with a
Vite + React frontend and a Matrix bot for GPX ingest. Backed by Postgres
via SQLx with class-table inheritance for the exercise hierarchy.

See [`DESIGN.md`](DESIGN.md) for the architecture and
[`MIGRATION.md`](MIGRATION.md) for the bootstrap checkpoint + per-phase
TODO list.

## Layout

```
health_tracker/
├── crates/
│   ├── core   # domain types — zero I/O
│   ├── db     # SQLx repository + migrations
│   ├── auth   # OIDC PKCE flow (TODO: Agent A)
│   ├── web    # axum server, binary 1  (TODO: Agent A)
│   └── bot    # matrix-sdk bot, binary 2
├── frontend/             # Vite + React + TS + shadcn/ui + echarts
├── migrations/           # SQLx migrations (Postgres-only)
├── config/default.toml   # checked-in config defaults (no secrets)
└── .gitea/workflows/     # CI (test) + release pipelines
```

## Quickstart

### Prerequisites

- Rust 1.85+ (workspace uses `edition = "2024"`). The pinned nightly
  toolchain lives in `rust-toolchain.toml`; `rustup` picks it up
  automatically.
- Postgres 13+ (the migrations use `gen_random_uuid()`, `INTERVAL`,
  `BYTEA`, `TIMESTAMPTZ`).
- Docker — unit tests for `crates/db` run against a Postgres testcontainer
  via `#[sqlx::test]`. SQLite is intentionally not supported; see
  `MIGRATION.md` §"Test strategy decision".
- Node 20+ and npm for the frontend.

### Build

```sh
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`cargo test --workspace` spins up a transient Postgres container per
`#[sqlx::test]`; Docker must be running locally.

### Frontend

```sh
cd frontend
npm install
npm run dev    # Vite dev server, proxies /api -> http://localhost:3000
npm run build  # type-check + production build into frontend/dist
```

In production the axum server (`crates/web`) serves `frontend/dist` as
static files with an SPA fallback; the dev server proxies `/api` to
`localhost:3000`.

### Bot

```sh
cargo run -p health-bot
```

### Configuration

Both `crates/web` and `crates/bot` load layered configuration via the
[`config`](https://docs.rs/config) crate:

1. `config/default.toml` (checked in, no secrets).
2. Environment variables prefixed `HEALTH_` (use `__` for nesting).
   These override the defaults and are the channel for secrets.

For the bot, the minimum set is:

| Variable                       | Purpose                        |
|--------------------------------|--------------------------------|
| `HEALTH_MATRIX__HOMESERVER`    | Matrix homeserver URL          |
| `HEALTH_MATRIX__USER_ID`       | Matrix user id (required)      |
| `HEALTH_MATRIX__PASSWORD`      | Matrix password (required)     |
| `HEALTH_MATRIX__SESSION_FILE`  | Where to cache `session.toml`  |
| `HEALTH_API__BASE_URL`         | Web API base URL               |
| `HEALTH_API__TOKEN`            | Bearer token for `POST /api/runs/gpx` |

The `session.toml` file persisted by the bot is git-ignored.

### Offline SQLx query cache (item 5.38)

Once the `web` crate switches from runtime `query_as` to the `query!`
macros, generate the offline cache with:

```sh
cargo sqlx prepare --workspace
```

The resulting `.sqlx/` directory is committed so `cargo check` works
without a live database (`DATABASE_URL`) in CI. Until then the codebase
ships with no offline cache and relies on `#[sqlx::test]` to validate
queries at test time.

## Development

`mise` tasks are wired up (see `mise.toml`):

| Task            | Description                              |
|-----------------|------------------------------------------|
| `mise run build`     | Build debug binaries                |
| `mise run test`      | `cargo test --all-features`         |
| `mise run lint`      | `cargo clippy --all-targets -- -D warnings` |
| `mise run fmt-check` | Verify formatting                     |
| `mise run check`     | Run `fmt-check`, `lint`, `test`      |
| `mise run run-web`   | Run the web server                   |
| `mise run run-bot`   | Run the bot                          |
| `mise run changelog` | Preview the next release notes       |
| `mise run tag`       | Bump version, tag, push to release   |