#!/usr/bin/env bash
# ==============================================================================
# 部署后真实断言（P0-3，2026-11-09）
#
# 取代此前 CI 里的 `curl ... || echo "skipped (placeholder domain)"`：
# 那类写法永远返回 0，等于把「根本没验证」上报成绿色。本脚本对目标地址做
# 确定性断言，任何一步失败都以非 0 退出，让流水线真正反映部署结果。
#
# 用法： bash scripts/verify-deployment.sh <base-url>
#   <base-url> 由仓库变量 DEV_BASE_URL / STAGING_BASE_URL / PROD_BASE_URL 注入；
#   未配置时调用方应先 fail-fast，不要用占位域名兜底。
#
# 校验项：
#   1. /health/ready         —— 存活 + DB 就绪（200，最多重试 5 次，间隔 5s）
#   2. 就绪响应必须是后端 JSON 信封，而不是 SPA 回退的 index.html
#   3. /api/v1/system/mode   —— 公开端点，确认新版本已上线且信封结构正常
#
# 注意：compose 场景由 frontend 的 nginx.conf `location /health/` 反代到后端，
#       K8s 场景由 10-ingress.yaml 的 `/health` path 直达后端 Service。两处缺一，
#       本脚本都会因为拿到非后端响应而失败（而不是误判成功）。
# ==============================================================================
set -euo pipefail

assert_health_payload() {
  python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
    assert d.get("success") is True, "unexpected health envelope"
    data = d.get("data") or {}
    assert data.get("status") == "ok", "backend not ready: %r" % (data,)
except Exception as e:
    print("invalid /health/ready payload (SPA fallback or wrong service?): %s" % e, file=sys.stderr)
    sys.exit(1)
print("backend ready, version=%s uptime=%s" % (data.get("version"), data.get("uptime")))
'
}

BASE_URL="${1:-}"
if [ -z "$BASE_URL" ]; then
  echo "::error::verify-deployment.sh requires a non-empty <base-url>" >&2
  exit 2
fi
# 去掉尾部斜杠，避免 //health/ready
BASE_URL="${BASE_URL%/}"

echo "=== Verifying deployment at $BASE_URL ==="

HEALTHY=0
for i in 1 2 3 4 5; do
  if HEALTH_JSON="$(curl -fsS --max-time 10 "$BASE_URL/health/ready")" \
     && assert_health_payload <<< "$HEALTH_JSON"; then
    echo "health/ready OK (attempt $i)"
    HEALTHY=1
    break
  fi
  echo "health/ready attempt $i failed or returned a non-backend payload; retrying in 5s"
  sleep 5
done
if [ "$HEALTHY" -ne 1 ]; then
  echo "::error::health/ready did not return a valid backend ready envelope after 5 attempts — deployment verification FAILED" >&2
  exit 1
fi

MODE_JSON="$(curl -fsS --max-time 10 "$BASE_URL/api/v1/system/mode")" || {
  echo "::error::/api/v1/system/mode unreachable — deployed build is not the expected version" >&2
  exit 1
}
echo "$MODE_JSON"
python3 -c '
import json, sys
d = json.load(sys.stdin)
assert d.get("success") is True, "unexpected envelope: %r" % (d,)
data = d.get("data") or {}
assert "simulated" in data and "execution_mode" in data, "missing mode fields: %r" % (data,)
print("execution_mode=%s k8s_data_source=%s simulated=%s" % (
    data["execution_mode"], data["k8s_data_source"], data["simulated"]))
' <<< "$MODE_JSON" || {
  echo "::error::/api/v1/system/mode returned an invalid payload" >&2
  exit 1
}

echo "=== Deployment verification PASSED ==="
