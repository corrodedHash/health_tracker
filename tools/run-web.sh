#!/usr/bin/env bash
set -euo pipefail

./tools/ensure-debug-database.sh

DB_NAME=$(./tools/branch-db-name.sh)
export HEALTH__DATABASE_URL="postgres://health:health@localhost:5432/${DB_NAME}"

if command -v psql &>/dev/null; then
  DB_EXISTS=$(PGPASSWORD=health psql -U health -h localhost -d postgres \
    -tAc "SELECT 1 FROM pg_database WHERE datname = '${DB_NAME}'")
  if [ "${DB_EXISTS}" != "1" ]; then
    # Quote the identifier: branch slugs contain '-' (e.g. csv-import), which
    # is invalid in an unquoted SQL identifier.
    PGPASSWORD=health psql -U health -h localhost -d postgres \
      -c "CREATE DATABASE \"${DB_NAME}\""
  fi
fi

cargo run -p health-web -- "$@"
