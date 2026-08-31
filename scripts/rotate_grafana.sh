#!/usr/bin/env bash
# Reset Grafana admin password on a RUNNING stack (zero-restart) using grafana-cli.
# grafana-cli talks to Grafana's DB directly, so the OLD password is NOT required.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
COMPOSE=${COMPOSE:-"docker compose"}
ENV_FILE="${ENV_FILE:-.env}"

fail() { echo "ERROR: $*" >&2; exit 1; }
[ -f "$ENV_FILE" ] || fail "missing $ENV_FILE"
command -v $COMPOSE >/dev/null 2>&1 || fail "$COMPOSE not found"

GRAFANA_ADMIN_PASSWORD=$(grep -E '^GRAFANA_ADMIN_PASSWORD=' "$ENV_FILE" | head -1 | cut -d= -f2-)
[ -n "$GRAFANA_ADMIN_PASSWORD" ] || fail "GRAFANA_ADMIN_PASSWORD not set in $ENV_FILE"

echo ">> reset grafana admin password (zero-restart)"
$COMPOSE exec -T grafana grafana-cli admin reset-admin-password "$GRAFANA_ADMIN_PASSWORD"
echo "DONE."
