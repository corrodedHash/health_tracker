//! `health-db` — SQLx repositories for the health_tracker workspace.
//!
//! See `MIGRATION.md` and `DESIGN.md` for the migration plan and schema.
//! This file is a checkpoint stub: repository traits + SQLx impls are
//! filled in next session per the migration TODO.

#![allow(clippy::missing_docs_in_private_items)]

pub const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
