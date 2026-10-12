#!/usr/bin/env bash
# ==============================================================================
# provision-test-secret.sh — 用测试值创建 metaclouds-secrets（仅测试集群）
# ==============================================================================
# ⚠️ TEST ONLY — 仅用于测试集群，生产部署必须替换为真实凭据，切勿用于生产。
#   本脚本包含的 secret 值均为明显标注的测试值（统一带 -test- / -change-me 后缀），
#   严禁提交真实生产凭据。生产请使用 External Secrets Operator / Sealed Secrets，
#   或按 02-secret.yaml 注释用 `openssl rand -base64` 生成随机值后手动 kubectl create。
#   本脚本自身进入 git 跟踪没关系（它只含测试值且已标注 TEST ONLY），但不要把真实
#   凭据写进任何 git 跟踪的 YAML（02-secret.yaml 的 CHANGEME 占位请保持不变）。
#
# 用法：
#   ./scripts/provision-test-secret.sh                  # 默认：--dry-run（预演并打印 key 列表，不创建）
#   ./scripts/provision-test-secret.sh --dry-run        # 同上：--dry-run=client -o yaml 预演并打印 key 列表
#   ./scripts/provision-test-secret.sh --apply          # 真正创建/更新 secret（幂等 upsert）
#   ./scripts/provision-test-secret.sh --delete         # 删除该 secret（回滚用）
#   ./scripts/provision-test-secret.sh --check-keys     # 仅打印将要创建的 key 集合（供静态断言使用）
#
# 覆盖的 key（必须与 metaclouds-backend-rust/k8s/02-secret.yaml 的 data: 完全一致，
# 缺失任一 key 都会导致后端启动阶段 fail-closed 退出）：
#   jwt-secret / database-url / redis-password / default-admin-password /
#   postgres-password / protected-endpoints-token
#
# 依赖：脚本使用 bash 4+ 关联数组（declare -A），现代 Git for Windows (2.x+) 自带
#      bash 5.x 满足；极旧 bash 3.x 会失败（关联数组不可用）。
# ==============================================================================
set -euo pipefail

# ---- 通用配置 ----
NAMESPACE="metaclouds"
SECRET_NAME="metaclouds-secrets"

# ---- database-url 拼装所需、与 17-postgresql.yaml 精确对齐的组件 ----
# 来源：17-postgresql.yaml 的 Service name=postgresql + 命名空间 metaclouds，
#       以及 01-configmap.yaml 的 DATABASE_USER/DATABASE_NAME 与 17-postgresql 的
#       POSTGRES_USER/POSTGRES_DB（后者由 configMapKeyRef 注入）、DATABASE_SSL_MODE。
PG_USER="metaclouds_user"                             # 01-configmap DATABASE_USER / 17-postgresql POSTGRES_USER
PG_HOST="postgresql.metaclouds.svc.cluster.local"    # 17-postgresql Service 名 postgresql + namespace metaclouds
PG_PORT="5432"                                        # 17-postgresql Service port / targetPort
PG_DB="metaclouds"                                    # 01-configmap DATABASE_NAME / 17-postgresql POSTGRES_DB
PG_SSLMODE="require"                                  # 01-configmap DATABASE_SSL_MODE（sslmode=require）

# ---- 明显标注的测试值（切勿用于生产）----
# 统一带 -change-me / -test- 后缀，便于在测试集群一眼识别、避免误用到生产。
TEST_JWT_SECRET="test-jwt-secret-change-me-do-not-use-in-prod-0123456789abcdef"
TEST_REDIS_PASSWORD="test-redis-password-change-me"
TEST_DEFAULT_ADMIN_PASSWORD="test-default-admin-password-change-me"
TEST_POSTGRES_PASSWORD="test-postgres-password-change-me"
TEST_PROTECTED_ENDPOINTS_TOKEN="test-protected-endpoints-token-change-me"
# database-url 由上述组件拼装，口令复用 TEST_POSTGRES_PASSWORD，
# 与 17-postgresql 的 POSTGRES_PASSWORD（secretKeyRef: postgres-password）保持一致。
TEST_DATABASE_URL="postgresql://${PG_USER}:${TEST_POSTGRES_PASSWORD}@${PG_HOST}:${PG_PORT}/${PG_DB}?sslmode=${PG_SSLMODE}"

# ---- 将要注入的 key 集合（顺序与 02-secret.yaml data: 保持一致，便于人工核对）----
SECRET_KEYS=(
  "jwt-secret"
  "database-url"
  "redis-password"
  "default-admin-password"
  "postgres-password"
  "protected-endpoints-token"
)

# ---- 各 key 对应的测试值（关联数组，bash 4+）----
declare -A SECRET_VALUES=(
  ["jwt-secret"]="${TEST_JWT_SECRET}"
  ["database-url"]="${TEST_DATABASE_URL}"
  ["redis-password"]="${TEST_REDIS_PASSWORD}"
  ["default-admin-password"]="${TEST_DEFAULT_ADMIN_PASSWORD}"
  ["postgres-password"]="${TEST_POSTGRES_PASSWORD}"
  ["protected-endpoints-token"]="${TEST_PROTECTED_ENDPOINTS_TOKEN}"
)

# ------------------------------------------------------------------------------
# 函数
# ------------------------------------------------------------------------------
usage() {
  cat <<'EOF'
用法：
  provision-test-secret.sh                 # 默认 --dry-run：预演并打印 key 列表，不创建
  provision-test-secret.sh --dry-run       # 同上
  provision-test-secret.sh --apply         # 真正创建/更新 secret（幂等 upsert）
  provision-test-secret.sh --delete        # 删除该 secret（回滚）
  provision-test-secret.sh --check-keys    # 仅打印将要创建的 key 集合（静态断言用）
EOF
}

check_kubectl() {
  if ! command -v kubectl >/dev/null 2>&1; then
    echo "ERROR: 未找到 kubectl，请先配置目标集群上下文（本脚本不连接集群）。" >&2
    exit 1
  fi
}

# 以稳定的标记行打印 key，便于静态断言用 grep 解析
print_keys() {
  for k in "${SECRET_KEYS[@]}"; do
    echo "__SECRET_KEY__=${k}"
  done
}

run_dry_run() {
  check_kubectl
  echo ">> 将要注入 ${SECRET_NAME} 的 key 集合（共 ${#SECRET_KEYS[@]} 个）："
  print_keys
  echo ">> --dry-run=client -o yaml 预演（不会真正创建）："
  local -a cmd=( kubectl -n "${NAMESPACE}" create secret generic "${SECRET_NAME}" )
  for k in "${SECRET_KEYS[@]}"; do
    cmd+=( "--from-literal=${k}=${SECRET_VALUES[$k]}" )
  done
  cmd+=( "--dry-run=client" "-o" "yaml" )
  "${cmd[@]}"
}

run_apply() {
  check_kubectl
  echo ">> 将要注入 ${SECRET_NAME} 的 key 集合（共 ${#SECRET_KEYS[@]} 个）："
  print_keys
  echo ">> 创建/更新 secret（幂等 upsert：先 --dry-run 生成 manifest 再 apply -f -）："
  local -a gen_cmd=( kubectl -n "${NAMESPACE}" create secret generic "${SECRET_NAME}" )
  for k in "${SECRET_KEYS[@]}"; do
    gen_cmd+=( "--from-literal=${k}=${SECRET_VALUES[$k]}" )
  done
  gen_cmd+=( "--dry-run=client" "-o" "yaml" )
  # 生成 manifest 再 apply，等价于 create --dry-run=client -o yaml | kubectl apply -f -
  "${gen_cmd[@]}" | kubectl -n "${NAMESPACE}" apply -f -
  echo ">> 已应用。校验 data 下已有的 key："
  kubectl -n "${NAMESPACE}" get secret "${SECRET_NAME}" \
    -o jsonpath='{.data}' | tr ',' '\n' | sed 's/.:.*//' | sed 's/^/   - /'
}

run_delete() {
  check_kubectl
  echo ">> 删除 secret ${SECRET_NAME}（回滚用，--ignore-not-found 保证幂等）："
  kubectl -n "${NAMESPACE}" delete secret "${SECRET_NAME}" --ignore-not-found
  echo ">> 已删除（若不存在则忽略）。"
}

# ------------------------------------------------------------------------------
# 入口参数解析
# ------------------------------------------------------------------------------
MODE="dry-run"
case "${1:-}" in
  "")            MODE="dry-run" ;;
  --dry-run)     MODE="dry-run" ;;
  --apply)       MODE="apply" ;;
  --delete)      MODE="delete" ;;
  --check-keys)  MODE="check-keys" ;;
  *)             echo "未知参数: $1" >&2; usage; exit 2 ;;
esac

case "$MODE" in
  dry-run)    run_dry_run ;;
  apply)      run_apply ;;
  delete)     run_delete ;;
  check-keys) print_keys ;;
esac
