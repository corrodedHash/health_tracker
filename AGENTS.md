# Agent rules

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
