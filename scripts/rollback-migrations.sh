#!/usr/bin/env bash
# 数据库迁移回滚（破坏性操作，默认 dry-run）。
#
# 与 sqlx::migrate!("./migrations") 配套：SQLx 只提供 up，不提供 down。
# 本脚本按 `--to <版本>` 逆序执行 scripts/sql/rollback.{sqlite,postgres}.sql
# 中大于目标版本的段落，并同步删除 _sqlx_migrations 中对应记录，
# 否则重新 up 时 SQLx 会误判「已应用」而跳过。
#
# 安全护栏：
#   1. 默认 dry-run，只打印将执行的 SQL；必须显式 --confirm 才写入。
#   2. 需要 --to 明确目标版本（例如 --to 012 表示撤销 013/014/015）。
#   3. 生产执行前必须先跑 scripts/backup-postgres.sh 并校验归档。
#   4. 不在命令行传数据库密码（Postgres 走 DATABASE_URL / PG* / ~/.pgpass）。
#
# 用法：
#   # SQLite（离线/本地）
#   bash scripts/rollback-migrations.sh --dialect sqlite --db ./data.db --to 012
#   bash scripts/rollback-migrations.sh --dialect sqlite --db ./data.db --to 012 --confirm
#
#   # Postgres（生产，先备份）
#   DATABASE_URL=postgres://user@host:5432/metaclouds \
#     bash scripts/rollback-migrations.sh --dialect postgres --to 012 --confirm
#
# 注意：离线开发沙箱无 sqlite3/psql CLI，脚本未实测；down SQL 的正确性由
# tests/migrations_rollback_test.rs 在内存 SQLite 上验证。生产需先演练。

set -euo pipefail

DIALECT=""
DB_PATH=""
TARGET=""
CONFIRM=0
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dialect) DIALECT="${2:-}"; shift 2 ;;
    --db) DB_PATH="${2:-}"; shift 2 ;;
    --to) TARGET="${2:-}"; shift 2 ;;
    --confirm) CONFIRM=1; shift ;;
    -h|--help) sed -n '2,28p' "$0"; exit 0 ;;
    *) echo "ERROR: unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "$DIALECT" ]] || [[ -z "$TARGET" ]]; then
  echo "ERROR: --dialect sqlite|postgres and --to <version> are required" >&2
  exit 2
fi
if [[ ! "$TARGET" =~ ^[0-9]+$ ]]; then
  echo "ERROR: --to must be a numeric migration version (e.g. 012)" >&2
  exit 2
fi

case "$DIALECT" in
  sqlite)
    if [[ -z "$DB_PATH" ]]; then
      echo "ERROR: --db <sqlite file> is required for sqlite dialect" >&2
      exit 2
    fi
    if ! command -v sqlite3 >/dev/null 2>&1; then
      echo "ERROR: sqlite3 not found in PATH" >&2
      exit 2
    fi
    DOWN_FILE="$ROOT/scripts/sql/rollback.sqlite.sql"
    ;;
  postgres)
    if ! command -v psql >/dev/null 2>&1; then
      echo "ERROR: psql not found in PATH; install postgresql-client first" >&2
      exit 2
    fi
    DOWN_FILE="$ROOT/scripts/sql/rollback.postgres.sql"
    ;;
  *)
    echo "ERROR: --dialect must be sqlite or postgres" >&2
    exit 2
    ;;
esac

# 抽取版本 > TARGET 的段落（文件本身已按降序排列）。
SQL="$(awk -v target="$((10#$TARGET))" '
  /^-- @down [0-9]+/ {
    ver = $3 + 0
    keep = (ver > target)
  }
  keep { print }
' "$DOWN_FILE")"

if [[ -z "$SQL" ]]; then
  echo "Nothing to roll back: no migration version > $TARGET"
  exit 0
fi

# 同步元数据：删除大于目标版本的 SQLx 迁移记录。
META_SQL="DELETE FROM _sqlx_migrations WHERE version > $((10#$TARGET));"
FULL_SQL="$SQL
$META_SQL"

echo "=== ROLLBACK PLAN (dialect=$DIALECT, to=$TARGET) ==="
echo "$FULL_SQL"
echo "====================================================="

if [[ "$CONFIRM" -ne 1 ]]; then
  echo "DRY RUN — 未执行任何写入。确认备份与目标版本后加 --confirm 重跑。"
  exit 0
fi

echo "[rollback] applying down migrations..."
case "$DIALECT" in
  sqlite) printf '%s\n' "$FULL_SQL" | sqlite3 "$DB_PATH" ;;
  postgres) printf '%s\n' "$FULL_SQL" | psql -v ON_ERROR_STOP=1 "${DATABASE_URL:-}" ;;
esac

echo "[rollback] done. Verify with your schema check before redeploying an older binary."
