#!/bin/bash
# Verify migration integrity:
#   1. No existing migration files have been modified or deleted.
#   2. No duplicate migration version numbers exist.
# New migration files (added, not modified) are always allowed.
# Intended for CI and local pre-push checks.
set -euo pipefail

BASE_REF="${1:-origin/main}"
errors=0

# Check 1: existing migrations must not be modified/deleted
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

# Check 2: duplicate version numbers — different migrations using the same prefix
dups="$(
  # Strip path and .up/.down suffix, leaving "VERSION_DESCRIPTION"
  # Then sort -u gives one line per unique migration (not per file)
  # Then extract just the version prefix and look for duplicates
  find migrations/ -name '*.up.sql' -o -name '*.down.sql' \
    | sed 's#.*/##; s#\.\(up\|down\)\.sql$##' \
    | sort -u \
    | sed 's#^\([0-9]\+\).*#\1#' \
    | sort \
    | uniq -d
)"
if [ -n "$dups" ]; then
  echo "ERROR: Multiple migration files share the same version number:"
  echo "$dups" | while IFS= read -r ver; do
    echo "  Version $ver:"
    find migrations/ -name "${ver}_*" -type f | sort | sed 's/^/    /'
  done
  errors=1
fi

if [ "$errors" -ne 0 ]; then
  echo ""
  echo "Existing migrations must never be changed. Create a new migration instead."
  exit 1
fi
echo "All existing migrations are unmodified."
