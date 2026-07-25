#!/usr/bin/env bash
set -euo pipefail

DB_NAME=$(./tools/branch-db-name.sh ht_prepare)
export DATABASE_URL="postgres://health:health@localhost:5432/${DB_NAME}"

cargo sqlx database reset -y
cargo sqlx prepare --workspace
