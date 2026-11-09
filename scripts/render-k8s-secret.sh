#!/usr/bin/env bash
# ==============================================================================
# 从 CI secrets 渲染真实的 K8s Secret 工作副本（2026-11-09 第三轮）
#
# 问题：02-secret.yaml 是「占位模板」，恒含 CHANGEME。此前 CI 的
#       "Guard - reject placeholder secrets" 直接解析仓库里的模板，因此只要
#       K8S_ENABLED=true 就 100% 失败；即便人工绕过 Guard，
#       `kustomize build | kubectl apply -f -` 又会把集群中预置的真实 Secret
#       覆盖回占位值（deploy-cluster.sh 的 ④.5 步骤就是在打这个补丁）。
#
# 方案：apply 之前，用本脚本把 6 个真实凭据渲染进 02-secret.yaml 的**工作副本**
#       （只存在于 runner 的 checkout 中，绝不提交），随后 kustomize build
#       拿到的就是真实 Secret，Guard 校验渲染结果才有意义。
#
# 用法：
#   bash scripts/render-k8s-secret.sh                 # 覆盖 k8s/02-secret.yaml（CI）
#   bash scripts/render-k8s-secret.sh --dry-run      # 只校验变量，不写文件
#   bash scripts/render-k8s-secret.sh --output /tmp/x.yaml
#
# 安全：不打印任何凭据明文；产物权限 600；输出到 stdout 的只有状态信息。
# ==============================================================================
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_OUT="${REPO_ROOT}/metaclouds-backend-rust/k8s/02-secret.yaml"
OUTPUT="$DEFAULT_OUT"
DRY_RUN=0

while [ $# -gt 0 ]; do
  case "$1" in
    --output)   OUTPUT="${2:-}"; shift 2 ;;
    --output=*) OUTPUT="${1#*=}"; shift ;;
    --dry-run)  DRY_RUN=1; shift ;;
    -h|--help)  echo "Usage: $0 [--output <path>] [--dry-run]"; exit 0 ;;
    *) echo "::error::unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [ -z "$OUTPUT" ]; then
  echo "::error::--output must not be empty" >&2
  exit 2
fi

# key 名 -> 环境变量名 -> 最小长度
KEYS=(
  "jwt-secret|K8S_JWT_SECRET|32"
  "database-url|K8S_DATABASE_URL|20"
  "redis-password|K8S_REDIS_PASSWORD|16"
  "default-admin-password|K8S_DEFAULT_ADMIN_PASSWORD|12"
  "postgres-password|K8S_POSTGRES_PASSWORD|16"
  "protected-endpoints-token|K8S_PROTECTED_ENDPOINTS_TOKEN|16"
)

b64() { printf '%s' "$1" | base64 | tr -d '\n'; }

MISSING=0
declare -a ENCODED=()
for spec in "${KEYS[@]}"; do
  IFS='|' read -r key var minlen <<< "$spec"
  value="${!var:-}"
  if [ -z "$value" ]; then
    echo "::error::missing CI secret: $var (K8s key: $key)" >&2
    MISSING=1
    continue
  fi
  case "$value" in
    *CHANGEME*|*placeholder*)
      echo "::error::$var still contains a placeholder value" >&2
      MISSING=1
      continue ;;
  esac
  if [ "${#value}" -lt "$minlen" ]; then
    echo "::error::$var is shorter than $minlen characters" >&2
    MISSING=1
    continue
  fi
  if [ "$key" = "database-url" ] && ! printf '%s' "$value" | grep -Eq '^postgres(ql)?://[^:]+:[^@[:space:]]+@[^/[:space:]]+/[^[:space:]]+'; then
    echo "::error::K8S_DATABASE_URL must be a PostgreSQL URL with credentials" >&2
    MISSING=1
    continue
  fi
  ENCODED+=("${key}|$(b64 "$value")")
done

if [ "$MISSING" -ne 0 ]; then
  echo "::error::refusing to render a Secret with missing/placeholder credentials" >&2
  echo "::error::see docs/deployment-prerequisites.md (section: K8s 真实 Secret 注入)" >&2
  exit 1
fi

if [ "$DRY_RUN" -eq 1 ]; then
  echo "render dry-run PASSED: ${#ENCODED[@]} keys validated, no file written"
  exit 0
fi

mkdir -p "$(dirname "$OUTPUT")"
TMP="$(mktemp "${OUTPUT}.XXXXXX")"
chmod 600 "$TMP"
{
  cat <<'EOF'
# ==============================================================================
# 02-secret.yaml — CI 运行期渲染产物（DO NOT COMMIT）
# 由 scripts/render-k8s-secret.sh 从 GitHub Secrets 生成，仅存在于 runner 的
# checkout 工作副本中。仓库内的同名文件是占位模板（不含真实凭据）。
# 生产更优方案：External Secrets Operator / Sealed Secrets 托管，Git 不落明文。
# ==============================================================================
apiVersion: v1
kind: Secret
metadata:
  name: metaclouds-secrets
  namespace: metaclouds
  labels:
    app: metaclouds
    component: backend
    app.kubernetes.io/part-of: metaclouds
  annotations:
    metaclouds.io/managed-by: "ci-preflight"
    metaclouds.io/rotation-policy: "90d"
type: Opaque
data:
EOF
  for entry in "${ENCODED[@]}"; do
    IFS='|' read -r key encoded <<< "$entry"
    printf '  %s: %s\n' "$key" "$encoded"
  done
} > "$TMP"

# 渲染后自检：只解析 data: 段的 base64 值并解码检查。
# 不能对全文 grep "CHANGEME"——注释里提到占位模板就会误判；
# 也不能用宽松正则数 key——"namespace: metaclouds" 会被算成第 7 个 data key。
mv "$TMP" "$OUTPUT"
chmod 600 "$OUTPUT"

COUNT=0
IN_DATA=0
while IFS= read -r line || [ -n "$line" ]; do
  if [ "$line" = "data:" ]; then IN_DATA=1; continue; fi
  [ "$IN_DATA" -eq 1 ] || continue
  case "$line" in ''|\#*) continue ;; esac
  read -r raw_key value <<< "$line" || continue
  key="${raw_key%:}"
  [ -n "$value" ] || continue
  if ! plain="$(printf '%s' "$value" | base64 -d 2>/dev/null)"; then
    echo "::error::rendered Secret: cannot base64-decode $key" >&2
    exit 1
  fi
  case "$plain" in
    *CHANGEME*|*placeholder*)
      echo "::error::rendered Secret: $key still holds a placeholder value" >&2
      exit 1 ;;
  esac
  COUNT=$((COUNT + 1))
done < "$OUTPUT"

if [ "$COUNT" -ne "${#KEYS[@]}" ]; then
  echo "::error::rendered Secret has $COUNT data keys, expected ${#KEYS[@]}" >&2
  exit 1
fi

echo "rendered real K8s Secret at $OUTPUT (${COUNT} keys, values redacted)"
