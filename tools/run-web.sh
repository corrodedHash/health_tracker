#!/usr/bin/env bash
set -euo pipefail

./tools/ensure-debug-database.sh

DB_NAME=$(./tools/branch-db-name.sh)
export HEALTH__DATABASE_URL="postgres://health:health@localhost:5432/${DB_NAME}"

if command -v psql &>/dev/null; then
  PGPASSWORD=health psql -U health -h localhost -d postgres \
    -c "CREATE DATABASE ${DB_NAME}" 2>/dev/null || true
fi

cargo run -p health-web -- "$@"
