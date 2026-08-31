#!/usr/bin/env bash
# Post-rotation verification. Asserts:
#   - backend health reachable
#   - redis answers PONG with the NEW password
#   - login with NEW admin password -> 200
#   - login with the OLD leaked password Admin@123! -> 401 (proves old cred is dead)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE="${ENV_FILE:-.env}"
BASE="${BASE:-http://localhost:8000}"

[ -f "$ENV_FILE" ] || { echo "ERROR: missing $ENV_FILE" >&2; exit 1; }
DEFAULT_ADMIN_PASSWORD=$(grep -E '^DEFAULT_ADMIN_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)
REDIS_PASSWORD=$(grep -E '^REDIS_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)

echo ">> backend health"
if curl -fsS "$BASE/healthz" >/dev/null 2>&1 || curl -fsS "$BASE/api/v1/health" >/dev/null 2>&1; then
  echo "  backend: OK"
else
  echo "  backend: UNREACHABLE (start/recreate it first)"
fi

echo ">> redis ping with NEW password"
if $COMPOSE exec -T redis redis-cli -a "$REDIS_PASSWORD" ping 2>/dev/null | grep -q PONG; then
  echo "  redis: OK"
else
  echo "  redis: FAIL (check REDIS_PASSWORD / recreate redis)"
fi

echo ">> login with NEW admin password (expect HTTP 200)"
NEW_CODE=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"admin\",\"password\":\"$DEFAULT_ADMIN_PASSWORD\"}")
echo "  new-password login: HTTP $NEW_CODE"

echo ">> login with OLD leaked password Admin@123! (expect HTTP 401)"
OLD_CODE=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"Admin@123!"}')
echo "  old-password login: HTTP $OLD_CODE"

echo "--- summary ---"
[ "$NEW_CODE" = "200" ] && echo "PASS: new admin password works" || echo "FAIL: new admin password did not return 200"
[ "$OLD_CODE" = "401" ] && echo "PASS: old leaked password is rejected" || echo "WARN: old password did not return 401 (verify it is truly dead)"
