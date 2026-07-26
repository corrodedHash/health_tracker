#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/dex-config.yaml"

if ! command -v curl &>/dev/null; then
  echo "curl not found — skipping OIDC health check."
  exit 0
fi

if curl -sf http://localhost:5556/dex/.well-known/openid-configuration &>/dev/null; then
  echo "Dex (OIDC provider) already running on localhost:5556."
  exit 0
fi

echo "No Dex found on localhost:5556, starting temporary container..."

docker run --rm -d --name ht-dex-ensure \
  -p 5556:5556 \
  -v "$CONFIG_FILE:/etc/dex/config.docker.yaml:ro" \
  ghcr.io/dexidp/dex:v2.42.0

echo "Waiting for Dex to be ready..."
for i in $(seq 1 30); do
  if curl -sf http://localhost:5556/dex/.well-known/openid-configuration &>/dev/null; then
    echo "Dex is ready."
    break
  fi
  sleep 1
done
