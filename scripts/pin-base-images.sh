#!/usr/bin/env bash
# ==============================================================================
# 基础镜像 digest 检查 / 注入（R6 遗留项；2026-11-10 第四轮重写）
#
# 背景：Dockerfile 的 FROM 必须保持**字面 tag**（2026-11-09 的 ARG 变量化方案
# 已回退——checkov 无法解析 ${ARG}，误报 CKV_DOCKER_7 "latest"）。
# digest 钉死改由本脚本在**构建前改写工作副本**实现（改动不提交）。
#
# 用法：
#   bash scripts/pin-base-images.sh --check             # FROM 是否均为字面 tag（非变量/latest）
#   bash scripts/pin-base-images.sh --check --strict    # 未钉 digest 则 exit 1（可作 CI 门禁）
#   bash scripts/pin-base-images.sh --resolve           # 联网解析 digest，输出 BASE_IMAGE_DIGESTS 格式
#   bash scripts/pin-base-images.sh --apply             # 按 BASE_IMAGE_DIGESTS 环境变量改写工作副本
#
# BASE_IMAGE_DIGESTS 格式（分号分隔；由 --resolve 输出或联网主机生成）：
#   rust:1.96-alpine=sha256:abc...;alpine:3.20=sha256:def...;node:22-alpine=sha256:...;nginx:1.27-alpine=sha256:...
# 解析需要能访问镜像仓库（docker buildx imagetools / crane / registry API）。
# 本仓库的离线 Windows 沙箱无法满足，脚本在工具/网络缺失时明确报错，**绝不伪造 digest**。
# ==============================================================================
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="check"
STRICT=0

while [ $# -gt 0 ]; do
  case "$1" in
    --check)   MODE="check"; shift ;;
    --resolve) MODE="resolve"; shift ;;
    --apply)   MODE="apply"; shift ;;
    --strict)  STRICT=1; shift ;;
    -h|--help) echo "Usage: $0 [--check [--strict] | --resolve | --apply]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# 镜像 ref | Dockerfile 路径（相对仓库根）
BASES=(
  "rust:1.96-alpine|metaclouds-backend-rust/Dockerfile"
  "alpine:3.20|metaclouds-backend-rust/Dockerfile"
  "node:22-alpine|metaclouds-frontend-vue/Dockerfile"
  "nginx:1.27-alpine|metaclouds-frontend-vue/Dockerfile"
)

if [ "$MODE" = "check" ]; then
  BAD=0; PINNED=0
  echo "=== Base image digest pinning ==="
  for spec in "${BASES[@]}"; do
    IFS='|' read -r ref path <<< "$spec"
    line="$(grep -m1 "^FROM ${ref%%:*}" "${REPO_ROOT}/${path}" 2>/dev/null || true)"
    if [ -z "$line" ]; then
      printf '  [FAIL] %-22s FROM %s not found in %s\n' "$ref" "$ref" "$path" >&2
      BAD=$((BAD + 1))
      continue
    fi
    actual="$(printf '%s' "$line" | awk '{print $2}')"
    base="${actual%%@sha256:*}"
    if [ "$base" != "$ref" ]; then
      printf '  [FAIL] %-22s %s: FROM is %s (expected literal %s)\n' "$ref" "$path" "$base" "$ref" >&2
      BAD=$((BAD + 1))
    elif printf '%s' "$actual" | grep -q '@sha256:'; then
      PINNED=$((PINNED + 1))
      printf '  [ OK ] %-24s pinned (%s)\n' "$ref" "${actual##*@}"
    else
      printf '  [WARN] %-23s tag only: %s\n' "$ref" "$actual"
    fi
  done
  printf '%d/4 base images are digest-pinned in the working tree.\n' "$PINNED"
  if [ "$BAD" -gt 0 ]; then
    echo "::error::FROM lines must stay literal tags (checkov CKV_DOCKER_7 cannot resolve variables)" >&2
    exit 1
  fi
  if [ "$PINNED" -lt 4 ]; then
    echo "Digests not pinned yet (resolution requires a networked environment)."
    echo "Resolve with: bash scripts/pin-base-images.sh --resolve"
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

if [ "$MODE" = "resolve" ]; then
  echo "=== Resolving base image digests (requires registry access) ==="
  FAILED=0
  DIGESTS=""
  for spec in "${BASES[@]}"; do
    IFS='|' read -r ref path <<< "$spec"
    if DIGEST="$(resolve_digest "$ref")"; then
      case "$DIGEST" in sha256:*) ;; *) echo "unexpected digest format for $ref: $DIGEST" >&2; FAILED=1; continue ;; esac
      [ -n "$DIGESTS" ] && DIGESTS="${DIGESTS};"
      DIGESTS="${DIGESTS}${ref}=${DIGEST}"
      printf '  [ OK ] %-24s %s\n' "$ref" "$DIGEST"
    else
      printf '  [FAIL] %-24s cannot resolve %s (docker buildx / crane unavailable or offline)\n' "$ref" "$ref" >&2
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
  echo "BASE_IMAGE_DIGESTS=${DIGESTS}"
  echo "(export it and run: bash scripts/pin-base-images.sh --apply)"
  exit 0
fi

# ---- apply -----------------------------------------------------------------
if [ -z "${BASE_IMAGE_DIGESTS:-}" ]; then
  echo "::error::BASE_IMAGE_DIGESTS is empty; get it with: bash scripts/pin-base-images.sh --resolve (networked host)" >&2
  exit 2
fi

echo "=== Applying base image digests to the working tree (not committed) ==="
FAILED=0
while IFS= read -r pair; do
  [ -z "$pair" ] && continue
  ref="${pair%%=*}"
  digest="${pair#*=}"
  target_file=""
  for spec in "${BASES[@]}"; do
    IFS='|' read -r bref bpath <<< "$spec"
    [ "$bref" = "$ref" ] && target_file="${REPO_ROOT}/${bpath}"
  done
  if [ -z "$target_file" ] || [ ! -f "$target_file" ]; then
    echo "::error::no Dockerfile maps to ${ref}" >&2
    FAILED=1
    continue
  fi
  case "$digest" in sha256:*) ;; *)
    echo "::error::invalid digest for ${ref}: ${digest}" >&2; FAILED=1; continue ;;
  esac
  if sed -i -E "s|^FROM ${ref}([[:space:]]+AS[[:space:]]+[A-Za-z0-9_-]+)?\$|FROM ${ref}@${digest}\1|" "$target_file" 2>/dev/null \
     && grep -qE "^FROM ${ref}@${digest}([[:space:]]+AS[[:space:]]+[A-Za-z0-9_-]+)?\$" "$target_file"; then
    printf '  [ OK ] %-24s pinned in %s\n' "$ref" "${target_file#${REPO_ROOT}/}"
  else
    echo "::error::failed to rewrite FROM ${ref} in ${target_file}" >&2
    FAILED=1
  fi
done <<< "$(printf '%s' "${BASE_IMAGE_DIGESTS}" | tr ';' '\n')"

if [ "$FAILED" -ne 0 ]; then exit 1; fi
echo "Working tree updated. Verify with: bash scripts/pin-base-images.sh --check"
exit 0
