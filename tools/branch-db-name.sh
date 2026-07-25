#!/usr/bin/env bash
set -euo pipefail

PREFIX="${1:-ht_dev}"

BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [ "$BRANCH" = "HEAD" ]; then
  BRANCH="detached-$(git rev-parse --short HEAD)"
fi
BRANCH_SLUG="$(echo "$BRANCH" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g; s/^-//; s/-$//')"
BRANCH_HASH="$(echo "$BRANCH" | md5sum | head -c 8)"
echo "${PREFIX}_${BRANCH_SLUG}_${BRANCH_HASH}"
