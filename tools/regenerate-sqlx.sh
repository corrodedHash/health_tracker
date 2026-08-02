#!/usr/bin/env bash
set -euo pipefail

DB_NAME=$(./tools/branch-db-name.sh ht_prepare)
export DATABASE_URL="postgres://health:health@localhost:5432/${DB_NAME}"

cargo sqlx database reset -y
# sqlx-cli 0.9.0 only builds the root `health-tracker` package (no `query!`
# macros) unless the workspace is forwarded to cargo via `-- --workspace`.
cargo sqlx prepare --workspace -- --workspace
