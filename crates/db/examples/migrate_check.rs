#![allow(clippy::unwrap_used, reason = "example binary, not library code")]

use sqlx::postgres::PgPoolOptions;
use std::path::Path;

#[tokio::main]
async fn main() {
    let migrations_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
    eprintln!("migrations_dir = {migrations_dir}");
    eprintln!("exists = {}", Path::new(migrations_dir).exists());

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect("postgresql://postgres:password@172.17.0.2/postgres")
        .await
        .unwrap();
    eprintln!("connected");

    health_db::run_migrations(&pool).await.unwrap();

    let rows: Vec<(i64,)> = sqlx::query_as("SELECT version FROM _sqlx_migrations ORDER BY version")
        .fetch_all(&pool)
        .await
        .unwrap();
    eprintln!("applied migrations: {}", rows.len());
    for r in &rows {
        eprintln!("  {}", r.0);
    }

    let tables: Vec<(String,)> = sqlx::query_as(
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public' ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    eprintln!("tables in public:");
    for t in &tables {
        eprintln!("  {}", t.0);
    }
}
