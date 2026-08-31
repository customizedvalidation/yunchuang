#!/usr/bin/env bash
# ==============================================================================
# 从已轮换的 .env.production 生成 Kubernetes Secret（不在仓库中提交任何密钥）。
#
# 该 Secret 被后端 Pod 与备份 CronJob 共同使用，是集群内数据库/管理员/Redis 密码的
# 唯一可信来源。请勿把真实密钥写进任何 YAML 或文档。
#
# 用法:
#   ./generate_k8s_secrets.sh                 # 默认读取 ../.env.production
#   ./generate_k8s_secrets.sh -f /path/.env.production
#   ./generate_k8s_secrets.sh -n my-namespace
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_FILE="$SCRIPT_DIR/../.env.production"
NS="metaclouds"
SEC_NAME="metaclouds-secrets"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -f|--file) ENV_FILE="$2"; shift 2;;
    -n|--namespace) NS="$2"; shift 2;;
    *) shift;;
  esac
done

if [[ ! -f "$ENV_FILE" ]]; then
  echo "ERROR: 配置文件不存在: $ENV_FILE" >&2
  echo "提示: 复制 .env.production.example 为 .env.production 并填入已轮换的强密码。" >&2
  exit 1
fi

require() {
  local key="$1"
  local v
  v="$(grep "^${key}=" "$ENV_FILE" | tail -n1 | cut -d'=' -f2- | sed -e "s/^[\"']//" -e "s/[\"']$//" || true)"
  if [[ -z "$v" ]]; then
    echo "ERROR: $key 未在 $ENV_FILE 中设置" >&2
    exit 1
  fi
  printf '%s' "$v"
}

JWT=$(require SECURE_JWT_SECRET)
ADMIN=$(require SECURE_DEFAULT_ADMIN_PASSWORD)
DB=$(require SECURE_DATABASE_PASSWORD)

ARGS=(--from-literal=jwt-secret="$JWT" --from-literal=default-admin-password="$ADMIN" --from-literal=database-password="$DB")
if grep -q '^SECURE_REDIS_PASSWORD=' "$ENV_FILE"; then
  REDIS_VAL=$(require SECURE_REDIS_PASSWORD)
  ARGS+=(--from-literal=redis-password="$REDIS_VAL")
fi

# 用 --dry-run 生成清单后 apply，避免把密钥落盘到任何文件
kubectl -n "$NS" create secret generic "$SEC_NAME" "${ARGS[@]}" --dry-run=client -o yaml | kubectl apply -f -
echo "✓ Secret '$SEC_NAME' 已在命名空间 '$NS' 创建/更新（源: $ENV_FILE，未提交到仓库）"
echo "  备份 CronJob 也通过该 Secret 的 database-password 访问数据库。"
