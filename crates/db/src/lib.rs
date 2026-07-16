//! `health-db` — SQLx repository implementations + migrations for the
//! health_tracker workspace.
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
//! The current impl uses **runtime** `sqlx::query_as` calls so the
//! crate compiles with no live `DATABASE_URL` and no `.sqlx/` cache.
//! Phase 6 item 5.38 runs `cargo sqlx prepare --workspace` to produce
//! the offline cache; at that point the `query!` macros can be adopted
//! without changing the private API surface.

#![allow(clippy::missing_docs_in_private_items)]

pub mod error;
pub mod migrate;
pub mod repo;
pub mod traits;

pub use error::DbError;
pub use migrate::{run_migrations, MIGRATIONS_DIR};
pub use repo::SqlxRepository;
pub use traits::{
    ApiTokenRepository, CoreRepository, HeartrateRepository, OidcStateRepository,
    RunningRepository, SessionsRepository, UsersRepository, WeightRepository,
};