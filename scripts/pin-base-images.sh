#!/usr/bin/env bash
# ==============================================================================
# 基础镜像 digest 解析 / 检查（P1-2 遗留项，2026-11-09 第三轮）
#
# 两个 Dockerfile 已把 4 个基础镜像参数化为全局 ARG（默认仍是 tag，行为零变化）：
#   metaclouds-backend-rust/Dockerfile   RUST_BUILDER_IMAGE / RUNTIME_IMAGE
#   metaclouds-frontend-vue/Dockerfile   NODE_BUILDER_IMAGE / NGINX_RUNTIME_IMAGE
#
# 用法：
#   bash scripts/pin-base-images.sh --check              # 是否已钉 digest（默认只告警）
#   bash scripts/pin-base-images.sh --check --strict     # 未钉则 exit 1（可作 CI 门禁）
#   bash scripts/pin-base-images.sh --resolve            # 联网解析 digest，输出 --build-arg
#
# 解析需要能访问镜像仓库（docker buildx imagetools 或 crane）。本仓库的离线
# Windows 沙箱无法满足，因此脚本在工具/网络缺失时明确报错，**绝不伪造 digest**。
# ==============================================================================
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="check"
STRICT=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check)   MODE="check"; shift ;;
    --resolve) MODE="resolve"; shift ;;
    --strict)  STRICT=1; shift ;;
    -h|--help) echo "Usage: $0 [--check [--strict] | --resolve]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# ARG 名 | 默认 tag 引用 | Dockerfile 路径
BASES=(
  "RUST_BUILDER_IMAGE|rust:1.96-alpine|metaclouds-backend-rust/Dockerfile"
  "RUNTIME_IMAGE|alpine:3.20|metaclouds-backend-rust/Dockerfile"
  "NODE_BUILDER_IMAGE|node:22-alpine|metaclouds-frontend-vue/Dockerfile"
  "NGINX_RUNTIME_IMAGE|nginx:1.27-alpine|metaclouds-frontend-vue/Dockerfile"
)

if [ "$MODE" = "check" ]; then
  UNPINNED=0
  echo "=== Base image digest pinning ==="
  for spec in "${BASES[@]}"; do
    IFS='|' read -r arg default path <<< "$spec"
    line="$(grep -m1 "^ARG ${arg}=" "${REPO_ROOT}/${path}" 2>/dev/null || true)"
    value="${line#*=}"
    if [ -z "$line" ]; then
      printf '  [FAIL] %-24s ARG not found in %s\n' "$arg" "$path"
      UNPINNED=$((UNPINNED + 1))
    elif printf '%s' "$value" | grep -q '@sha256:'; then
      printf '  [ OK ] %-24s pinned (%s)\n' "$arg" "${value##*@}"
    else
      printf '  [WARN] %-24s tag only: %s\n' "$arg" "$value"
      UNPINNED=$((UNPINNED + 1))
    fi
  done
  printf '\n%d/4 base images are digest-pinned.\n' "$((4 - UNPINNED))"
  if [ "$UNPINNED" -gt 0 ]; then
    echo "Resolve real digests with: bash scripts/pin-base-images.sh --resolve"
    echo "then replace the ARG defaults (or pass --build-arg in the build job)."
    if [ "$STRICT" -eq 1 ]; then
      echo "::error::base image digest pinning is incomplete" >&2
      exit 1
    fi
  fi
  exit 0
fi

# ---- resolve ---------------------------------------------------------------
resolve_digest() {
  local ref="$1" out=""
  if command -v docker >/dev/null 2>&1; then
    out="$(docker buildx imagetools inspect "$ref" 2>/dev/null | awk '/^Digest:/{print $2; exit}')"
    [ -n "$out" ] && { printf '%s' "$out"; return 0; }
  fi
  if command -v crane >/dev/null 2>&1; then
    out="$(crane digest "$ref" 2>/dev/null)"
    [ -n "$out" ] && { printf '%s' "$out"; return 0; }
  fi
  return 1
}

echo "=== Resolving base image digests (requires registry access) ==="
FAILED=0
for spec in "${BASES[@]}"; do
  IFS='|' read -r arg default path <<< "$spec"
  if DIGEST="$(resolve_digest "$default")"; then
    case "$DIGEST" in sha256:*) ;; *) echo "unexpected digest format for $default: $DIGEST" >&2; FAILED=1; continue ;; esac
    printf -- '--build-arg %s=%s@%s\n' "$arg" "$default" "$DIGEST"
  else
    printf '  [FAIL] %-24s cannot resolve %s (docker buildx / crane unavailable or offline)\n' "$arg" "$default" >&2
    FAILED=1
  fi
done

if [ "$FAILED" -ne 0 ]; then
  echo "" >&2
  echo "Cannot resolve all digests from this environment. Do NOT invent digests." >&2
  echo "Run this on a networked host with Docker (or install google/go-containerregistry's crane)." >&2
  exit 3
fi

echo ""
echo "Pass the four --build-arg flags to docker/build-push-action, or paste the"
echo "image@sha256 values into the ARG defaults and commit them for a hard pin."
