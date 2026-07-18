#!/usr/bin/env bash
set -euo pipefail

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" = "HEAD" ]; then
  BRANCH="detached-$(git rev-parse --short HEAD)"
fi
BRANCH_SLUG="$(echo "$BRANCH" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g; s/^-//; s/-$//')"
BRANCH_HASH="$(echo "$BRANCH" | md5sum | head -c 8)"
DB_NAME="ht_prepare_${BRANCH_SLUG}_${BRANCH_HASH}"

export DATABASE_URL="postgres://health:health@localhost:5432/${DB_NAME}"

cargo sqlx database reset -y
cargo sqlx prepare --workspace
