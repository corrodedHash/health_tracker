//! Migration runner. Loads SQL files from the workspace `migrations/`
//! directory (see [`MIGRATIONS_DIR`]) and applies them to a Postgres
//! pool. Used by the `web` crate at startup.

use std::path::Path;

use sqlx::PgPool;
use sqlx::migrate::Migrator;

use crate::error::DbError;

/// Location of the migration directory, baked in at compile time as
/// `<this crate>/../../migrations` so the binary locates it regardless
/// of its current working directory at runtime.
pub const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");

/// Apply all pending migrations to `pool`. Idempotent: re-running on
/// an up-to-date database is a no-op.
///
/// # Errors
/// [`DbError::Invalid`] if the migration directory cannot be resolved
/// or read, [`DbError::Sqlx`] for any failure applying a migration.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    Migrator::new(Path::new(MIGRATIONS_DIR))
        .await
        .map_err(|e| DbError::Invalid(format!("locate migrations dir: {e}")))?
        .run(pool)
        .await
        .map_err(|e| DbError::Invalid(format!("apply migrations: {e}")))?;
    Ok(())
}
