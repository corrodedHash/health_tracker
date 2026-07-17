#!/bin/bash
# Verify that no existing migration files have been modified or deleted.
# New migration files (added, not modified) are always allowed.
# Intended for CI and local pre-push checks.
set -euo pipefail

BASE_REF="${1:-origin/main}"

errors=0
while IFS= read -r f; do
  if [ -f "$f" ]; then
    if ! git diff --quiet "$BASE_REF" -- "$f" 2>/dev/null; then
      echo "ERROR: Migration file modified: $f"
      errors=1
    fi
  else
    echo "ERROR: Migration file deleted: $f"
    errors=1
  fi
done < <(git ls-tree -r --name-only "$BASE_REF" -- migrations/ | grep -E '\.(up|down)\.sql$')

if [ "$errors" -ne 0 ]; then
  echo ""
  echo "Existing migrations must never be changed. Create a new migration instead."
  exit 1
fi
echo "All existing migrations are unmodified."
