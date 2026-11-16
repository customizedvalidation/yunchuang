#!/usr/bin/env bash
# 从 custom-format 归档恢复 PostgreSQL（破坏性操作）。
#
# 安全护栏：
#   1. 默认 dry-run：只展示归档内容与目标连接，不做任何写入。
#   2. 必须显式传 --confirm 才执行恢复。
#   3. 恢复前自动对当前库做一次快照（同目录 metaclouds-pre-restore-*.dump）。
#   4. 不在命令行传密码（连接走 DATABASE_URL / PG* 环境变量 / ~/.pgpass）。
#
# 用法：
#   DATABASE_URL=postgres://user@host:5432/metaclouds \
#     bash scripts/restore-postgres.sh --file /var/backups/metaclouds-20261116T020000Z.dump
#   # 检查无误后：
#   DATABASE_URL=... bash scripts/restore-postgres.sh --file <同上> --confirm
#
# 注意：本脚本含破坏性操作，且离线开发沙箱无法实测；务必先在预发环境演练。

set -euo pipefail

BACKUP_FILE=""
CONFIRM=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --file) BACKUP_FILE="${2:-}"; shift 2 ;;
    --confirm) CONFIRM=1; shift ;;
    -h|--help)
      sed -n '2,16p' "$0"
      exit 0
      ;;
    *)
      echo "ERROR: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$BACKUP_FILE" ]]; then
  echo "ERROR: --file <custom-format dump> is required" >&2
  exit 2
fi
if [[ ! -f "$BACKUP_FILE" ]]; then
  echo "ERROR: backup file not found: $BACKUP_FILE" >&2
  exit 2
fi
for tool in pg_dump pg_restore; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "ERROR: $tool not found in PATH; install postgresql-client first" >&2
    exit 2
  fi
done

PG_ARGS=()
if [[ -n "${DATABASE_URL:-}" ]]; then
  PG_ARGS=(-d "$DATABASE_URL")
fi

echo "[restore] archive : $BACKUP_FILE"
echo "[restore] target  : ${DATABASE_URL:+DATABASE_URL (credentials redacted)}${DATABASE_URL:-PG* env vars}"
echo "[restore] archive entries (first 20):"
pg_restore --list "$BACKUP_FILE" | head -20

if [[ "$CONFIRM" -ne 1 ]]; then
  echo ""
  echo "DRY RUN — 未执行任何恢复。"
  echo "确认归档与目标库无误后，加 --confirm 重跑。"
  exit 0
fi

SNAPSHOT="${BACKUP_DIR:-./backups}/metaclouds-pre-restore-$(date -u +%Y%m%dT%H%M%SZ).dump"
mkdir -p "$(dirname "$SNAPSHOT")"
echo "[restore] snapshot current DB -> $SNAPSHOT"
pg_dump "${PG_ARGS[@]}" --format=custom --no-owner --no-privileges --file="$SNAPSHOT"

echo "[restore] applying archive (--clean --if-exists)..."
pg_restore "${PG_ARGS[@]}" --clean --if-exists --no-owner --no-privileges "$BACKUP_FILE"

echo "[restore] done. Pre-restore snapshot kept at: $SNAPSHOT"
