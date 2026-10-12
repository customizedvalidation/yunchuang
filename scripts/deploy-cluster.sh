#!/usr/bin/env bash
# ==============================================================================
# deploy-cluster.sh — 在目标集群执行"运行期前置" + 整包部署（测试数据版）
# ==============================================================================
# ⚠️ TEST ONLY — 本脚本第 ③ 步调用的 provision-test-secret.sh 注入的是明显标注的
#   测试凭据，仅适用于测试集群；生产请勿使用，必须替换为真实凭据（见 docs 下的
#   生产切换 checklist）。本脚本不修改任何仓库内的 YAML 占位。
#
# 编排顺序（对应 05-deployment.yaml 文件头的"前置"要求 + 17-postgresql 注释）：
#   ① kubectl apply -f metaclouds-backend-rust/k8s/17-postgresql.yaml   # 先部署 PostgreSQL 数据面
#   ② 等待 PostgreSQL Pod Ready（rollout status / wait）                # 否则后端连不上库会 fail-closed
#   ③ scripts/provision-test-secret.sh --apply                         # 注入测试 secret（覆盖 02-secret 全部 key）
#   ④ ./scripts/deploy-k8s.sh（或 kubectl apply -k）                    # 整包 apply（含后端 3 副本 + Redis + 前端）
#   ④.5 回填测试 secret（见下：抵消 apply -k 对 02-secret 占位的覆盖）
#   ⑤ 验证：kubectl -n metaclouds get pods -l app=metaclouds + 后端 /health
#
# ⚠️ 关于 ④.5（回填测试 secret）：13-kustomization.yaml 引用了 02-secret.yaml，
#    步骤 ④ 的 `kubectl apply -k` 会按 02-secret.yaml 的 CHANGEME 占位把
#    metaclouds-secrets 覆盖回占位值（集群侧无 CI 门禁，apply 会成功），导致后端
#    启动因 database-url 仍是占位而 fail-closed。因此在 ④ 之后必须再次 upsert
#    测试值，确保最终 secret 为真实（测试）值。生产请直接以真实凭据注入替代 ②/④.5
#    两步，且不要依赖 02-secret.yaml 的占位 apply。
#
# 用法：
#   ./scripts/deploy-cluster.sh            # 真正执行（需 kubectl 已配置目标集群）
#   ./scripts/deploy-cluster.sh --dry-run  # 预览所有命令（kubectl --dry-run=client 校验，不真正变更）
#
# 前置：kubectl 已配置目标集群上下文；命名空间 metaclouds 本脚本会按需创建（缺失时）。
# 说明：本沙箱无 kubectl/集群，仅用于静态校验语法；真实执行须在用户侧已配置 kubectl 的环境。
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"   # .../scripts
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"                  # 仓库根
K8S_DIR="${REPO_ROOT}/metaclouds-backend-rust/k8s"
NAMESPACE="metaclouds"
PROVISION_SECRET="${SCRIPT_DIR}/provision-test-secret.sh"
DEPLOY_K8S="${SCRIPT_DIR}/deploy-k8s.sh"
DRY_RUN=0

usage() {
  cat <<'EOF'
用法：
  deploy-cluster.sh            # 真正执行（需 kubectl 已配置目标集群）
  deploy-cluster.sh --dry-run  # 预览所有命令（kubectl --dry-run=client，不真正变更）
EOF
}

case "${1:-}" in
  "")          DRY_RUN=0 ;;
  --dry-run)   DRY_RUN=1 ;;
  *)           echo "未知参数: $1" >&2; usage; exit 2 ;;
esac

# kubectl 可用性检查
if ! command -v kubectl >/dev/null 2>&1; then
  echo "ERROR: 未找到 kubectl，请先配置目标集群上下文。" >&2
  exit 1
fi

# ------------------------------------------------------------------------------
# 执行封装
# ------------------------------------------------------------------------------
step() {
  echo ""
  echo "=================================================================="
  echo ">> $1"
  echo "=================================================================="
}

# apply 类命令：DRY_RUN 下追加 --dry-run=client 真正跑校验；其余命令仅打印
krun() {
  if [[ $DRY_RUN -eq 1 ]]; then
    if [[ "$1" == "apply" ]]; then
      echo "[DRY-RUN] kubectl $* --dry-run=client"
      command kubectl "$@" --dry-run=client
    else
      echo "[DRY-RUN] kubectl $*"
    fi
  else
    command kubectl "$@"
  fi
}

# 等待/状态类命令：DRY_RUN 下仅打印，不阻塞
kwait() {
  if [[ $DRY_RUN -eq 1 ]]; then
    echo "[DRY-RUN] (skip) kubectl $*"
  else
    command kubectl "$@"
  fi
}

# ------------------------------------------------------------------------------
# 执行流程
# ------------------------------------------------------------------------------

# 0/5 确保命名空间存在（00-namespace.yaml 声明了 metaclouds）
step "0/5 确保命名空间 ${NAMESPACE} 存在"
krun apply -f "${K8S_DIR}/00-namespace.yaml"

# 1/5 先部署 PostgreSQL（运行期前置之一）
step "1/5 部署 PostgreSQL StatefulSet（17-postgresql.yaml）"
krun apply -f "${K8S_DIR}/17-postgresql.yaml"

# 2/5 等 PostgreSQL Pod Ready（运行期前置之一，否则后端连不上库）
step "2/5 等待 PostgreSQL Pod Ready"
kwait rollout status statefulset/postgresql -n "${NAMESPACE}" --timeout=300s
kwait wait -n "${NAMESPACE}" pod -l "app=postgresql" --for=condition=Ready --timeout=300s

# 3/5 注入测试 secret（运行期前置之二：覆盖 02-secret 全部 key）
step "3/5 注入测试凭据（provision-test-secret.sh --apply）"
if [[ ! -f "${PROVISION_SECRET}" ]]; then
  echo "ERROR: 找不到 ${PROVISION_SECRET}" >&2
  exit 1
fi
if [[ $DRY_RUN -eq 1 ]]; then
  echo "[DRY-RUN] bash ${PROVISION_SECRET} --dry-run"
  bash "${PROVISION_SECRET}" --dry-run
else
  bash "${PROVISION_SECRET}" --apply
fi

# 4/5 整包部署（复用已有 apply 入口；支持 GIT_SHA 钉版本）
step "4/5 整包部署（deploy-k8s.sh / kubectl apply -k）"
if [[ $DRY_RUN -eq 1 ]]; then
  echo "[DRY-RUN] kubectl apply -k ${K8S_DIR} --dry-run=client"
  command kubectl apply -k "${K8S_DIR}" --dry-run=client
else
  if [[ -x "${DEPLOY_K8S}" ]]; then
    bash "${DEPLOY_K8S}"
  else
    krun apply -k "${K8S_DIR}"
  fi
fi

# ④.5 回填测试 secret：apply -k 会按 02-secret.yaml 的 CHANGEME 占位覆盖
#      metaclouds-secrets，必须再次 upsert 测试值，确保后端最终拿到真实 database-url。
#      （生产请用真实凭据注入替代此步。）
step "4.5/5 回填测试 secret（抵消 apply -k 对 02-secret 占位的覆盖）"
if [[ $DRY_RUN -eq 1 ]]; then
  echo "[DRY-RUN] bash ${PROVISION_SECRET} --dry-run"
  bash "${PROVISION_SECRET}" --dry-run
else
  bash "${PROVISION_SECRET}" --apply
fi

# ④.7 重启后端以拾取回填后的 secret（env 注入不热更新）
step "4.7/5 重启后端以拾取回填后的 secret（env 注入不热更新）"
if [[ $DRY_RUN -eq 1 ]]; then
  echo "[DRY-RUN] kubectl -n ${NAMESPACE} rollout restart deployment/metaclouds-backend"
  echo "[DRY-RUN] kubectl -n ${NAMESPACE} rollout status deployment/metaclouds-backend --timeout=300s"
else
  kubectl -n "${NAMESPACE}" rollout restart deployment/metaclouds-backend
  kubectl -n "${NAMESPACE}" rollout status deployment/metaclouds-backend --timeout=300s
fi

# 5/5 验证
step "5/5 验证工作负载与后端 /health"
krun get pods -n "${NAMESPACE}" -l "app=metaclouds"
if [[ $DRY_RUN -eq 0 ]]; then
  echo ">> 等待后端 Deployment 滚动完成..."
  kwait rollout status deployment/metaclouds-backend -n "${NAMESPACE}" --timeout=300s
  echo ">> 探针 /health（临时 kubectl port-forward 暴露 8000 端口抓取，最多等待 10s 不阻塞）"
  kubectl -n "${NAMESPACE}" port-forward svc/metaclouds-backend 18080:8000 >/dev/null 2>&1 &
  PF_PID=$!
  for _ in $(seq 1 10); do
    if curl -fsS http://127.0.0.1:18080/health >/dev/null 2>&1; then
      echo ">> /health 返回 200 OK"
      break
    fi
    sleep 1
  done
  kill "${PF_PID}" 2>/dev/null || true
fi

echo ""
echo ">> 完成。如需回滚：kubectl delete -k ${K8S_DIR} && bash ${PROVISION_SECRET} --delete"
