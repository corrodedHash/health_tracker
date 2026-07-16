# MIGRATION.md — health_tracker bootstrap checkpoint

> This file is the restart point. Read top-to-bottom before continuing work.
> Pair it with `DESIGN.md` (the spec) — together they cover what, why, and
> how-far-we've-gotten.

## Status at checkpoint

- `cargo check --workspace` — **passes**
- `cargo test -p health-core` — **8/8 unit tests pass**
- Workspace skeleton with five crates (`core`, `db`, `auth`, `web`, `bot`)
  is wired up and compiling. `core` is implemented; the others are stubs.
- No migrations yet. No frontend yet. No CI yet.

Continue from the ["TODO"](#todo) section below.

---

## 1. What we are building (one-paragraph recap)

A Rust workspace (`crates/{core,db,auth,web,bot}`) with a Vite+React+TS
frontend (`frontend/`). Postgres + SQLx with class-table inheritance
(parent `exercise_sessions` + per-type child tables + `heartrate_samples`
+ `users` + `api_tokens`). `web` (axum, OIDC-protected) is binary #1.
`bot` (matrix-sdk, listens for GPX files, POSTs to `web` with a bearer
token) is binary #2. Full spec lives in `DESIGN.md`.

## 2. Assessment of the two reference repos

### 2.1 `workout_tracker` (actix-web + Diesel + OIDC + Vite/React)

Location: `/home/lukas/documents/coding/rust/workout_tracker`

| Aspect | State | Reuse |
|---|---|---|
| Web framework | actix-web 4.11 + actix-session/identity | **Adapt** to axum — patterns but not code |
| DB | Diesel 2.2 (sync) + r2d2 + `web::block` | **Replace** with SQLx |
| Auth | OIDC PKCE via `openidconnect 4.0.1`, PocketID IdP | **HIGHLY REUSABLE** — see `src/oidc.rs:50-253` |
| Domain | One flat `workouts` table, no types/HR/GPX | **Discard** — design uses CTI |
| Frontend | Vite 7 + React 19 + TS + TanStack Query 5 + axios + dayjs + PWA | **HIGHLY REUSABLE** as skeleton |
| Frontend UI lib | MUI v7 + MUI X DataGrid | **Replace** with shadcn/ui per design |
| Charting | None | **Add** echarts |
| DI | Closed `Database::{Real,Mock}` enum; empty `db/traits.rs` | **Replace** with trait objects |
| Errors | `thiserror` in OIDC; hand-rolled `WorkoutError`/`DbError` elsewhere; no `anyhow` | **Restructure** per design: `thiserror` in core/db, `anyhow` in web/bot |
| Config | Hardcoded `./config.toml`, plaintext secret committed | **Replace** with env-over-defaults |
| Tests | 3 actix-web tests, stale/broken (assert SEE_OTHER against handlers now returning 200) | **Don't reuse** — write fresh |

**Reusable source files (verbatim paths, port next):**
- `src/oidc.rs` (253 lines): `OidcSetupError` / `OidcInitError` /
  `OidcCallbackError`, `setup_oidc_client`, `oidc_init` (PKCE challenge +
  DB state insert), `oidc_callback` (exchange code, validate ID token,
  access-token-hash check, `Identity::login`). The PKCE state row shape
  is preserved 1:1 as `health_core::OidcState` / `NewOidcState`.
- `migrations/2025-12-07-202324-0000_add_oidc_table/up.sql` — becomes the
  `oidc_state` migration in our workspace.
- `frontend/` (Vite config w/ dev proxy at `vite.config.ts:44-50`, React
  Query setup at `frontend/src/app.tsx:19-31`, axios resume-token dance
  at `frontend/src/app.tsx:43-110`).
- `.gitea/workflows/{release,test}.yaml` — CI pattern.
- Strict clippy config (`Cargo.toml:42-49`) — already lifted into
  workspace `[workspace.lints]`.

**Adaptation needed when porting `oidc.rs`:**
1. actix-identity → bespoke axum middleware stamping `UserId` into request
   extensions (design's testability sketch uses exactly this).
2. `actix_web::web::Data` → `axum::State`.
3. `web::Form<WorkoutData>` is irrelevant — we use JSON DTOs.
4. The `panic!` at `oidc.rs:223` ("Server did not return an ID token")
   must become a proper error.
5. Need a `users` table so the verified `sub` claim maps to a real user
   row (current repo just stamps literal `"user"`).

### 2.2 `matrix-running` (matrix-sdk bot + gpx + tokio-postgres)

Location: `/home/lukas/documents/coding/rust/matrix-running`

| Aspect | State | Reuse |
|---|---|---|
| Matrix SDK | `matrix-sdk 0.11`, `default-features=false`, `native-tls` | **HIGHLY REUSABLE** as bot skeleton |
| Auth | Matrix password login, session restore to `session.toml` | **Reuse** for matrix auth (separate from OIDC) |
| GPX parsing | `gpx 0.10` + `haversine-rs 0.3`, moving/total pace | **HIGHLY REUSABLE** — logic-port verbatim |
| Storage | **Direct tokio-postgres INSERT** into a 3-column `running` table | **Replace** with HTTP POST to web API + bearer token |
| GPX on disk | Writes to `data/routes/<date>.gpx` | **Replace** with BYTEA in `running_sessions.gpx_data` |
| Heartrate | Reads JSON files with `maxHR`/`averageHR`/`activeSeconds`, sorts by chat command | **Seed** for scrapers, but design stores time-series samples |
| DB trait | `#[async_trait] Db` with `RealDb`/`NoopDb` | **Pattern-reuse**, but trait lives in `db` crate and uses SQLx |
| Tests | Inline units on `run.gpx`/`heartrate.json` fixtures; one `#[ignore]`d PG test | **Fixtures reusable**, tests not |
| Config | Single `Config::from_path` reading `[login]` + `[db]`; re-serializes login to temp file (wart at `main.rs:73-75`) | **Replace** with env-layered config |
| Runtime | `#[tokio::main(flavor = "current_thread")]` despite `rt-multi-thread` feature | **Fix** — drop to single-thread or remove the feature |
| Known problems | `heartbeat_manager.rs:77` TODO: path-traversal risk; disk-vs-code `heartbeat`/`heartrate` naming drift | **Don't carry over** |

**Reusable source files (verbatim paths, port next):**
- `src/routes.rs` (`get_track_moving_distance_time`,
  `get_segment_distance_time`) — port verbatim into `crates/bot/src/gpx.rs`.
- `src/heartrate.rs` (`HeartRateData`/`UnitV` with camelCase
  `serde(rename)`) — seed for `crates/scrapers` (not yet planned in
  workspace members).
- `src/auth.rs` (`restore_session`/`login`/`get_client`) — port to
  `crates/bot/src/matrix_auth.rs` (matrix session persistence).
- `src/events.rs:29-87` `on_stripped_state_member` (auto-join, refuse
  encrypted rooms, exp-backoff) — port to bot.
- `src/events.rs:94-144` `on_room_message` dispatcher — port; **but**
  replace `Db` context with `ApiClient` (the design's HTTP client trait).
- `src/testdata/run.gpx` (~105KB OsmAnd trail run), `heartrate.json` —
  **copy verbatim** to `crates/bot/tests/fixtures/` (or a workspace
  `fixtures/` dir).
- `Cargo.toml` release profile (`lto="fat"`, `opt-level="s"`,
  `codegen-units=1`) — already lifted into workspace `[profile.release]`.

**Adaptation needed when porting the bot:**
1. Wrap `matrix-sdk` in a `MatrixClient` trait (design's mock boundary).
2. Wrap the outgoing HTTP API call in an `ApiClient` trait; mock with
   `wiremock` in tests.
3. Replace `db.insert_row(date, dist, dur)` with
   `api_client.post_run_gpx(gpx_bytes, kind=Running, started_at=date,
   distance_m, duration)`. The bot is no longer DB-aware.
4. Heartbeat-sorting flow (write-to-disk + chat-command classification)
   is **not** in the design — drop it. HR ingest goes through scrapers in
   a future crate.
5. Use a `MatrixSession` from `session.toml` as before, but load via the
   workspace's shared config layer.

## 3. Schema we will create (mirrors DESIGN.md §"Database Schema")

Migration files go under `migrations/<timestamp>_<name>/{up,down}.sql`.
SQLx migration runner loads them at startup from `MIGRATIONS_DIR` (see
`crates/db/src/lib.rs`).

Order (+ names we will use):

1. `0001_create_users` — `users(id UUID PK, external_id TEXT UNIQUE,
   display_name TEXT, created_at TIMESTAMPTZ default NOW())`.
   `external_id` is the OIDC `sub` claim. Replaces the literal `"user"`
   identity in `workout_tracker`.
2. `0002_create_oidc_state` — port of
   `workout_tracker/migrations/2025-12-07-202324-0000_add_oidc_table/up.sql`:
   `oidc_state(csrf VARCHAR(255) PK, code_verifier VARCHAR(255),
   nonce VARCHAR(255), resume_token VARCHAR(36) NULL, created_at
   TIMESTAMPTZ default NOW())`. Rename from `oidc` → `oidc_state` to
   avoid confusion with the OIDC *provider*.
3. `0003_create_exercise_sessions` — the parent table verbatim from
   `DESIGN.md`:
   ```sql
   CREATE TABLE exercise_sessions (
       id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
       user_id     UUID NOT NULL REFERENCES users(id),
       kind        TEXT NOT NULL CHECK (kind IN ('weight','core','running')),
       started_at  TIMESTAMPTZ NOT NULL,
       duration    INTERVAL NOT NULL,
       notes       TEXT,
       created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   ```
4. `0004_create_weight_exercises` — child table, PK+FK to parent with
   `ON DELETE CASCADE`, per design.
5. `0005_create_running_sessions` — includes `gpx_data BYTEA` (the
   design's BLOB storage decision).
6. `0006_create_core_exercises` — child table.
7. `0007_create_heartrate_samples` — composite PK
   `(session_id, offset_secs)` per design; FK to parent with CASCADE.
8. `0008_create_api_tokens` — `api_tokens(id UUID PK, user_id UUID FK,
   label TEXT, token_hash CHAR(64) UNIQUE, created_at TIMESTAMPTZ,
   last_used_at TIMESTAMPTZ NULL)`. SHA-256 hex of the cleartext.

**Repository traits** (in `crates/db/src/`):

- `SessionsRepository` — `list`, `get`, `insert(parent + child in one
  tx)`, `delete`, `filter(kind, from, to)`.
- `WeightRepository`, `CoreRepository`, `RunningRepository` — child-row
  inserts plus `get_by_session`.
- `HeartrateRepository` — bulk insert (`INSERT ... ON CONFLICT DO
  NOTHING` to make replays idempotent), `list_for_session`.
- `UsersRepository` — `upsert_by_external_id`, `get`.
- `OidcStateRepository` — port of `workout_tracker`'s `insert_oidc` /
  `fetch_oidc` / `delete_oidc`.
- `ApiTokenRepository` — `issue(user_id, label) -> NewApiToken`,
  `verify(token) -> Option<UserId>` (updates `last_used_at`), `revoke`.

## 4. What is done in this checkpoint

```
health_tracker/
├── Cargo.toml                     ✅ workspace root, [workspace.dependencies]
│                                      + [workspace.lints] (pedantic/nursery/cargo)
│                                      + [profile.release] (lto=fat/s/codegen=1)
├── DESIGN.md                       (pre-existing)
├── MIGRATION.md                    ✅ this file
├── crates/
│   ├── core/                       ✅ COMPLETE
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs              ExerciseKind, ExerciseSession, WeightSession,
│   │       │                       CoreSession, RunningSession, HeartrateSample,
│   │       │                       User, ApiToken, NewApiToken, OidcState,
│   │       │                       NewOidcState, ValidationError, validation
│   │       │                       impls + 8 passing tests
│   │       └── duration_ext.rs     time::Duration → std::time::Duration helpers
│   ├── db/                         🚧 STUB: only Cargo.toml + 5-line lib.rs
│   │   ├── Cargo.toml             (sqlx + sqlite + postgres features wired)
│   │   └── src/lib.rs             MIGRATIONS_DIR constant only
│   ├── auth/                       🚧 STUB: only Cargo.toml + doc-comment lib.rs
│   ├── web/                        🚧 STUB: only Cargo.toml + 4-line main.rs
│   └── bot/                        🚧 STUB: only Cargo.toml + 4-line main.rs
├── migrations/                     ❌ empty dir (no SQL yet)
├── frontend/                       ❌ empty dir (no Vite project yet)
└── .gitea/workflows/               ❌ empty dir
```

The decisions baked in so far:

- **Workspace names**: `health-core`, `health-db`, `health-auth`,
  `health-web`, `health-bot`. (`DESIGN.md` calls them `core`/`db`/etc;
  the `health-` prefix avoids collisions when publishing and is shorter
  than `health_tracker_*`.)
- **Workspace-level dependency versions** pinned in
  `[workspace.dependencies]`. All crates pull from there with
  `dep.workspace = true`.
- **Strict clippy** at the workspace level (lifted from both repos):
  `pedantic` + `nursery` + `cargo` + `unwrap_used` + `expect_used` as
  warnings. `multiple_crate_versions` allowed (Ruma pulls duplicates).
- **`edition = "2024"`** for all crates; `rust-version = "1.85"`.
- **`core` never depends on tokio/sqlx/axum.** It re-exports `chrono`,
  `time`, `uuid` for downstream crates so they don't all need to declare
  them separately.
- **`OidcState` lives in `core`** (not `auth`) so both `auth` and `db`
  reference one struct. Mirrors `workout_tracker/src/models.rs`.
- **No `Eq` on f64-containing types.** `ValidationError`, `WeightSession`,
  `RunningSession` derive only `PartialEq` — bite that bullet up front to
  avoid churn later.
- **`timetz`/`time` interop helper** in `core::duration_ext` — the bot
  uses `time::Duration` (gpx/haversine) and the rest use
  `std::time::Duration`. Single conversion point.

## 5. <a name="todo"></a>TODO

Roughly priority-ordered. Each item references the design section and
the reference repo to lift from where applicable.

### Phase 1 — db (high priority, unblocks everything else)

- [ ] **5.1** Create migration `0001_create_users/up.sql` & `down.sql`
- [ ] **5.2** Create `0002_create_oidc_state` (port verbatim from
      `workout_tracker/migrations/2025-12-07-202324-0000_add_oidc_table`)
- [ ] **5.3** Create `0003_create_exercise_sessions`
- [ ] **5.4** Create `0004_create_weight_exercises`
- [ ] **5.5** Create `0005_create_running_sessions` (with `gpx_data
      BYTEA`)
- [ ] **5.6** Create `0006_create_core_exercises`
- [ ] **5.7** Create `0007_create_heartrate_samples`
- [ ] **5.8** Create `0008_create_api_tokens`
- [ ] **5.9** In `crates/db/src/`: define traits
      (`SessionsRepository`, `WeightRepository`, `CoreRepository`,
      `RunningRepository`, `HeartrateRepository`, `UsersRepository`,
      `OidcStateRepository`, `ApiTokenRepository`) and a
      `SqlxRepository` impl. Replace `workout_tracker`'s closed
      `Database` enum with trait objects (design §Testability).
- [ ] **5.10** SQLite in-memory unit tests for the repo impls (design
      §Testability, db tier 1). Use `#[sqlx::test]`.
- [ ] **5.11** Optional: Postgres Testcontainers integration tier
      (design §Testability, db tier 2).

### Phase 2 — auth (unblocks web)

- [ ] **5.12** Port `setup_oidc_client` from
      `workout_tracker/src/oidc.rs:65-97` into
      `crates/auth/src/oidc.rs`. Keep the discovery+PKCE flow, swap
      actix types for plain reqwest (already its HTTP client).
- [ ] **5.13** Port `oidc_init` (`workout_tracker/src/oidc.rs:124-154`)
      → `crates/auth/src/flow.rs::start_login`. Returns auth URL +
      `NewOidcState` to be persisted by the caller.
- [ ] **5.14** Port `oidc_callback`
      (`workout_tracker/src/oidc.rs:194-253`) →
      `flow.rs::finish_login`. Takes the code + the fetched
      `OidcState`; returns the verified `sub` claim; **the panic at
      `oidc.rs:223` becomes `OidcCallbackError::MissingIdToken`**.
- [ ] **5.15** Define `AuthProvider` trait per design §Testability,
      put the impl behind it. Mock impl returns canned claims.
- [ ] **5.16** Session token logic: after `finish_login`, web stamps a
      signed session cookie. Keep token validation in `auth`, not
      `web`.

### Phase 3 — web (high priority)

- [ ] **5.17** `crates/web/src/main.rs`: axum server setup, tracing
      init, config loading (env-over-defaults — not hardcoded `./config.toml`),
      run SQLx migrations on startup.
- [ ] **5.18** Routes (design §API):
  - [ ] `GET /api/exercise-sessions?kind=&from=&to=`
  - [ ] `POST /api/exercise-sessions` (JSON, OIDC)
  - [ ] `GET /api/exercise-sessions/:id`
  - [ ] `DELETE /api/exercise-sessions/:id`
  - [ ] `POST /api/exercise-sessions/:id/heartrate`
  - [ ] `POST /api/runs/gpx` (**bearer token**, parses GPX server-side)
  - [ ] `GET /api/runs/:id/gpx` (raw bytes)
  - [ ] `GET /api/tokens` / `POST /api/tokens` (issue bearer tokens
        from the web UI)
- [ ] **5.19** Auth middleware: extract the OIDC session, stamp
      `UserId` into request extensions (design §Testability sketch).
      Provides `test_auth_layer` for tests.
- [ ] **5.20** Bearer-token middleware for bot endpoints.
- [ ] **5.21** Static file serving via `tower-http::ServeDir` for
      `frontend/dist` with SPA fallback (replace
      `workout_tracker/src/main.rs:85`'s actix_files pattern).
- [ ] **5.22** Router-level tests via `tower::ServiceExt::oneshot`
      + `mockall::MockRepository` (design §Testability, web tier).

### Phase 4 — bot (medium priority)

- [ ] **5.23** Port `matrix-running/src/routes.rs` verbatim →
      `crates/bot/src/gpx.rs`. Keep `get_track_moving_distance_time`.
- [ ] **5.24** Port `matrix-running/src/auth.rs` →
      `crates/bot/src/matrix_auth.rs` (session restore from
      `session.toml`).
- [ ] **5.25** Define `MatrixClient` trait
      (`wait_for_gpx_file -> Future<(Vec<u8>, Metadata)>`); real impl
      wraps `matrix-sdk` (port `matrix-running/src/events.rs:217-306`
      `handle_file` as the trait's body).
- [ ] **5.26** Define `ApiClient` trait
      (`post_run_gpx(bytes, started_at, distance_m, duration ->
      Future<Result<Uuid>>)`); real impl uses reqwest + bearer token.
- [ ] **5.27** `crates/bot/src/main.rs`: wire config, build traits,
      run sync loop (port `matrix-running/src/main.rs:65-125` but
      drop the `argh` main-args dance and use
      `figment`/`config`/just-env — TBD which).
- [ ] **5.28** Copy fixtures:
      `cp /home/lukas/documents/coding/rust/matrix-running/src/testdata/*.gpx
       crates/bot/tests/fixtures/` (and the heartrate.json as a
       future-scraper seed).
- [ ] **5.29** Tests: `wiremock` for `ApiClient`, hand-written mock
      for `MatrixClient` (design §Testability, bot tier).

### Phase 5 — frontend (medium priority, mostly lecture-by-example)

- [ ] **5.30** `npm create vite@latest frontend -- --template react-ts`
      or copy `workout_tracker/frontend/*` minus `node_modules`/`dist`.
- [ ] **5.31** Keep: `vite.config.ts` dev proxy (lines 44-50),
      `package.json` (TanStack Query 5, axios, dayjs, PWA plugin),
      `app.tsx`'s resume-token-dance logic (lines 43-110).
- [ ] **5.32** Remove MUI: uninstall `@mui/*` + `@emotion/*` +
      `@mui/x-*`. Add shadcn/ui (`npx shadcn@latest init`). Add
      echarts (`echarts` + `echarts-for-react`).
- [ ] **5.33** Build skeletons: login page, session list (echarts
      weight-over-time), exercise entry form, run-detail map view
      (consume `/api/runs/:id/gpx`).

### Phase 6 — quality + ops (low priority)

- [ ] **5.34** `.gitea/workflows/test.yaml` mirroring
      `workout_tracker/.gitea/workflows/test.yaml` but with the
      workspace + Postgres Testcontainer + `cargo test --workspace`
      + `cargo clippy --all-targets`.
- [ ] **5.35** `.gitea/workflows/release.yaml` mirroring
      `workout_tracker/.gitea/workflows/release.yaml` (git-cliff,
      build matrix, release).
- [ ] **5.36** `README.md` with quickstart (env vars, `sqlx prepare`
      flow for offline query check, dev frontend proxy port).
- [ ] **5.37** `.gitignore` (target/, frontend/node_modules/,
      frontend/dist/, session.toml, config.toml if secrets, `.sqlx/`).
- [ ] **5.38** `.sqlx/` offline query cache committed once queries
      exist (`cargo sqlx prepare --workspace`).

## 6. Open questions (decide before starting the relevant phase)

1. **Config lib:** `config` crate, `figment`, or hand-rolled `toml` +
   `std::env` (as both parent repos do)? DESIGN.md says "environment
   variable layers over checked-in defaults" — `figment` is closest.
2. **Bot feature gate:** keep `crates/bot` always in workspace, or gate
   with a feature flag so `cargo build` doesn't pull `matrix-sdk` by
   default? DESIGN.md offers both options. Recommend: always in
   workspace, accept the matrix-sdk compile cost.
3. **Frontend bundler extras:** keep `vite-plugin-pwa` from the parent
   repo? Probably yes — small cost, useful mobile UX.
4. **Map rendering for GPX:** leaflet vs maplibre-gl. Maplibre is
   lighter without the Google tiles dependency. Decide at phase 5.
5. **Watch scraper crate:** design says `crates/scrapers/` or scripts.
   Suggest adding it as a 6th workspace member only when first scraper
   is actually built; don't spec it now.

## 7. How to verify progress so far

```sh
cd /home/lukas/documents/coding/rust/health_tracker
cargo check --workspace          # should be clean
cargo test -p health-core       # 8 passed as of this checkpoint
cargo clippy --workspace --all-targets -- -D warnings
```

If the first command fails with a toolchain error, ensure the nightly
toolchain is installed (`rustc 1.98.0-nightly 2026-06-17` was used to
produce this checkpoint) — the workspace uses `edition = "2024"` which
needs at least Rust 1.85 stable, but some parent repos required nightly.

## 8. Where to start next session

Begin at **Phase 1 → item 5.1**: write `migrations/0001_create_users/`
then `0003_create_exercise_sessions` etc. Once all 8 migrations exist,
open `crates/db/src/lib.rs` and replace the stub with traits + a
`SqlxRepository` impl. Run `cargo test -p health-db` against SQLite
in-memory to validate row mapping before moving on to `auth`.