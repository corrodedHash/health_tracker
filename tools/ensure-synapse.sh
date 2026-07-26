#!/usr/bin/env bash
set -euo pipefail

SYNAPSE_CONTAINER="ht-synapse"
SYNAPSE_VOLUME="ht-synapse-data"
SYNAPSE_PORT=8008
SYNAPSE_SERVER_NAME="localhost"
SYNAPSE_IMAGE="matrixdotorg/synapse:latest"
CONFIG_FILE="config/local.toml"
API_BASE="http://localhost:3000"
COOKIE_JAR="/tmp/ht-synapse-cookies.txt"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# ---------------------------------------------------------------------------
# 1. Ensure Synapse is running
# ---------------------------------------------------------------------------
synapse_ready() {
  curl -sf "http://localhost:${SYNAPSE_PORT}/_matrix/client/versions" >/dev/null 2>&1
}

if synapse_ready; then
  echo -e "${GREEN}✓${NC} Synapse already running on localhost:${SYNAPSE_PORT}."
else
  docker rm -f "$SYNAPSE_CONTAINER" 2>/dev/null || true

  if ! docker volume inspect "$SYNAPSE_VOLUME" &>/dev/null; then
    echo "Creating Synapse data volume..."
    docker volume create "$SYNAPSE_VOLUME"
    echo "Generating Synapse config..."
    docker run --rm \
      -v "$SYNAPSE_VOLUME:/data" \
      -e SYNAPSE_SERVER_NAME="$SYNAPSE_SERVER_NAME" \
      -e SYNAPSE_REPORT_STATS=no \
      "$SYNAPSE_IMAGE" generate
  fi

  echo "Starting Synapse container..."
  docker run -d \
    --name "$SYNAPSE_CONTAINER" \
    -v "$SYNAPSE_VOLUME:/data" \
    -p "127.0.0.1:${SYNAPSE_PORT}":8008 \
    "$SYNAPSE_IMAGE"

  echo "Waiting for Synapse to be ready..."
  for i in $(seq 1 60); do
    if synapse_ready; then
      echo -e "${GREEN}✓${NC} Synapse is ready."
      break
    fi
    sleep 2
  done

  if ! synapse_ready; then
    echo "ERROR: Synapse did not become ready within 120 seconds."
    docker logs "$SYNAPSE_CONTAINER"
    exit 1
  fi

  # Register bot user (idempotent)
  echo "Registering bot user..."
  docker exec "$SYNAPSE_CONTAINER" \
    register_new_matrix_user -c /data/homeserver.yaml \
    -u botuser -p botpass --admin http://localhost:8008 2>/dev/null || \
    echo "  (bot user may already exist)"

  # Register test user (idempotent)
  echo "Registering test user..."
  docker exec "$SYNAPSE_CONTAINER" \
    register_new_matrix_user -c /data/homeserver.yaml \
    -u testuser -p testpass --no-admin http://localhost:8008 2>/dev/null || \
    echo "  (test user may already exist)"
fi

# ---------------------------------------------------------------------------
# 2. Obtain an API bearer token (if web API is reachable)
# ---------------------------------------------------------------------------
TOKEN="<get-from-web-api>"

API_STATUS=$(curl -s --max-time 3 -o /dev/null -w '%{http_code}' "${API_BASE}/auth/login" 2>/dev/null || echo "000")
if [ "$API_STATUS" != "000" ]; then
  echo -e "Web API reachable at ${API_BASE}, obtaining bearer token..."

  rm -f "$COOKIE_JAR"
  if curl -sf -c "$COOKIE_JAR" "${API_BASE}/auth/login" >/dev/null 2>&1 && [ -s "$COOKIE_JAR" ]; then
    TOKEN_RESP=$(curl -sf -b "$COOKIE_JAR" \
      -X POST "${API_BASE}/api/tokens" \
      -H 'Content-Type: application/json' \
      -d '{"label":"health-bot-dev"}')

    if [ -n "$TOKEN_RESP" ]; then
      if command -v jq &>/dev/null; then
        TOKEN=$(echo "$TOKEN_RESP" | jq -r '.token')
      else
        TOKEN=$(echo "$TOKEN_RESP" | sed 's/.*"token":"\([^"]*\)".*/\1/')
      fi
    fi
  fi
  rm -f "$COOKIE_JAR"
fi

# ---------------------------------------------------------------------------
# 3. Generate config/local.toml
# ---------------------------------------------------------------------------
echo ""
echo "=== Generating ${CONFIG_FILE} ==="
mkdir -p "$(dirname "$CONFIG_FILE")"

cat > "$CONFIG_FILE" << EOF
[matrix]
homeserver = "http://localhost:${SYNAPSE_PORT}"
user_id = "@botuser:localhost"
password = "botpass"
session_file = "session.toml"

[api]
base_url = "${API_BASE}"
token = "${TOKEN}"
EOF

echo -e "${GREEN}✓${NC} Written ${CONFIG_FILE}"

# ---------------------------------------------------------------------------
# 4. Summary
# ---------------------------------------------------------------------------
echo ""
echo "=== Synapse E2E test environment ==="
echo -e "  ${CYAN}Homeserver:${NC}  http://localhost:${SYNAPSE_PORT}"
echo -e "  ${CYAN}Bot user:${NC}     @botuser:localhost / botpass"
echo -e "  ${CYAN}Test user:${NC}    @testuser:localhost / testpass"

if [ "$TOKEN" = "<get-from-web-api>" ]; then
  echo ""
  echo -e "${YELLOW}⚠ No bearer token in config.${NC}"
  echo "  Start the web API and re-run this script to auto-configure:"
  echo "    mise run run-web"
  echo "    mise run ensure-synapse"
  echo ""
  echo "  Or get a token manually:"
  echo "    curl -c ${COOKIE_JAR} '${API_BASE}/auth/login'"
  echo "    curl -b ${COOKIE_JAR} -X POST '${API_BASE}/api/tokens' \\"
  echo "      -H 'Content-Type: application/json' -d '{\"label\":\"bot\"}'"
else
  echo -e "${GREEN}✓${NC} Bearer token configured in ${CONFIG_FILE}"
fi

echo ""
echo -e "Start the bot: ${CYAN}mise run run-bot${NC}"
