#!/usr/bin/env bash
# Rotate Redis requirepass.
#  - If OLD_REDIS_PASSWORD is provided: live CONFIG SET (no restart), then CONFIG REWRITE to persist.
#  - Otherwise: fall back to container recreate, which applies the new REDIS_PASSWORD from .env
#    (compose passes --requirepass on start). This avoids the "need the old password" chicken-egg.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE="${ENV_FILE:-.env}"

fail() { echo "ERROR: $*" >&2; exit 1; }
[ -f "$ENV_FILE" ] || fail "missing $ENV_FILE"
command -v $COMPOSE >/dev/null 2>&1 || fail "$COMPOSE not found"

REDIS_PASSWORD=$(grep -E '^REDIS_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)
[ -n "$REDIS_PASSWORD" ] || fail "REDIS_PASSWORD not set in $ENV_FILE"

if [ -n "${OLD_REDIS_PASSWORD:-}" ]; then
  echo ">> live-rotating redis requirepass (no restart)"
  $COMPOSE exec -T redis redis-cli -a "$OLD_REDIS_PASSWORD" CONFIG SET requirepass "$REDIS_PASSWORD" 2>/dev/null
  $COMPOSE exec -T redis redis-cli -a "$REDIS_PASSWORD" CONFIG REWRITE 2>/dev/null
  echo "DONE (live). Recreate backend to reconnect: $COMPOSE up -d --force-recreate backend"
else
  echo "OLD_REDIS_PASSWORD not set -> applying via container recreate (reads new .env)"
  $COMPOSE up -d --force-recreate redis
  echo "DONE (recreate). Recreate backend: $COMPOSE up -d --force-recreate backend"
fi
