#!/usr/bin/env bash
# ==============================================================================
# 推送前自检（2026-11-09 第三轮）
#
# 背景：本仓库在离线 Windows 沙箱中开发，推送经常因代理层阻断失败
# （CONNECT tunnel failed 502 / schannel server closed abruptly），且本地门禁
# 全绿容易让人误以为远端已同步。本脚本把「能不能推、推之前还差什么」一次说清。
#
# 只读：不 push、不 commit、不改动任何文件。
#   bash scripts/pre-push-check.sh            # 报告，始终 exit 0
#   bash scripts/pre-push-check.sh --strict  # 工作区脏 / 网络不通时 exit 1
# ==============================================================================
set -uo pipefail

STRICT=0
BRANCH="main"
BUNDLE_DIR=".workbuddy"

while [ $# -gt 0 ]; do
  case "$1" in
    --strict)    STRICT=1; shift ;;
    --branch)    BRANCH="${2:-main}"; shift 2 ;;
    -h|--help)   echo "Usage: $0 [--strict] [--branch <name>]"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

FAIL=0
say_fail() { printf '  [FAIL] %s\n' "$1"; FAIL=$((FAIL + 1)); }
say_ok()   { printf '  [ OK ] %s\n' "$1"; }
say_info() { printf '  [INFO] %s\n' "$1"; }

echo "=== 1. Repository state ==="
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  say_fail "not a git work tree"
else
  CURRENT="$(git branch --show-current)"
  say_info "current branch: ${CURRENT:-<detached>}"
  [ "$CURRENT" = "$BRANCH" ] || say_info "note: expected branch $BRANCH"

  if [ -n "$(git status --porcelain)" ]; then
    say_info "working tree has uncommitted changes:"
    git status --short | sed 's/^/         /'
    say_fail "working tree is not clean (commit or stash before pushing)"
  else
    say_ok "working tree clean"
  fi

  HEAD_SHA="$(git rev-parse --short HEAD)"
  say_info "HEAD: $HEAD_SHA  $(git log -1 --format='%s')"

  if git rev-parse --verify "origin/$BRANCH" >/dev/null 2>&1; then
    AHEAD="$(git rev-list --count "origin/$BRANCH..$BRANCH")"
    BEHIND="$(git rev-list --count "$BRANCH..origin/$BRANCH")"
    say_info "ahead origin/$BRANCH by $AHEAD commit(s), behind by $BEHIND"
    [ "$BEHIND" -eq 0 ] || say_fail "branch has diverged (behind by $BEHIND) — integrate remote first"
  else
    say_info "no local origin/$BRANCH ref yet"
  fi
fi

echo ""
echo "=== 2. Deployment prerequisites (diagnostic only, never blocks a push) ==="
if [ -x scripts/check-deploy-prereqs.sh ] || [ -f scripts/check-deploy-prereqs.sh ]; then
  # --report 只打印缺失项，exit 0；部署未启用是正常状态，不是推送阻塞
  bash scripts/check-deploy-prereqs.sh --report >/dev/null 2>&1 \
    && say_ok "check-deploy-prereqs.sh --report executed" \
    || say_info "preflight reported missing deployment configuration (expected unless DEPLOY_ENABLED=true)"
  say_info "full catalog: bash scripts/check-deploy-prereqs.sh --list"
  say_info "runbook: docs/deployment-prerequisites.md"
else
  say_fail "scripts/check-deploy-prereqs.sh missing"
fi

echo ""
echo "=== 3. Helper script syntax ==="
for s in check-deploy-prereqs render-k8s-secret generate-deploy-secrets \
         verify-deployment pin-base-images pre-push-check; do
  if [ -f "scripts/${s}.sh" ]; then
    bash -n "scripts/${s}.sh" && say_ok "scripts/${s}.sh" || say_fail "scripts/${s}.sh has syntax errors"
  fi
done

echo ""
echo "=== 4. Remote reachability ==="
LS_REMOTE_OK=0
if command -v timeout >/dev/null 2>&1; then
  GIT_TERMINAL_PROMPT=0 timeout 20 git ls-remote --exit-code origin HEAD >/dev/null 2>&1 && LS_REMOTE_OK=1
else
  GIT_TERMINAL_PROMPT=0 git ls-remote --exit-code origin HEAD >/dev/null 2>&1 && LS_REMOTE_OK=1
fi
if [ "$LS_REMOTE_OK" -eq 1 ]; then
  say_ok "origin reachable"
  say_info "push with: git push origin $BRANCH"
else
  say_fail "origin unreachable from this sandbox (proxy/credential layer) — pushing here will fail"
fi

echo ""
echo "=== 5. Offline bundle fallback ==="
BUNDLES="$(find "$BUNDLE_DIR" -maxdepth 1 -name '*.bundle' -type f 2>/dev/null | sort)"
if [ -n "$BUNDLES" ]; then
  LATEST="$(printf '%s\n' "$BUNDLES" | tail -n 1)"
  if git bundle verify "$LATEST" >/dev/null 2>&1; then
    say_ok "$(basename "$LATEST") verifies OK ($(du -h "$LATEST" | cut -f1))"
  else
    say_fail "$(basename "$LATEST") failed git bundle verify"
  fi
else
  say_info "no offline bundle found"
fi
printf '%s\n' \
  "  refresh bundle:  git bundle create ${BUNDLE_DIR}/yunchuang-${BRANCH}-\$(date +%Y%m%d).bundle ${BRANCH}" \
  "  on a networked host:" \
  "    git fetch /path/to/<bundle> ${BRANCH} && git push origin FETCH_HEAD:${BRANCH}"

echo ""
if [ "$FAIL" -gt 0 ]; then
  printf '=== Pre-push check: %d blocking issue(s) ===\n' "$FAIL"
  if [ "$STRICT" -eq 1 ]; then exit 1; fi
  echo "report mode exits 0; use --strict to gate an automated push"
else
  echo "=== Pre-push check: all blocking checks passed ==="
fi
exit 0
