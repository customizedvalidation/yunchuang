#!/usr/bin/env bash
# Rotate PostgreSQL credentials for a RUNNING metaclouds stack.
#   1) superuser role `metaclouds` (DB connection used by the app) -> POSTGRES_PASSWORD
#   2) application `admin` account (web login)                  -> DEFAULT_ADMIN_PASSWORD
# Uses Postgres pgcrypto.crypt(), so NO external bcrypt tooling (Go/Python) is needed.
# Idempotent & non-destructive to data. Requires: docker compose, postgres service UP.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE="${ENV_FILE:-.env}"

fail() { echo "ERROR: $*" >&2; exit 1; }

[ -f "$ENV_FILE" ] || fail "missing $ENV_FILE (run from workspace root or set ENV_FILE)"
command -v $COMPOSE >/dev/null 2>&1 || fail "$COMPOSE not found"

POSTGRES_PASSWORD=$(grep -E '^POSTGRES_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)
DEFAULT_ADMIN_PASSWORD=$(grep -E '^DEFAULT_ADMIN_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)
[ -n "$POSTGRES_PASSWORD" ] || fail "POSTGRES_PASSWORD not set in $ENV_FILE"
[ -n "$DEFAULT_ADMIN_PASSWORD" ] || fail "DEFAULT_ADMIN_PASSWORD not set in $ENV_FILE"

echo ">> ensure pgcrypto extension (idempotent)"
$COMPOSE exec -T postgres psql -U metaclouds -d metaclouds -c "CREATE EXTENSION IF NOT EXISTS pgcrypto;"

echo ">> rotate superuser role 'metaclouds' (SET log_statement=none to avoid logging the password)"
$COMPOSE exec -T postgres psql -U metaclouds -d metaclouds -v pw="$POSTGRES_PASSWORD" \
  -c "SET log_statement='none'; ALTER USER metaclouds WITH PASSWORD :'pw';"

echo ">> rotate application admin account (bcrypt via pgcrypto, cost 10 = Go bcrypt.DefaultCost)"
$COMPOSE exec -T postgres psql -U metaclouds -d metaclouds -v pw="$DEFAULT_ADMIN_PASSWORD" \
  -c "SET log_statement='none'; UPDATE users SET password = crypt(:'pw', gen_salt('bf', 10)) WHERE username = 'admin';"

echo ">> verify app admin row exists"
N=$($COMPOSE exec -T postgres psql -U metaclouds -d metaclouds -tAc "SELECT count(*) FROM users WHERE username='admin';")
[ "${N:-0}" -ge 1 ] || echo "WARN: admin row not found (DB may use a different store); check manually."

echo "DONE: PostgreSQL credentials rotated."
echo "Next: recreate backend (and redis/grafana) so they pick up the new .env values:"
echo "  $COMPOSE up -d --force-recreate backend redis grafana"
