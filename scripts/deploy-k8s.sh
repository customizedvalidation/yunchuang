#!/usr/bin/env bash
# ==============================================================================
# deploy-k8s.sh — Metaclouds Kubernetes 部署入口（消除 ImagePullBackOff 的规范做法）
# ==============================================================================
# 背景：k8s 清单里的镜像 tag 由 kustomization 的 images.newTag 注入（默认 main）。
#   默认 kubectl apply -k 即可拉到 CI 推送的 main 分支 tag，避免此前 placeholder
#   占位 tag 导致的 ImagePullBackOff。
#
# 用法：
#   ./scripts/deploy-k8s.sh                 # 使用 kustomization 默认 tag（main，可漂移）
#   GIT_SHA=<commit-sha> ./scripts/deploy-k8s.sh   # 钉到具体构建（不可变，生产推荐）
#
# 前置：
#   1) kubectl 已配置目标集群上下文；
#   2) 已部署 17-postgresql StatefulSet，且 02-secret 的 database-url 为真实
#      postgresql://... 连接串（CI 门禁会拒绝 CHANGEME 占位）；
#   3) 已按 02-secret.yaml 注释创建 metaclouds-secrets（jwt-secret / redis-password /
#      default-admin-password / protected-endpoints-token / postgres-password）。
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
K8S_DIR="${SCRIPT_DIR}/metaclouds-backend-rust/k8s"
GIT_SHA="${GIT_SHA:-}"

if ! command -v kubectl >/dev/null 2>&1; then
  echo "ERROR: kubectl not found in PATH" >&2
  exit 1
fi

if [[ -n "$GIT_SHA" ]]; then
  echo ">> Deploying with pinned GIT_SHA=${GIT_SHA}"
  # 渲染清单并把镜像 tag 从 main 钉到不可变 sha（后端/前端两处都替换）
  kubectl kustomize "$K8S_DIR" | sed "s#:main#:${GIT_SHA}#g" | kubectl apply -f -
else
  echo ">> Deploying with kustomization default tag (main)"
  kubectl apply -k "$K8S_DIR"
fi

echo ">> Done. 验证：kubectl -n metaclouds get pods -l app=metaclouds,component=backend"
