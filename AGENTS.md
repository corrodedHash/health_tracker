# Agent rules

## Package manager

- The frontend uses **pnpm** (not npm). The version is pinned via `package.json`'s
  `packageManager` field and `mise.toml`. Always use `pnpm` for frontend commands.
- Most common tasks are available as `mise run <task>` — see `mise.toml` or
  `mise tasks` for the full list. Prefer `mise run` over raw commands when
  applicable.

## Migrations

- **Never modify an existing migration** once it has been committed to `main`.
  Always add a new numbered migration (e.g. `0009_<description>.up.sql` /
  `0009_<description>.down.sql`).

## SQLx type safety

Use the compile-time `query!` / `query_as!` macros everywhere (never
`sqlx::query` / `sqlx::query_as::<_, T>`).

Workflow after any schema change:

1. Start a Postgres instance with the schema applied (e.g. via `cargo run` or
   the CI Postgres service).
2. `cargo sqlx prepare --workspace` — generates `sqlx-data.json` at the
   workspace root.
3. Commit the updated `sqlx-data.json` alongside the new migration.
4. From that point `query!` macros verify against the cached schema at compile
   time with no live database needed.

## Prefer mise tasks

Run `mise tasks` to see all available tasks. Using `mise run <task>` ensures
the correct tool versions and environment are set up automatically.

## Verification via hk

Prefer `hk check` to verify changes — it runs cargo fmt/clippy/test,
frontend lint/typecheck/build, and migration checks. Pre-commit hooks also
run most of these automatically (migration checks, cargo fmt, cargo clippy,
frontend lint, frontend typecheck). If `hk check` passes, the code is
almost certainly correct.
