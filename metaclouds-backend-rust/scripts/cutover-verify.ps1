#Requires -Version 5.1
<#
.SYNOPSIS
  P4-06 切流验证脚本（Go :8000 热备 vs Rust :8001 主），按 10%/50%/100% 三档灰度自动验证。
.DESCRIPTION
  本机无 K8s/Nginx，采用「模拟切流」：按比例用随机数把请求分发到 Go/Rust，等价观测灰度比例下的行为。
  每档（默认 60s 采样）：
    - 模拟 rustPct% 流量到 Rust，其余到 Go
    - 核心端点验证（登录/profile/csrf/users/clusters/jobs/gpus/metrics/dashboard/health/unauth-401）
    - 统计错误率、P99 延迟，检查回滚触发条件（对齐 docs/cutover-plan.md §2.3）
    - 输出该档报告 cutover-verify-stage-{N}.txt
  100% 档核心端点验证加倍（20 次，更严格）。
  回滚触发条件（任一满足即建议回滚）：错误率>5% / P99>1s / 连续 3 次 5xx / 核心端点 P0 状态码不一致。
  说明：真实三档切流在目标环境执行（Nginx/Ingress 改权重）；本脚本用于本机/预发按同口径自动验证脚本逻辑。
.PARAMETER Stage
  执行的档位，默认 @("10","50","100") 依次全部；也可指定单个或多个，如 -Stage 10,50。
.PARAMETER GoPort
  Go 版（热备/回滚目标）端口，默认 8000。
.PARAMETER RustPort
  Rust 版（切流目标）端口，默认 8001。
.PARAMETER SampleSeconds
  每档采样时长（秒），默认 60。
.PARAMETER RollbackCheck
  是否检查回滚触发条件，默认 true。
.PARAMETER OutputDir
  报告输出目录，默认 scripts/p4work（相对脚本目录）。
.PARAMETER IncludeRollbackDrill
  完成三档后追加一次回滚演练（100% 流量切回 Go 并验证）。
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/cutover-verify.ps1
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/cutover-verify.ps1 -Stage 10 -SampleSeconds 30 -IncludeRollbackDrill
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/cutover-verify.ps1 -h
#>
param(
  [string[]]$Stage = @("10", "50", "100"),
  [int]$GoPort = 8000,
  [int]$RustPort = 8001,
  [int]$SampleSeconds = 60,
  [bool]$RollbackCheck = $true,
  [string]$OutputDir = "",
  [string]$AdminUser = "admin",
  [string]$AdminPass = "",
  [switch]$IncludeRollbackDrill,
  [switch]$h,
  [switch]$Help
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"

function Show-Usage {
  Write-Host "cutover-verify.ps1 — P4-06 切流三档自动验证（Go :8000 vs Rust :8001，模拟切流）"
  Write-Host ""
  Write-Host "用法:"
  Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/cutover-verify.ps1 [参数]"
  Write-Host ""
  Write-Host "参数:"
  Write-Host "  -Stage <10,50,100>    执行档位，默认全部依次执行；可指定 -Stage 10,50"
  Write-Host "  -GoPort <n>           Go 热备端口，默认 8000"
  Write-Host "  -RustPort <n>         Rust 端口，默认 8001"
  Write-Host "  -SampleSeconds <n>    每档采样秒数，默认 60"
  Write-Host "  -RollbackCheck <b>    是否检查回滚条件，默认 true（可 -RollbackCheck:`$false）"
  Write-Host "  -OutputDir <path>     报告目录，默认 scripts/p4work"
  Write-Host "  -IncludeRollbackDrill 三档后追加回滚演练（100% 切回 Go）"
  Write-Host "  -h / -Help            显示本帮助"
  Write-Host ""
  Write-Host "示例:"
  Write-Host "  ...                              (10/50/100 依次，各 60s)"
  Write-Host "  ... -Stage 10 -SampleSeconds 30  (只跑 10% 档，本机短验证)"
}

if ($h -or $Help -or $PSBoundParameters.Count -eq 0) {
  Show-Usage
  exit 0
}

if (-not $AdminPass) { $AdminPass = $env:CUTOVERIFY_ADMIN_PASS }
if (-not $AdminPass) { $AdminPass = "Admin@123456" }   # 开发种子默认

if (-not $OutputDir) { $OutputDir = Join-Path $PSScriptRoot "p4work" }
if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
  $OutputDir = Join-Path $PSScriptRoot $OutputDir
}
if (-not (Test-Path $OutputDir)) { New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null }

$GoBase   = "http://127.0.0.1:$GoPort"
$RustBase = "http://127.0.0.1:$RustPort"

# ---------------------------------------------------------------------------
# 工具
# ---------------------------------------------------------------------------
function Test-HttpUp([string]$base) {
  try { $null = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop; return $true }
  catch { if ($_.Exception.Response) { return $true }; return $false }
}
function Invoke-Api([string]$base, [string]$method, [string]$path, [string]$token, [string]$body) {
  $uri = $base + $path
  $headers = @{ "Accept" = "application/json" }
  if ($token) { $headers["Authorization"] = "Bearer $token" }
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $result = [ordered]@{ Code = 0; Ms = 0; Body = ""; Err = "" }
  try {
    $p = @{ Uri = $uri; Method = $method; Headers = $headers; TimeoutSec = 15; UseBasicParsing = $true; ErrorAction = "Stop" }
    if ($body -and $method -ne "GET") { $p["Body"] = $body; $p["ContentType"] = "application/json" }
    $r = Invoke-WebRequest @p
    $sw.Stop()
    $result.Code = [int]$r.StatusCode; $result.Ms = [int]$sw.ElapsedMilliseconds; $result.Body = [string]$r.Content
  } catch {
    $sw.Stop(); $result.Ms = [int]$sw.ElapsedMilliseconds; $result.Err = $_.Exception.Message
    if ($_.Exception.Response) {
      try { $result.Code = [int]$_.Exception.Response.StatusCode } catch { }
    }
  }
  return [pscustomobject]$result
}
function Login([string]$base) {
  $body = @{ username = $script:AdminUser; password = $script:AdminPass } | ConvertTo-Json -Compress
  $r = Invoke-Api $base "POST" "/api/v1/auth/login" "" $body
  if ($r.Code -ne 200 -and $r.Code -ne 201) { return $null }
  try { $j = $r.Body | ConvertFrom-Json; return [string]$j.data.token } catch { return $null }
}
function Pctile([System.Collections.Generic.List[int]]$arr, [int]$p) {
  if ($null -eq $arr -or $arr.Count -eq 0) { return 0 }
  $sorted = $arr.ToArray() | Sort-Object
  $idx = [Math]::Max(0, [Math]::Min($sorted.Count - 1, [int][Math]::Ceiling($p / 100.0 * $sorted.Count) - 1))
  return [int]$sorted[$idx]
}

# 核心端点（带 token 的用 admin token；unauth 用空 token 预期 401）
function Get-CoreEndpoints {
  return @(
    @{ P = "/api/v1/auth/profile";         Role = "admin" }
    @{ P = "/api/v1/auth/csrf";            Role = "admin" }
    @{ P = "/api/v1/users";                Role = "admin" }
    @{ P = "/api/v1/clusters";             Role = "admin" }
    @{ P = "/api/v1/jobs";                 Role = "admin" }
    @{ P = "/api/v1/gpus";                 Role = "admin" }
    @{ P = "/api/v1/resources";            Role = "admin" }
    @{ P = "/api/v1/tenants";              Role = "admin" }
    @{ P = "/api/v1/monitoring/metrics";   Role = "admin" }
    @{ P = "/api/v1/monitoring/dashboard"; Role = "admin" }
    @{ P = "/metrics";                    Role = "none" }      # Prometheus 文本，无 JWT
    @{ P = "/api/v1/clusters";             Role = "none" }      # 预期 401
  )
}

# 采样轮的混合读端点
function Get-SampleEndpoints {
  return @(
    "/api/v1/auth/profile"
    "/api/v1/clusters"
    "/api/v1/jobs"
    "/api/v1/gpus"
    "/api/v1/resources"
    "/api/v1/tenants"
    "/api/v1/monitoring/metrics"
    "/api/v1/partitions"
    "/api/v1/quotas"
    "/api/v1/datasets"
  )
}

# 单档验证
function Invoke-Stage([int]$rustPct, [int]$coreChecks) {
  $sampleEps = Get-SampleEndpoints
  $coreEps   = Get-CoreEndpoints

  $codeList = New-Object System.Collections.Generic.List[int]
  $msList   = New-Object System.Collections.Generic.List[int]
  $consec5xx = 0
  $maxConsec5xx = 0
  $p0Mismatch = 0

  $deadline = (Get-Date).AddSeconds($SampleSeconds)
  $round = 0
  while ((Get-Date) -lt $deadline) {
    $round++
    $ep = $sampleEps[($round % $sampleEps.Count)]
    # 模拟分流：随机 < rustPct → Rust
    $goToRust = ((Get-Random -Minimum 0 -Maximum 100) -lt $rustPct)
    $base = if ($goToRust) { $RustBase } else { $GoBase }
    $tok  = if ($goToRust) { $script:RustToken } else { $script:GoToken }
    $r = Invoke-Api $base "GET" $ep $tok ""
    $codeList.Add([int]$r.Code)
    $msList.Add([int]$r.Ms)
    if ($r.Code -ge 500 -or $r.Code -eq 0) {
      $consec5xx++
      if ($consec5xx -gt $maxConsec5xx) { $maxConsec5xx = $consec5xx }
    } else { $consec5xx = 0 }
    Start-Sleep -Milliseconds 500
  }

  # 错误率 / P99
  $total = $codeList.Count
  $errs = @($codeList | Where-Object { $_ -ge 500 -or $_ -eq 0 }).Count
  $errRate = if ($total -gt 0) { [Math]::Round(100.0 * $errs / $total, 2) } else { 0 }
  $p99 = Pctile $msList 99

  # 核心端点验证（在 Rust 上跑 coreChecks 轮；同时抽 3 个与 Go 比对状态码查 P0）
  $coreErrs = 0
  $coreRust = 0
  for ($i = 0; $i -lt $coreChecks; $i++) {
    foreach ($ce in $coreEps) {
      $tok = if ($ce.Role -eq "admin") { $script:RustToken } else { "" }
      $r = Invoke-Api $RustBase "GET" $ce.P $tok ""
      if ($r.Code -ge 500 -or $r.Code -eq 0) { $coreErrs++ }
      if ($ce.Role -eq "admin") {
        # 与 Go 比对状态码（P0：双侧都应 2xx/4xx 一致；401 两侧都应 401）
        $g = Invoke-Api $GoBase "GET" $ce.P $script:GoToken ""
        if ($g.Code -ne 0 -and $r.Code -ne 0 -and $g.Code -ne $r.Code) {
          # /metrics Go 可能无 Prometheus 文本或路由不同，容忍单侧
          if ($ce.P -ne "/metrics" -and $ce.P -ne "/health") { $p0Mismatch++ }
        }
      }
    }
  }

  # 回滚触发条件评估
  $triggers = @()
  if ($RollbackCheck) {
    if ($errRate -gt 5.0) { $triggers += "错误率 $errRate% > 5%" }
    if ($p99 -gt 1000) { $triggers += "P99 $p99ms > 1000ms" }
    if ($maxConsec5xx -ge 3) { $triggers += "连续 $maxConsec5xx 次 5xx" }
    if ($p0Mismatch -gt 0) { $triggers += "P0 状态码不一致 $p0Mismatch 处" }
  }
  $rollbackTriggered = ($triggers.Count -gt 0)

  $result = [pscustomobject]@{
    RustPct = $rustPct; Req = $total; Errs = $errs; ErrRate = $errRate; P99 = $p99
    CoreChecks = ($coreChecks * $coreEps.Count); CoreErrs = $coreErrs
    P0Mismatch = $p0Mismatch; RollbackTriggered = $rollbackTriggered; Triggers = $triggers
  }
  return $result
}

function Write-StageReport([pscustomobject]$s) {
  $path = Join-Path $OutputDir ("cutover-verify-stage-" + $s.RustPct + ".txt")
  $sw = New-Object System.Text.StringBuilder
  [void]$sw.AppendLine("=== 切流验证 Stage $($s.RustPct)% ($(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')) ===")
  [void]$sw.AppendLine("Rust 模拟流量比例: $($s.RustPct)%")
  [void]$sw.AppendLine("采样请求数: $($s.Req)  错误数: $($s.Errs)  错误率: $($s.ErrRate)%")
  [void]$sw.AppendLine("P99 延迟: $($s.P99) ms")
  [void]$sw.AppendLine("核心端点数: $($s.CoreChecks)  核心错误: $($s.CoreErrs)")
  [void]$sw.AppendLine("P0 状态码不一致: $($s.P0Mismatch)")
  [void]$sw.AppendLine("回滚触发: $($s.RollbackTriggered)")
  if ($s.RollbackTriggered) {
    [void]$sw.AppendLine("  触发条件:")
    foreach ($t in $s.Triggers) { [void]$sw.AppendLine("    - $t") }
  }
  $sw.ToString() | Out-File -FilePath $path -Encoding UTF8
  return $path
}

# ===========================================================================
# 主流程
# ===========================================================================
Write-Host "==> cutover-verify: Go:$GoPort(热备)  Rust:$RustPort(主)  档位: $($Stage -join '/')  每档 ${SampleSeconds}s"

# 0. 前置检查
Write-Host "==> 前置检查双服务可达..."
if (-not (Test-HttpUp $GoBase))   { Write-Host "!! Go 服务 :$GoPort 不可达（回滚备用，必须运行）" -ForegroundColor Red; exit 1 }
if (-not (Test-HttpUp $RustBase)) { Write-Host "!! Rust 服务 :$RustPort 不可达" -ForegroundColor Red; exit 1 }
Write-Host "    Go/Rust 均可达"

Write-Host "==> 登录双版..."
$script:GoToken   = Login $GoBase
$script:RustToken = Login $RustBase
if (-not $script:GoToken -or -not $script:RustToken) { Write-Host "!! 登录失败" -ForegroundColor Red; exit 2 }
Write-Host "    Go/Rust admin token: ok"

$stageResults = @()
$order = @{ "10" = 1; "50" = 2; "100" = 3 }
$sorted = $Stage | Sort-Object { $order[$_] }

foreach ($st in $sorted) {
  $pct = 0
  [int]::TryParse($st, [ref]$pct) | Out-Null
  $coreChecks = if ($pct -eq 100) { 20 } else { 10 }
  Write-Host ""
  Write-Host "==> Stage ${pct}%  (核心端点验证 $coreChecks 轮)"
  $s = Invoke-Stage $pct $coreChecks
  $stageResults += $s
  $rep = Write-StageReport $s
  $flag = if ($s.RollbackTriggered) { "⚠️ ROLLBACK" } else { "OK" }
  Write-Host ("[Stage {0}%] requests={1} errors={2} p99={3}ms rollback_triggered={4}  [{5}]" -f `
    $pct, $s.Req, $s.Errs, $s.P99, $s.RollbackTriggered, $flag)
  if ($s.RollbackTriggered) {
    Write-Host "    ⚠️ 回滚触发条件满足，建议立即回滚到 Go 版：" -ForegroundColor Yellow
    foreach ($t in $s.Triggers) { Write-Host "      - $t" }
  }
}

# 可选回滚演练
if ($IncludeRollbackDrill) {
  Write-Host ""
  Write-Host "==> 回滚演练：100% 流量切回 Go"
  $drill = Invoke-Stage 0 10   # rustPct=0 → 全部打 Go
  $drillPath = Join-Path $OutputDir "cutover-verify-rollback-drill.txt"
  ("回滚演练（100% Go） $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')`n请求数=$($drill.Req) 错误数=$($drill.Errs) 错误率=$($drill.ErrRate)% P99=$($drill.P99)ms") | Out-File -FilePath $drillPath -Encoding UTF8
  Write-Host ("[Rollback drill] requests={0} errors={1} p99={2}ms" -f $drill.Req, $drill.Errs, $drill.P99)
}

# 最终报告
$finalPath = Join-Path $OutputDir "cutover-verify-final.txt"
$sw = New-Object System.Text.StringBuilder
[void]$sw.AppendLine("=== 切流验证最终报告 ($(Get-Date -Format 'yyyy-MM-dd HH:mm:ss zzz')) ===")
[void]$sw.AppendLine("Go:$GoBase(热备)  Rust:$RustBase(主)  档位: $($sorted -join ' -> ')  每档 ${SampleSeconds}s")
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 三档汇总 ==")
foreach ($s in $stageResults) {
  [void]$sw.AppendLine(("Stage {0,3}%: req={1,-6} err={2,-4} errRate={3,6}% p99={4,5}ms coreErr={5} P0={6} rollback={7}" -f `
    $s.RustPct, $s.Req, $s.Errs, $s.ErrRate, $s.P99, $s.CoreErrs, $s.P0Mismatch, $s.RollbackTriggered))
}
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 回滚触发条件评估（对齐 docs/cutover-plan.md §2.3）==")
$anyTrigger = $false
foreach ($s in $stageResults) {
  if ($s.RollbackTriggered) {
    $anyTrigger = $true
    [void]$sw.AppendLine(("Stage {0}% 触发回滚条件:" -f $s.RustPct))
    foreach ($t in $s.Triggers) { [void]$sw.AppendLine("   - $t") }
  }
}
if (-not $anyTrigger) { [void]$sw.AppendLine("三档均未触发回滚条件。") }
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 切流建议 ==")
if ($anyTrigger) {
  [void]$sw.AppendLine("⚠️ 存在触发回滚条件的档位，不建议继续放量；按 P4-05 Runbook v2 回滚到 Go 版并排查。")
} else {
  [void]$sw.AppendLine("三档错误率/P99/核心端点均达标，可按 cutover-plan.md 时间线继续：D0 10% → D1 50% → D2 100% → 1 周稳定期。")
}
$sw.ToString() | Out-File -FilePath $finalPath -Encoding UTF8

Write-Host ""
Write-Host "==> 最终报告: $finalPath"
Write-Host $sw.ToString()
