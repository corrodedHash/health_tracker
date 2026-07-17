//! `health-db` — `SQLx` repository implementations + migrations for the
//! `health_tracker` workspace.
//!
//! See `MIGRATION.md` and `DESIGN.md` for the migration plan and schema.
//!
//! ## Layout
//!
//! - [`error`] — [`DbError`] (`thiserror`-based).
//! - [`traits`] — the eight repository traits (the mock boundary used
//!   by `web` / `bot` per the design's testability sketch).
//! - [`repo`] — [`repo::SqlxRepository`], the concrete Postgres impl.
//! - [`migrate`] — [`migrate::run_migrations`] for startup.
//!
//! ## Compile-time query checking
//!
//! All queries use the **compile-time** `query!` / `query_as!` macros.
//! The offline cache in `sqlx-data.json` (workspace root) is generated
//! via `cargo sqlx prepare --workspace` against a live Postgres with
//! all migrations applied, then committed to the repository so every
//! build is type-safe without a live database.

#![allow(clippy::missing_docs_in_private_items)]

pub mod error;
pub mod migrate;
pub mod repo;
pub mod traits;

pub use error::DbError;
pub use migrate::{MIGRATIONS_DIR, run_migrations};
pub use repo::SqlxRepository;
pub use traits::{
    ApiTokenRepository, CoreRepository, HeartrateRepository, OidcStateRepository,
    RunningRepository, SessionsRepository, UsersRepository, WeightRepository,
};
