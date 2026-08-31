#!/usr/bin/env bash
# Master rotation orchestrator. Runs the full live rotation end-to-end:
#   1) PostgreSQL: role `metaclouds` + application `admin` (pure SQL, no external tooling)
#   2) Recreate redis/grafana/backend to load the new .env values
#   3) Verify (health, redis ping, login old-vs-new)
# Pre-flight guard: refuses to run in what looks like production without an explicit flag.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE="${ENV_FILE:-.env}"

fail() { echo "ERROR: $*" >&2; exit 1; }

echo "=== Pre-flight ==="
command -v $COMPOSE >/dev/null 2>&1 || fail "$COMPOSE not found"
[ -f "$ENV_FILE" ] || fail "missing $ENV_FILE"
$COMPOSE ps 2>/dev/null | grep -q postgres || fail "postgres service is not running (start the stack first)"
[ "${ALLOW_PROD:-}" = "1" ] || grep -qiE '^(ENVIRONMENT|SERVER_ENV|GO_ENV)=production' "$ENV_FILE" 2>/dev/null \
  && fail "ENVIRONMENT=production detected. Re-run with ALLOW_PROD=1 only if you intend to rotate prod."

read -r -p "This rotates LIVE credentials and recreates containers. Continue? [y/N] " ans
[ "$ans" = "y" ] || { echo "Aborted."; exit 1; }

echo "=== 1/3 PostgreSQL (role + app admin) ==="
"$ROOT/scripts/rotate_db_credentials.sh"

echo "=== 2/3 Recreate redis/grafana/backend to load new .env ==="
$COMPOSE up -d --force-recreate redis grafana backend

echo "=== 3/3 Verify ==="
"$ROOT/scripts/verify_rotation.sh"

echo "=== Rotation complete ==="
echo "If Grafana was NOT recreated (you used grafana-cli separately), it is already rotated."
echo "Old JWTs are now invalid (JWT_SECRET changed) -> users must re-login."
