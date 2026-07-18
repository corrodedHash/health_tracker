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

## Releases (release-please)

Releases are fully automated via [release-please](https://github.com/googleapis/release-please).

**How to cut a release:**
1. Merge conventional commits (`feat:`, `fix:`, etc.) to `main`
2. release-please creates/updates a **Release PR** (version bump + changelog)
3. Merge the Release PR → release-please tags the release and creates a GitHub
   Release; CI builds binaries and uploads them as release assets

**Config files:**
- `release-please-config.json` — release-please configuration
- `.release-please-manifest.json` — current version manifest
- `.github/workflows/release-please.yml` — the CI workflow

## Verification via hk

**Always use `hk check` to verify any code change.** It only checks the
files you modified (no full-project rebuild), and runs cargo fmt/clippy/test,
frontend lint/typecheck/build, and migration checks. Pre-commit hooks also
run most of these automatically (migration checks, cargo fmt, cargo clippy,
frontend lint, frontend typecheck). If `hk check` passes, the code is
correct. Do not look for or run individual mise tasks — `hk check` is the
single authoritative command.
