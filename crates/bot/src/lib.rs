//! `health-bot` — Matrix listener that receives GPX files and POSTs them
//! to the `health_tracker` web API.
//!
//! See `DESIGN.md` §"Matrix Bot" and `MIGRATION.md` Phase 4.

pub mod api_client;
pub mod gpx;
pub mod matrix_auth;
pub mod matrix_client;