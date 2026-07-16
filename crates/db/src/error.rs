//! Database errors for the `health-db` crate.
//!
//! Mirrors the design rule "`thiserror` in `core` / `db`". Callers in
//! `web` / `bot` convert [`DbError`] into `anyhow::Error` at the
//! boundary.

/// All errors returned by the repository layer.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// No row matched the query (e.g. `get` by id).
    #[error("row not found")]
    NotFound,

    /// A write violated a uniqueness constraint or FK.
    #[error("conflict: {0}")]
    Conflict(String),

    /// The caller passed data the DB rejected (CHECK, bad enum, etc.).
    #[error("invalid input: {0}")]
    Invalid(String),

    /// A kind discriminator mismatch — e.g. inserting a `weight_exercises`
    /// row against a parent whose `kind = 'running'`. Enforced in the
    /// repository transaction because Postgres `CHECK` cannot reference
    /// other tables (see `migrations/0004_create_weight_exercises/up.sql`).
    #[error("kind mismatch: parent kind is {parent}, child expects {child}")]
    KindMismatch { parent: String, child: String },

    /// Anything else bubbling up from `SQLx`.
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
