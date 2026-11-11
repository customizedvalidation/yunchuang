#!/usr/bin/env bash
# ==============================================================================
# 部署前置条件校验器（2026-11-09 第三轮）
#
# 背景：第二轮修复把「没配置就跳过并报绿」改成了「没配置就 exit 1」，但前置
# 条件只存在于文档表格里，且散落在 4 个 deploy job 各自手写的 3 行 Guard 中：
#   - 漏检 *_USER / DEPLOY_DIR / KUBE_CONFIG / K8s 真实凭据
#   - 只查「是否为空」，不查格式（example.com 占位域名、假 SSH key、假 kubeconfig）
#   - 未配置与「配置错」同样让整条流水线变红，推送前必须一次性配齐三套环境
#   - 02-secret.yaml 恒含 CHANGEME，Guard 会 100% 拒绝；即便绕过，apply -k
#     还会把集群里预置的真实 Secret 覆盖回占位值（见 render-k8s-secret.sh）
#
# 本脚本是唯一真源：
#   - 本地：bash scripts/check-deploy-prereqs.sh --report [--env-file .env.deploy]
#   - CI  ：bash scripts/check-deploy-prereqs.sh --ci（由 deploy-prereqs job 调用）
#   - 自助：bash scripts/check-deploy-prereqs.sh --list
#
# 状态机（CI）：
#   DEPLOY_ENABLED != true  -> 所有部署 job skipped，主构建链路保持绿，不误报红
#   DEPLOY_ENABLED == true  -> 仅校验 DEPLOY_TARGETS 列出的目标；缺一项即 exit 1
#   声明启用后配置齐全      -> 对应 target 的 job 才真正执行部署
#
# 安全：只打印变量名，绝不打印任何 secret 值（包括 kubeconfig 解码结果）。
# ==============================================================================
set -uo pipefail

MODE="report"
TARGET="all"
ENV_FILE=""
SHOW_LIST=0

usage() {
  cat <<'EOF'
Usage: check-deploy-prereqs.sh [options]

  --target <name>   development | staging | production | k8s | all (default: all)
  --report          仅报告，不因缺配置失败（默认；适合推送前自检）
  --strict          校验指定 target，任一前置条件不满足则 exit 1
  --verify-gh       通过 gh api 核验 GitHub 侧 Variables/Secrets 是否已配齐
                    （只列名称、绝不读取 secret 值；需要 gh 已登录）
  --ci              CI 模式：按 DEPLOY_ENABLED / DEPLOY_TARGETS 决定开关并写 $GITHUB_OUTPUT
  --env-file <path> 从本地 env 文件加载变量（该文件不得提交到 Git）
  --list            打印全部变量清单与用途后退出
  -h, --help        显示帮助
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --target)       TARGET="${2:-}"; shift 2 ;;
    --target=*)     TARGET="${1#*=}"; shift ;;
    --env-file)     ENV_FILE="${2:-}"; shift 2 ;;
    --env-file=*)   ENV_FILE="${1#*=}"; shift ;;
    --report)       MODE="report"; shift ;;
    --strict)       MODE="strict"; shift ;;
    --verify-gh)    MODE="verify-gh"; shift ;;
    --ci)           MODE="ci"; shift ;;
    --list)         SHOW_LIST=1; shift ;;
    -h|--help)      usage; exit 0 ;;
    *) echo "::error::unknown argument: $1" >&2; usage; exit 2 ;;
  esac
done

# ---------------------------------------------------------------------------
# 变量清单（与 docs/deployment-prerequisites.md 同源；--list 直接输出它）
# ---------------------------------------------------------------------------
cat_catalog() {
  cat <<'EOF'
[开关 · GitHub Variables]
  DEPLOY_ENABLED         true 才启用部署链路；未设置 = 全部部署 job skipped（不报红）
  DEPLOY_TARGETS         逗号分隔，默认 staging；可选 development,staging,production,k8s
  REQUIRE_PG_HA          可选，true 时把内置 PG 单点从 warning 升级为硬门禁

[development · GitHub Variables / Secrets]
  DEV_BASE_URL           (var)    部署后验收地址，须 http(s):// 且非 example.* 占位
  DEV_SSH_KEY            (secret) ed25519/RSA 私钥全文，含 -----BEGIN ... PRIVATE KEY-----
  DEV_HOST               (secret) 目标机 IP/域名
  DEV_USER               (secret) SSH 登录用户
  DEPLOY_DIR             (secret) 目标机仓库目录，默认 /opt/metaclouds

[staging]
  STAGING_BASE_URL       (var)
  STAGING_SSH_KEY        (secret)
  STAGING_HOST           (secret)
  STAGING_USER           (secret)
  DEPLOY_DIR             (secret) 同上，三个环境共用

[production · 仅 workflow_dispatch + 审批门触发]
  PROD_BASE_URL          (var)
  PROD_SSH_KEY           (secret)
  PROD_HOST              (secret)
  PROD_USER              (secret)
  DEPLOY_DIR             (secret)

[k8s · GitHub Secrets]
  KUBE_CONFIG            base64 编码的 kubeconfig（含 clusters/contexts）
  K8S_JWT_SECRET         JWT 签名密钥，>= 32 字符
  K8S_DATABASE_URL       postgresql:// 连接串，含真实口令且非 CHANGEME
  K8S_REDIS_PASSWORD     >= 16 字符
  K8S_DEFAULT_ADMIN_PASSWORD      >= 12 字符
  K8S_POSTGRES_PASSWORD          >= 16 字符
  K8S_PROTECTED_ENDPOINTS_TOKEN  >= 16 字符
  K8S_BASE_URL           (var, 可选) 仅用于 GitHub Environment URL 展示；
                         K8s 验收走 port-forward，不依赖该地址

[目标机预置（非 GitHub 配置，SSH 部署前必须人工完成）]
  docker login ghcr.io   GHCR 只读凭据（能拉 sha-<commit> 镜像）
  .env                   POSTGRES_* / REDIS_PASSWORD / DATABASE_URL(postgresql://) /
                         JWT_SECRET / DEFAULT_ADMIN_PASSWORD / ALLOW_SIMULATED_EXECUTION
  git pull --ff-only     目标机仓库可拉取（CI 只取编排文件，不在机上编译）
EOF
}

# ---------------------------------------------------------------------------
# --verify-gh：GitHub 侧配置存在性核验（2026-11-11 第八轮）
#
# 背景：--strict 读的是本地 --env-file，无法证明「值真的注入了 GitHub」；
# gh secret 也不能回读明文。本模式用 gh api 只列名称（绝不读取 secret 值）
# 核对期望键集，并可比对非敏感 Variables 的值是否与本地 env 文件一致。
# CI 的 --ci 模式（runner 把 secrets 注入环境变量后校验）仍是最终验收。
# ---------------------------------------------------------------------------
verify_gh_expected() {
  case "$1" in
    development)
      printf 'variable:DEV_BASE_URL\nsecret:DEV_SSH_KEY\nsecret:DEV_HOST\nsecret:DEV_USER\n' ;;
    staging)
      printf 'variable:STAGING_BASE_URL\nsecret:STAGING_SSH_KEY\nsecret:STAGING_HOST\nsecret:STAGING_USER\n' ;;
    production)
      printf 'variable:PROD_BASE_URL\nsecret:PROD_SSH_KEY\nsecret:PROD_HOST\nsecret:PROD_USER\n' ;;
    k8s)
      printf 'secret:KUBE_CONFIG\nsecret:K8S_JWT_SECRET\nsecret:K8S_DATABASE_URL\n'
      printf 'secret:K8S_REDIS_PASSWORD\nsecret:K8S_DEFAULT_ADMIN_PASSWORD\n'
      printf 'secret:K8S_POSTGRES_PASSWORD\nsecret:K8S_PROTECTED_ENDPOINTS_TOKEN\n' ;;
  esac
}

run_verify_gh() {
  if ! command -v gh >/dev/null 2>&1; then
    echo "::error::gh CLI not found - install GitHub CLI and run 'gh auth login' first" >&2
    exit 1
  fi
  if ! gh auth status >/dev/null 2>&1; then
    echo "::error::gh is not authenticated - run 'gh auth login' (or set GH_TOKEN with Variables/Secrets write access)" >&2
    printf 'Expected key set for target(s) %s (offline preview):\n' "$TARGET" >&2
    for t in $(resolve_targets "$TARGET"); do
      printf '  %-14s %s\n' "$t" "$(verify_gh_expected "$t" | tr '\n' ' ')" >&2
    done
    exit 1
  fi

  local var_lines sec_lines err vfile sfile
  vfile="/tmp/verify-gh-vars.$$.err"; sfile="/tmp/verify-gh-secs.$$.err"
  if ! var_lines="$(gh api --paginate 'repos/{owner}/{repo}/actions/variables?per_page=100' \
        --jq '.variables[] | "\(.name)\t\(.value)"' 2>"$vfile")"; then
    err="$(cat "$vfile" 2>/dev/null)"; rm -f "$vfile" "$sfile"
    echo "::error::cannot list GitHub Actions Variables (needs repo admin / Variables read): $err" >&2
    exit 1
  fi
  if ! sec_lines="$(gh api --paginate 'repos/{owner}/{repo}/actions/secrets?per_page=100' \
        --jq '.secrets[].name' 2>"$sfile")"; then
    err="$(cat "$sfile" 2>/dev/null)"; rm -f "$vfile" "$sfile"
    echo "::error::cannot list GitHub Actions Secrets (needs repo admin / Secrets read): $err" >&2
    exit 1
  fi
  rm -f "$vfile" "$sfile"

  declare -A GH_VARS=() GH_SECRETS=()
  local n v
  while IFS=$'\t' read -r n v; do
    [ -n "$n" ] && GH_VARS["$n"]="$v"
  done <<< "$var_lines"
  while IFS= read -r n; do
    [ -n "$n" ] && GH_SECRETS["$n"]=1
  done <<< "$sec_lines"

  printf '\n=== GitHub-side configuration (names only; secret values are NEVER read) ===\n'
  printf 'repository: %s\n' "$(gh repo view --json nameWithOwner -q .nameWithOwner 2>/dev/null || echo '{owner}/{repo}')"
  printf 'switches:   DEPLOY_ENABLED=%s | DEPLOY_TARGETS=%s\n' \
    "${GH_VARS[DEPLOY_ENABLED]:-<not set>}" "${GH_VARS[DEPLOY_TARGETS]:-<not set>}"

  local t kind name local_val gh_val
  for t in $(resolve_targets "$TARGET"); do
    printf '\n--- %s ---\n' "$t"
    while IFS=: read -r kind name; do
      [ -z "$name" ] && continue
      if [ "$kind" = variable ]; then
        gh_val="${GH_VARS[$name]:-}"
        if [ -z "$gh_val" ]; then
          fail "$name" "missing GitHub Variable" "gh variable set $name --body '<real-url>'"
          continue
        fi
        local_val="${!name:-}"
        if [ -n "${ENV_FILE:-}" ] && [ -n "$local_val" ] && [ "$local_val" != "$gh_val" ]; then
          fail "$name" "value differs from --env-file" "re-run: gh variable set $name --body <value from $ENV_FILE>"
        else
          pass "$name" "GitHub Variable present"
        fi
      else
        if [ -z "${GH_SECRETS[$name]:-}" ]; then
          fail "$name" "missing GitHub Secret" "gh secret set $name < <value-file>  (or --body)"
        else
          pass "$name" "GitHub Secret present (value not read)"
        fi
      fi
    done < <(verify_gh_expected "$t")
  done

  # 可选键只报告存在性，不算失败
  local opt
  for opt in DEPLOY_DIR K8S_BASE_URL REQUIRE_PG_HA; do
    if [ -n "${GH_SECRETS[$opt]:-}" ] || [ -n "${GH_VARS[$opt]:-}" ]; then
      printf '  [INFO] %-38s optional key present\n' "$opt"
    fi
  done
}

if [ "$SHOW_LIST" -eq 1 ]; then
  cat_catalog
  exit 0
fi

if [ -n "$ENV_FILE" ]; then
  if [ ! -f "$ENV_FILE" ]; then
    echo "::error::env file not found: $ENV_FILE" >&2
    exit 2
  fi
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

TOTAL=0
FAILED=0
declare -a PROBLEMS=()

pass() { TOTAL=$((TOTAL + 1)); printf '  [ OK ] %-38s %s\n' "$1" "$2"; }
fail() {
  TOTAL=$((TOTAL + 1)); FAILED=$((FAILED + 1))
  printf '  [FAIL] %-38s %s\n' "$1" "$2"
  PROBLEMS+=("$1 :: $3")
}

# 通用检查：全部只读取变量名，不回显值
check_present() {
  local name="$1" hint="$2" v="${!1:-}"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  pass "$name" "present"
  return 0
}

check_url() {
  local name="$1" hint="$2" v="${!1:-}"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  if ! printf '%s' "$v" | grep -Eq '^https?://[^/[:space:]]+'; then
    fail "$name" "must be an absolute http(s) URL" "$hint"; return 1
  fi
  case "$v" in
    *example.com*|*example.org*|*example.net*|*CHANGEME*|*placeholder*)
      fail "$name" "placeholder URL is not a deployable target" "$hint"; return 1 ;;
  esac
  pass "$name" "valid URL (placeholder-free)"
  return 0
}

check_ssh_key() {
  local name="$1" hint="$2" v="${!1:-}"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  if ! printf '%s' "$v" | grep -Eq -- '-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----'; then
    fail "$name" "does not look like a PEM private key" "$hint"; return 1
  fi
  pass "$name" "PEM private key"
  return 0
}

check_host() {
  local name="$1" hint="$2" v="${!1:-}"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  case "$v" in
    *example.*|*CHANGEME*|*placeholder*)
      fail "$name" "placeholder host" "$hint"; return 1 ;;
    *[[:space:]]*)
      fail "$name" "contains whitespace" "$hint"; return 1 ;;
  esac
  pass "$name" "valid host"
  return 0
}

check_user() {
  local name="$1" hint="$2" v="${!1:-}"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  case "$v" in
    *[[:space:]]*|*@*|*CHANGEME*)
      fail "$name" "must be a bare SSH username" "$hint"; return 1 ;;
  esac
  pass "$name" "valid SSH user"
  return 0
}

check_deploy_dir() {
  local name="DEPLOY_DIR" v="${DEPLOY_DIR:-}"
  if [ -z "$v" ]; then
    pass "$name" "not set -> using default /opt/metaclouds"
    return 0
  fi
  case "$v" in
    /*) pass "$name" "absolute path" ;;
    *)  fail "$name" "must be an absolute path" "gh secret set DEPLOY_DIR --body '/opt/metaclouds'" ;;
  esac
}

check_kubeconfig() {
  local name="KUBE_CONFIG" v="${KUBE_CONFIG:-}" decoded=""
  if [ -z "$v" ]; then
    fail "$name" "not set" "Linux: base64 -w0 ~/.kube/config | gh secret set KUBE_CONFIG"; return 1
  fi
  if ! decoded="$(printf '%s' "$v" | base64 -d 2>/dev/null)"; then
    fail "$name" "not valid base64" "re-encode with: base64 -w0 ~/.kube/config"; return 1
  fi
  # 只做结构探测，绝不打印 decoded（内含集群 CA / token / 客户端证书）
  if ! printf '%s' "$decoded" | grep -Eq 'apiVersion:|clusters:|"clusters"'; then
    fail "$name" "decoded content is not a kubeconfig" "ensure the file is a real kubeconfig before base64 encoding"; return 1
  fi
  pass "$name" "base64 kubeconfig (structure verified)"
  return 0
}

check_db_url() {
  local name="K8S_DATABASE_URL" v="${K8S_DATABASE_URL:-}"
  if [ -z "$v" ]; then
    fail "$name" "not set" "gh secret set K8S_DATABASE_URL --body 'postgresql://user:PASS@host:5432/db?sslmode=require'"; return 1
  fi
  case "$v" in
    *CHANGEME*|*placeholder*)
      fail "$name" "placeholder credential" "inject a real password"; return 1 ;;
  esac
  if ! printf '%s' "$v" | grep -Eq '^postgres(ql)?://[^:]+:[^@[:space:]]+@[^/[:space:]]+/[^[:space:]]+'; then
    fail "$name" "expected postgresql://user:password@host/db" "USE_SQLITE=false requires a real PostgreSQL URL"; return 1
  fi
  pass "$name" "PostgreSQL URL with credentials"
  return 0
}

check_secret_strength() {
  local name="$1" minlen="$2" v="${!1:-}" hint="gh secret set $1 (generate with: openssl rand -base64 32)"
  if [ -z "$v" ]; then fail "$name" "not set" "$hint"; return 1; fi
  case "$v" in
    *CHANGEME*|*placeholder*)
      fail "$name" "placeholder value" "$hint"; return 1 ;;
  esac
  if [ "${#v}" -lt "$minlen" ]; then
    fail "$name" "shorter than $minlen chars" "$hint"; return 1
  fi
  pass "$name" ">= $minlen chars"
  return 0
}

# ---------------------------------------------------------------------------
# 按目标分组
# ---------------------------------------------------------------------------
check_ssh_target() {
  local pfx="$1"
  printf '\n--- %s (compose / GHCR image) ---\n' "$pfx"
  check_url        "${pfx}_BASE_URL" "gh variable set ${pfx}_BASE_URL --body 'https://<real-host>'"
  check_ssh_key    "${pfx}_SSH_KEY"  "gh secret set ${pfx}_SSH_KEY < ~/.ssh/metaclouds_deploy"
  check_host       "${pfx}_HOST"     "gh secret set ${pfx}_HOST --body '<ip-or-host>'"
  check_user       "${pfx}_USER"     "gh secret set ${pfx}_USER --body 'deploy'"
  check_deploy_dir
}

check_k8s_target() {
  printf '\n--- k8s (kustomize apply / real Secret) ---\n'
  check_kubeconfig
  check_secret_strength K8S_JWT_SECRET                  32
  check_db_url
  check_secret_strength K8S_REDIS_PASSWORD             16
  check_secret_strength K8S_DEFAULT_ADMIN_PASSWORD     12
  check_secret_strength K8S_POSTGRES_PASSWORD           16
  check_secret_strength K8S_PROTECTED_ENDPOINTS_TOKEN   16
  if [ -z "${K8S_BASE_URL:-}" ]; then
    printf '  [INFO] %-38s %s\n' "K8S_BASE_URL" "optional: only used as GitHub Environment URL"
  else
    check_url "K8S_BASE_URL" "gh variable set K8S_BASE_URL --body 'https://<real-host>'"
  fi
}

check_target() {
  case "$1" in
    development) check_ssh_target DEV ;;
    staging)     check_ssh_target STAGING ;;
    production)  check_ssh_target PROD ;;
    k8s)         check_k8s_target ;;
    *) echo "::error::unknown target: $1" >&2; exit 2 ;;
  esac
}

ALL_TARGETS="development staging production k8s"
resolve_targets() {
  if [ "$1" = "all" ]; then printf '%s' "$ALL_TARGETS"; else printf '%s' "$1"; fi
}

write_output() {
  local key="$1" val="$2"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then
    printf '%s=%s\n' "$key" "$val" >> "$GITHUB_OUTPUT"
  fi
}

print_summary() {
  printf '\n=== Deploy prerequisites: %d checks, %d failure(s) ===\n' "$TOTAL" "$FAILED"
  if [ "$FAILED" -gt 0 ]; then
    printf '\nMissing / invalid configuration (names only, no values):\n'
    local p
    for p in "${PROBLEMS[@]}"; do printf '  - %s\n' "$p"; done
    printf '\nGenerate / inspect the full catalog: bash scripts/check-deploy-prereqs.sh --list\n'
  fi
}

# ---------------------------------------------------------------------------
# 模式分派
# ---------------------------------------------------------------------------
if [ "$MODE" = "verify-gh" ]; then
  run_verify_gh
  print_summary
  if [ "$FAILED" -gt 0 ]; then
    echo "::error::verify-gh FAILED ($FAILED missing/invalid GitHub-side prerequisite(s))" >&2
    echo "::notice::re-run 'generate-deploy-secrets.sh --from-env .env.deploy --all-gh-commands' to produce the full injection script" >&2
    exit 1
  fi
  echo "::notice::verify-gh PASSED - names present. Final acceptance is the CI --ci preflight after enabling DEPLOY_ENABLED."
  exit 0
fi

if [ "$MODE" = "ci" ]; then
  ENABLED="${DEPLOY_ENABLED:-}"
  TARGETS_CSV="${DEPLOY_TARGETS:-staging}"
  # 规范化：去空格，逗号分隔
  TARGETS_CSV="$(printf '%s' "$TARGETS_CSV" | tr -d '[:space:]')"

  # 不预先写全量 false：GITHUB_OUTPUT 同名 key 会重复出现，虽然 runner 取最后
  # 一个值，但日志与调试都会误导。未写入的 key 天然是空串，不等于 'true'。
  if [ "$ENABLED" != "true" ]; then
    write_output "enabled" "false"
    echo "::notice::DEPLOY_ENABLED is not 'true' — all deployment jobs will be SKIPPED (not green, not red)."
    echo "::notice::Configure variables/secrets first, then set DEPLOY_ENABLED=true and DEPLOY_TARGETS=<targets>."
    echo "See docs/deployment-prerequisites.md."
    # 未启用时仍输出诊断报告，便于运维一次性看到还差什么
    for t in $ALL_TARGETS; do check_target "$t"; done
    print_summary
    echo "::notice::diagnostic only — preflight exits 0 because deployments are disabled"
    exit 0
  fi

  echo "DEPLOY_ENABLED=true; enabled targets: $TARGETS_CSV"
  BAD_TARGET=0
  IFS=',' read -r -a REQUESTED <<< "$TARGETS_CSV"
  for t in "${REQUESTED[@]}"; do
    case " $ALL_TARGETS " in
      *" $t "*) ;;
      *) echo "::error::DEPLOY_TARGETS contains unknown target '$t' (allowed: $ALL_TARGETS)" >&2; BAD_TARGET=1 ;;
    esac
  done
  if [ "$BAD_TARGET" -ne 0 ]; then exit 2; fi

  # production-approval 依赖 deploy-staging 成功；只启用 production 会让生产
  # 链路静默 skipped。与其让人以为部署成功，不如在预检直接说明约束。
  case ",$TARGETS_CSV," in
    *,production,*)
      case ",$TARGETS_CSV," in
        *,staging,*) ;;
        *) echo "::error::DEPLOY_TARGETS includes production but not staging — the production approval gate needs a successful staging deploy" >&2
           echo "::error::set DEPLOY_TARGETS to include staging (e.g. staging,production)" >&2
           exit 2 ;;
      esac ;;
  esac

  # 逐目标校验；失败先记下，最后统一判定，保证一次报全
  declare -a TARGET_STATUS=()
  for t in "${REQUESTED[@]}"; do
    BEFORE=$FAILED
    check_target "$t"
    if [ "$FAILED" -eq "$BEFORE" ]; then TARGET_STATUS+=("$t:true"); else TARGET_STATUS+=("$t:false"); fi
  done

  print_summary
  if [ "$FAILED" -gt 0 ]; then
    echo "::error::DEPLOY_ENABLED=true but prerequisites are incomplete — refusing to run a partial deployment"
    exit 1
  fi
  for s in "${TARGET_STATUS[@]}"; do write_output "${s%%:*}" "${s##*:}"; done
  write_output "enabled" "true"
  echo "::notice::preflight PASSED for: $TARGETS_CSV"
  exit 0
fi

for t in $(resolve_targets "$TARGET"); do check_target "$t"; done
print_summary

if [ "$MODE" = "strict" ] && [ "$FAILED" -gt 0 ]; then
  echo "::error::strict preflight FAILED ($FAILED missing/invalid prerequisite(s))" >&2
  exit 1
fi
if [ "$MODE" = "report" ] && [ "$FAILED" -gt 0 ]; then
  printf '\nNOTE: report mode exits 0. To gate a push/CI run, use --strict or set DEPLOY_ENABLED=true with --ci.\n'
fi
exit 0
