//! Migration runner. Embeds the SQL migration files into the binary at
//! compile time via the `sqlx::migrate!` macro and applies them to a
//! Postgres pool. Used by the `web` crate at startup.

use sqlx::PgPool;
use sqlx::migrate::Migrator;

use crate::error::DbError;

/// The migrations, embedded into the binary at compile time.
///
/// The workspace `migrations/` directory is resolved relative to this
/// crate's manifest directory (`crates/db`), so the binary works
/// regardless of its current working directory at runtime.
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// Apply all pending migrations to `pool`. Idempotent: re-running on
/// an up-to-date database is a no-op.
///
/// # Errors
/// [`DbError::Invalid`] for any failure applying a migration.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|e| DbError::Invalid(format!("apply migrations: {e}")))?;
    Ok(())
}
