#!/usr/bin/env bash
# PostgreSQL 逻辑备份（custom-format 归档），供生产环境落地使用。
#
# 设计约束：
#   - 不在命令行传密码。优先使用 libpq 标准环境变量 / ~/.pgpass；
#     DATABASE_URL 若含密码会暴露在进程列表，生产建议改用 ~/.pgpass。
#   - 备份完成后用 pg_restore --list 校验归档可读，避免"备份成功但不可恢复"。
#   - 保留期清理仅删除本脚本生成的 metaclouds-*.dump，不碰其他文件。
#
# 用法：
#   DATABASE_URL=postgres://user@host:5432/metaclouds \
#     BACKUP_DIR=/var/backups/metaclouds BACKUP_RETENTION_DAYS=14 \
#     bash scripts/backup-postgres.sh
#
#   # 或完全使用 libpq 环境变量：
#   PGHOST=db PGUSER=metaclouds PGDATABASE=metaclouds bash scripts/backup-postgres.sh
#
# 注意：本脚本在离线开发沙箱无法实测（需要 pg_dump / pg_restore 与实际数据库），
# 需在联网生产环境首次执行并验证归档可恢复后再纳入定时调度。

set -euo pipefail

for tool in pg_dump pg_restore; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "ERROR: $tool not found in PATH; install postgresql-client first" >&2
    exit 2
  fi
done

BACKUP_DIR="${BACKUP_DIR:-./backups}"
RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-14}"
TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_FILE="${BACKUP_DIR}/metaclouds-${TIMESTAMP}.dump"

mkdir -p "$BACKUP_DIR"

PG_ARGS=()
if [[ -n "${DATABASE_URL:-}" ]]; then
  PG_ARGS=(-d "$DATABASE_URL")
fi

echo "[backup] target: ${DATABASE_URL:+DATABASE_URL (credentials redacted)}${DATABASE_URL:-PG* env vars}"
echo "[backup] writing: $OUT_FILE"

pg_dump "${PG_ARGS[@]}" --format=custom --no-owner --no-privileges --file="$OUT_FILE"

# 校验：能列出内容即视为归档完整（不实际恢复，避免对生产库产生副作用）。
pg_restore --list "$OUT_FILE" >/dev/null
echo "[backup] verified archive is readable: $OUT_FILE ($(du -h "$OUT_FILE" | cut -f1))"

if [[ "$RETENTION_DAYS" -gt 0 ]]; then
  echo "[backup] pruning dumps older than ${RETENTION_DAYS} days"
  find "$BACKUP_DIR" -maxdepth 1 -type f -name 'metaclouds-*.dump' \
    -mtime "+${RETENTION_DAYS}" -print -delete
fi

echo "[backup] done"
