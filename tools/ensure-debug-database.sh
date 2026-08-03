#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATIONS_DIR="${SCRIPT_DIR}/../migrations"

if ! command -v pg_isready &>/dev/null; then
  echo "pg_isready not found — skipping Postgres health check."
  exit 0
fi
if ! pg_isready -h localhost -p 5432 -q 2>/dev/null; then
  echo "No PostgreSQL found on localhost:5432, starting temporary container..."
  docker run --rm -d --name ht-postgres-ensure \
    -e POSTGRES_USER=health \
    -e POSTGRES_PASSWORD=health \
    -e POSTGRES_DB=health \
    -p 5432:5432 postgres:16
  echo "Waiting for PostgreSQL to be ready..."
  for i in $(seq 1 30); do
    if pg_isready -h localhost -p 5432 -q 2>/dev/null; then
      echo "PostgreSQL is ready."
      break
    fi
    sleep 1
  done
else
  echo "PostgreSQL already running on localhost:5432."
fi

if ! command -v sqlx &>/dev/null; then
  echo "sqlx CLI not found — skipping migration (install via 'cargo install sqlx-cli')."
  exit 0
fi

# Apply any pending migrations so the database is immediately usable. This is
# idempotent: `sqlx migrate run` only applies what is not yet recorded.
echo "Applying pending migrations..."
DATABASE_URL="postgres://health:health@localhost:5432/health" \
  sqlx migrate run --source "${MIGRATIONS_DIR}"
echo "Migrations up to date."
