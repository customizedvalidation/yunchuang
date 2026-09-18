#Requires -Version 5.1
<#
.SYNOPSIS
  P4-02 长时间影子观察脚本（Go :8000 vs Rust :8001），支持连续运行（默认 24h，可设 5 工作日）。
.DESCRIPTION
  与 scripts/shadow-compare.ps1 同源（复用其双服务启动、登录、40 条混合请求集、信封/形状/diff 分级），
  区别在面向长时间运行：
    - 可配置采样时长（-DurationMinutes，默认 1440=24h；5 个工作日约 7200）与轮次间隔（-IntervalSeconds，默认 30s）
    - 按轮次定时双发同一混合请求集，逐请求对比状态码 / 响应体 SHA256 / 信封键集 / data 形状 / 耗时
    - diff 分级：P0 状态码不一致（白名单外）/ P1 信封或形状不一致 / P1-KNOWN-EMPTY 空集合 null vs []
                 / P2 Rust 延迟 > Go 延迟×3 且绝对差 >100ms / P3 可接受差异
    - 实时写 diff 日志（shadow-observe-yyyyMMdd-HHmm.log），逐轮控制台统计，结束写汇总（shadow-observe-summary.txt）
    - 长时间运行自动刷新 token（默认每 2 小时重登；遇到 401 也触发重登）
    - Ctrl-C 优雅停止：停止采样、输出最终统计；默认不停止双服务（-StopServicesOnExit 才停本脚本启动的进程）
  说明：长时间运行 / 真实 5 工作日观察在目标环境执行；本机用于脚本逻辑验证（短时长即可）。
.PARAMETER DurationMinutes
  采样总时长（分钟），默认 1440（24 小时）。5 个工作日连续观察约设 7200。
.PARAMETER IntervalSeconds
  每轮采样之间的间隔秒数，默认 30。
.PARAMETER GoPort
  Go 版端口，默认 8000。
.PARAMETER RustPort
  Rust 版端口，默认 8001。
.PARAMETER RequestSet
  请求集：mixed（默认，40 条混合含写探针）/ read-only（仅 GET，不写数据，适合长跑）/ full（mixed + obs_ 前缀写探针）。
.PARAMETER OutputDir
  日志与汇总输出目录，默认 scripts/p4work（相对脚本目录）。
.PARAMETER AdminPass
  admin 密码。不传则读环境变量 OBSERVE_ADMIN_PASS；仍为空则用开发种子默认 Admin@123456（与服务启动 DEFAULT_ADMIN_PASSWORD 一致）。
.PARAMETER StopServicesOnExit
  退出时停止本脚本启动的 Go/Rust 进程；默认保留双服务继续运行。
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 -DurationMinutes 7200 -IntervalSeconds 30 -RequestSet read-only
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 -h
#>
param(
  [int]$DurationMinutes = 1440,
  [int]$IntervalSeconds = 30,
  [int]$GoPort = 8000,
  [int]$RustPort = 8001,
  [ValidateSet("mixed", "read-only", "full")]
  [string]$RequestSet = "mixed",
  [string]$OutputDir = "",
  [string]$GoDir = "D:\YCYD\metaclouds-backend",
  [string]$RustDir = "D:\YCYD\metaclouds-backend-rust",
  [string]$AdminUser = "admin",
  [string]$AdminPass = "",
  [string]$TestUser = "shadowobs",
  [string]$TestPass = "User@123456",
  [switch]$StopServicesOnExit,
  [switch]$h,
  [switch]$Help
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"

# ---------------------------------------------------------------------------
# 帮助
# ---------------------------------------------------------------------------
function Show-Usage {
  Write-Host "shadow-observe.ps1 — P4-02 长时间影子观察（Go :8000 vs Rust :8001）"
  Write-Host ""
  Write-Host "用法:"
  Write-Host "  powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 [参数]"
  Write-Host ""
  Write-Host "参数:"
  Write-Host "  -DurationMinutes <n>   采样总时长(分钟)，默认 1440(24h)；5 工作日约 7200"
  Write-Host "  -IntervalSeconds <n>   每轮间隔(秒)，默认 30"
  Write-Host "  -GoPort <n>            Go 端口，默认 8000"
  Write-Host "  -RustPort <n>          Rust 端口，默认 8001"
  Write-Host "  -RequestSet <name>     mixed(默认) | read-only(仅GET) | full(含写探针)"
  Write-Host "  -OutputDir <path>      输出目录，默认 scripts/p4work"
  Write-Host "  -AdminPass <pwd>       admin 密码(或设环境变量 OBSERVE_ADMIN_PASS)"
  Write-Host "  -StopServicesOnExit    退出时停止本脚本启动的双服务进程"
  Write-Host "  -h / -Help             显示本帮助"
  Write-Host ""
  Write-Host "示例:"
  Write-Host "  ... -DurationMinutes 5 -RequestSet read-only   (本机短验证)"
  Write-Host "  ... -DurationMinutes 7200 -IntervalSeconds 30   (目标环境 5 工作日)"
}

if ($h -or $Help -or $PSBoundParameters.Count -eq 0) {
  Show-Usage
  exit 0
}

# 密码：参数 > 环境变量 > 开发种子默认
if (-not $AdminPass) { $AdminPass = $env:OBSERVE_ADMIN_PASS }
if (-not $AdminPass) { $AdminPass = "Admin@123456" }   # 开发种子，与服务启动 DEFAULT_ADMIN_PASSWORD 对齐

# 输出目录：默认相对脚本目录
if (-not $OutputDir) { $OutputDir = Join-Path $PSScriptRoot "p4work" }
if (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
  $OutputDir = Join-Path $PSScriptRoot $OutputDir
}
if (-not (Test-Path $OutputDir)) { New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null }

$GoBase   = "http://127.0.0.1:$GoPort"
$RustBase = "http://127.0.0.1:$RustPort"

$stamp       = Get-Date -Format "yyyyMMdd-HHmm"
$LogPath     = Join-Path $OutputDir ("shadow-observe-" + $stamp + ".log")
$SummaryPath = Join-Path $OutputDir "shadow-observe-summary.txt"
$GoOutLog    = Join-Path $OutputDir "shadow-observe-go-out.log"
$GoErrLog    = Join-Path $OutputDir "shadow-observe-go-err.log"
$RustOutLog  = Join-Path $OutputDir "shadow-observe-rust-out.log"
$RustErrLog  = Join-Path $OutputDir "shadow-observe-rust-err.log"

$script:StartedGo   = $null
$script:StartedRust = $null

# token 与登录时间（用于长跑刷新）
$script:GoAdmin = $null
$script:RustAdmin = $null
$script:GoUser = $null
$script:RustUser = $null
$script:TokenTime = [DateTime]::MinValue
$script:TokenRefreshAfter = [TimeSpan]::FromHours(2)

# 统计计数器
$script:Total = 0
$script:Round = 0
$script:P0 = 0
$script:P1 = 0
$script:P1KnownEmpty = 0
$script:P2 = 0
$script:P3 = 0
$script:RustErr = 0
$script:GoErr = 0
$script:RustMsList = New-Object System.Collections.Generic.List[int]
$script:GoMsList = New-Object System.Collections.Generic.List[int]
$script:LogBuffer = New-Object System.Collections.Generic.List[string]

# ---------------------------------------------------------------------------
# 工具函数
# ---------------------------------------------------------------------------
function Test-HttpUp([string]$base) {
  try {
    $null = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
    return $true
  } catch {
    if ($_.Exception.Response) { return $true }
    return $false
  }
}

function Wait-Ready([string]$base, [int]$maxSeconds = 90) {
  $deadline = (Get-Date).AddSeconds($maxSeconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-HttpUp $base) {
      try {
        $null = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 3 -UseBasicParsing -ErrorAction Stop
        return $true
      } catch {
        if (Test-HttpUp $base) { return $true }
      }
    }
    Start-Sleep -Milliseconds 500
  }
  return $false
}

function Stop-TrackedProcess($proc) {
  if ($null -eq $proc) { return }
  try {
    $cmd = "taskkill /PID $($proc.Id) /T /F 2>`$null"
    Invoke-Expression $cmd | Out-Null
  } catch { }
}

# ---------------------------------------------------------------------------
# HTTP 调用 + 体哈希
# ---------------------------------------------------------------------------
function Invoke-Api([string]$base, [string]$method, [string]$path, [string]$token, [string]$body) {
  $uri = $base + $path
  $headers = @{ "Accept" = "application/json" }
  if ($token) { $headers["Authorization"] = "Bearer $token" }
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $result = [ordered]@{ Code = 0; Ms = 0; Body = ""; Sha = ""; Err = "" }
  try {
    $p = @{ Uri = $uri; Method = $method; Headers = $headers; TimeoutSec = 15; UseBasicParsing = $true; ErrorAction = "Stop" }
    if ($body -and $method -ne "GET") { $p["Body"] = $body; $p["ContentType"] = "application/json" }
    $r = Invoke-WebRequest @p
    $sw.Stop()
    $result.Code = [int]$r.StatusCode
    $result.Ms = [int]$sw.ElapsedMilliseconds
    $result.Body = [string]$r.Content
  } catch {
    $sw.Stop()
    $result.Ms = [int]$sw.ElapsedMilliseconds
    $result.Err = $_.Exception.Message
    if ($_.Exception.Response) {
      try {
        $result.Code = [int]$_.Exception.Response.StatusCode
        $sr = New-Object IO.StreamReader($_.Exception.Response.GetResponseStream())
        $result.Body = $sr.ReadToEnd()
      } catch { }
    }
  }
  if ($result.Body) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($result.Body)
    $hash = $sha.ComputeHash($bytes)
    $result.Sha = ([System.BitConverter]::ToString($hash) -replace "-", "").Substring(0, 16).ToLower()
  }
  return [pscustomobject]$result
}

function Login-Admin([string]$base) {
  $body = @{ username = $script:AdminUser; password = $script:AdminPass } | ConvertTo-Json -Compress
  $r = Invoke-Api $base "POST" "/api/v1/auth/login" "" $body
  if ($r.Code -ne 200 -and $r.Code -ne 201) { return $null }
  try { $j = $r.Body | ConvertFrom-Json; return [string]$j.data.token } catch { return $null }
}

function Get-UserToken([string]$base, [string]$adminToken, [string]$which) {
  if ($which -eq "go") {
    $reg = @{ username = $script:TestUser; password = $script:TestPass; email = "$($script:TestUser)@example.com" } | ConvertTo-Json -Compress
    $null = Invoke-Api $base "POST" "/api/v1/auth/register" "" $reg
  } else {
    $u = @{ username = $script:TestUser; password = $script:TestPass; email = "$($script:TestUser)@example.com"; role = "user" } | ConvertTo-Json -Compress
    $null = Invoke-Api $base "POST" "/api/v1/users" $adminToken $u
  }
  $login = @{ username = $script:TestUser; password = $script:TestPass } | ConvertTo-Json -Compress
  $r = Invoke-Api $base "POST" "/api/v1/auth/login" "" $login
  try { $j = $r.Body | ConvertFrom-Json; return [string]$j.data.token } catch { return "" }
}

# 登录双版（长跑定期刷新）
function Ensure-Login {
  $now = Get-Date
  if ($script:GoAdmin -and $script:RustAdmin -and (($now - $script:TokenTime) -lt $script:TokenRefreshAfter)) { return $true }
  $g = Login-Admin $GoBase
  $r = Login-Admin $RustBase
  if (-not $g -or -not $r) { return $false }
  $script:GoAdmin = $g
  $script:RustAdmin = $r
  $script:GoUser = Get-UserToken $GoBase $g "go"
  $script:RustUser = Get-UserToken $RustBase $r "rust"
  $script:TokenTime = Get-Date
  return $true
}

# ---------------------------------------------------------------------------
# 混合请求集（与 shadow-compare.ps1 同源 40 条）
# ---------------------------------------------------------------------------
function Get-BaseRequestSet {
  $EmptyBody = "{}"
  return @(
    @{ M = "GET";  P = "/health";                          B = "";         Role = "none" }
    @{ M = "GET";  P = "/api/v1/auth/profile";             B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/auth/csrf";               B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/clusters";                B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/clusters?page=1&page_size=5"; B = "";     Role = "admin" }
    @{ M = "GET";  P = "/api/v1/resources";               B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/tenants";                  B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/jobs";                    B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/jobs?status=running";     B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/gpus";                    B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/gpus/allocations";         B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/gpus/utilization";        B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/partitions";               B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/quotas";                   B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/schedulers";               B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/datasets";                B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/checkpoints";             B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/acceleration";             B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/security/policies";        B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/monitoring/metrics";      B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/topology";                 B = "";         Role = "admin" }
    @{ M = "POST"; P = "/api/v1/clusters";                B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/tenants";                 B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/jobs";                    B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/gpus";                    B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/partitions";              B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/quotas";                  B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/schedulers";              B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/datasets";                B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/checkpoints";            B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/acceleration";             B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/security/policies";       B = $EmptyBody; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/quotas/check";           B = $EmptyBody; Role = "admin" }
    @{ M = "GET";  P = "/api/v1/clusters/1";              B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/jobs/1";                  B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/gpus/1";                  B = "";         Role = "admin" }
    @{ M = "GET";  P = "/api/v1/clusters";                B = "";         Role = "none" }
    @{ M = "GET";  P = "/api/v1/auth/profile";            B = "";         Role = "none" }
    @{ M = "POST"; P = "/api/v1/clusters";                B = $EmptyBody; Role = "user" }
    @{ M = "POST"; P = "/api/v1/tenants";                B = $EmptyBody; Role = "user" }
  )
}

# full 模式额外写探针（obs_ 测试前缀，尽量不污染业务数据；失败 422 也属预期探针）
function Get-WriteProbes {
  $probe = '{"name":"observe_probe"}'
  return @(
    @{ M = "POST"; P = "/api/v1/clusters"; B = $probe; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/tenants";  B = $probe; Role = "admin" }
    @{ M = "POST"; P = "/api/v1/datasets"; B = $probe; Role = "admin" }
  )
}

function Build-RequestSet([string]$mode) {
  $base = Get-BaseRequestSet
  switch ($mode) {
    "read-only" { return @($base | Where-Object { $_.M -eq "GET" }) }
    "full"      { return @($base + (Get-WriteProbes)) }
    default     { return @($base) }   # mixed
  }
}

# ---------------------------------------------------------------------------
# 信封 / 形状 / diff 分级（复用 shadow-compare.ps1 逻辑）
# ---------------------------------------------------------------------------
function Get-EnvKeys($body) {
  try { $j = $body | ConvertFrom-Json; return ($j.PSObject.Properties.Name | Sort-Object) -join "," }
  catch { return "non-json" }
}
function Get-Shape($node, $depth = 0) {
  if ($depth -gt 4) { return "..." }
  if ($null -eq $node) { return "null" }
  if ($node -is [array]) {
    if ($node.Count -eq 0) { return "[]" } else { return "[ " + (Get-Shape $node[0] ($depth + 1)) + " ]" }
  }
  if ($node -is [pscustomobject]) { return "{" + (($node.PSObject.Properties.Name | Sort-Object) -join ",") + "}" }
  if ($node -is [bool]) { return "bool" }
  if (($node -is [int]) -or ($node -is [long]) -or ($node -is [double]) -or ($node -is [decimal])) { return "num" }
  if ($node -is [string]) { return "str" }
  return $node.GetType().Name
}
function Get-DataShape($body) {
  try { $j = $body | ConvertFrom-Json; if ($null -eq $j) { return "null" }; return Get-Shape $j.data 0 }
  catch { return "non-json" }
}
function Get-SuccessBool($body) {
  try { $j = $body | ConvertFrom-Json; if ($null -ne $j.success) { return [bool]$j.success } } catch { }
  return $null
}
function ShapeNorm($s) {
  if ($s -eq "null") { return "null" }
  if ($s.StartsWith("[")) { return "array" }
  return $s
}
function Is-KnownP3([string]$method, [string]$path, [int]$gCode, [int]$rCode) {
  if ($path -eq "/health" -and $gCode -eq 200 -and $rCode -eq 404) { return $true }
  if ($method -eq "POST" -and $gCode -ne $rCode -and $rCode -eq 422) {
    $gOk = (($gCode -ge 200 -and $gCode -lt 300) -or ($gCode -ge 400 -and $gCode -lt 500))
    if ($gOk) { return $true }
  }
  if ($path -eq "/api/v1/gpus/utilization" -and $gCode -eq 200 -and $rCode -eq 200) { return $true }
  if ($method -eq "GET" -and $path -match "^/api/v1/(clusters|jobs|gpus)/\d+$") {
    if (($gCode -eq 200 -and $rCode -eq 404) -or ($gCode -eq 404 -and $rCode -eq 200)) { return $true }
  }
  return $false
}

# 分级：P0 / P1 / P1-KNOWN-EMPTY / P2 / P3
function Classify-Diff([string]$method, [string]$path, [pscustomobject]$g, [pscustomobject]$r) {
  if ($g.Code -ne $r.Code) {
    if (Is-KnownP3 $method $path $g.Code $r.Code) { return "P3" }
    return "P0"
  }
  $gS = Get-SuccessBool $g.Body
  $rS = Get-SuccessBool $r.Body
  if ($null -ne $gS -and $null -ne $rS -and $gS -ne $rS) { return "P1" }
  $gKeys = Get-EnvKeys $g.Body
  $rKeys = Get-EnvKeys $r.Body
  if ($gKeys -ne $rKeys) {
    if ($path -eq "/api/v1/auth/logout") { return "P3" }
    return "P1"
  }
  if ($gS -eq $true -and $rS -eq $true) {
    $gShape = Get-DataShape $g.Body
    $rShape = Get-DataShape $r.Body
    if ((ShapeNorm $gShape) -ne (ShapeNorm $rShape)) {
      if ($path -eq "/api/v1/monitoring/metrics" -or $path -eq "/api/v1/gpus/utilization") { return "P3" }
      if ((ShapeNorm $gShape) -eq "null" -and (ShapeNorm $rShape) -eq "array") { return "P1-KNOWN-EMPTY" }
      return "P1"
    }
  }
  # P2: Rust 延迟 > Go 延迟×3 且绝对差 >100ms（长跑阈值；基线 <5ms 噪声忽略）
  if ($g.Ms -ge 5 -and $r.Ms -gt ($g.Ms * 3) -and ($r.Ms - $g.Ms) -gt 100) { return "P2" }
  return "P3"
}

# ---------------------------------------------------------------------------
# 日志缓冲落盘
# ---------------------------------------------------------------------------
function Flush-Log {
  if ($script:LogBuffer.Count -eq 0) { return }
  $script:LogBuffer | Out-File -FilePath $LogPath -Append -Encoding UTF8
  $script:LogBuffer.Clear()
}
function Add-LogLine([string]$line) {
  $script:LogBuffer.Add($line)
  if ($script:LogBuffer.Count -ge 50) { Flush-Log }
}

function Pctile([System.Collections.Generic.List[int]]$arr, [int]$p) {
  if ($null -eq $arr -or $arr.Count -eq 0) { return 0 }
  $sorted = $arr.ToArray() | Sort-Object
  $idx = [Math]::Max(0, [Math]::Min($sorted.Count - 1, [int][Math]::Ceiling($p / 100.0 * $sorted.Count) - 1))
  return [int]$sorted[$idx]
}

# ---------------------------------------------------------------------------
# 最终统计输出（正常退出与 Ctrl-C 均会调用）
# ---------------------------------------------------------------------------
function Write-FinalSummary {
  Flush-Log
  $total = $script:Total
  $goAvg = 0; $rustAvg = 0
  if ($script:GoMsList.Count -gt 0)   { $goAvg   = [Math]::Round(($script:GoMsList   | Measure-Object -Average).Average, 1) }
  if ($script:RustMsList.Count -gt 0) { $rustAvg = [Math]::Round(($script:RustMsList | Measure-Object -Average).Average, 1) }
  $goP99   = Pctile $script:GoMsList 99
  $rustP99 = Pctile $script:RustMsList 99
  $rustErrRate = if ($total -gt 0) { [Math]::Round(100.0 * $script:RustErr / $total, 3) } else { 0 }

  $sw = New-Object System.Text.StringBuilder
  [void]$sw.AppendLine("P4-02 影子长时间观察汇总 (shadow-observe)")
  [void]$sw.AppendLine("生成时间: " + (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz"))
  [void]$sw.AppendLine("Go: $GoBase  Rust: $RustBase  请求集: $RequestSet")
  [void]$sw.AppendLine("起止: " + $script:StartTime.ToString("yyyy-MM-dd HH:mm:ss") + " ~ " + (Get-Date).ToString("yyyy-MM-dd HH:mm:ss"))
  [void]$sw.AppendLine("")
  [void]$sw.AppendLine("== 总览 ==")
  [void]$sw.AppendLine("总请求数:    $total")
  [void]$sw.AppendLine("轮次:        $($script:Round)")
  [void]$sw.AppendLine("P0 diff 数:  $($script:P0)")
  [void]$sw.AppendLine("P1 diff 数:  $($script:P1)  (已知遗留 P1-KNOWN-EMPTY: $($script:P1KnownEmpty))")
  [void]$sw.AppendLine("P2 diff 数:  $($script:P2)  (P2率=" + $(if ($total) { [Math]::Round(100.0 * $script:P2 / $total, 2) } else { 0 }) + "%)")
  [void]$sw.AppendLine("P3 diff 数:  $($script:P3)")
  [void]$sw.AppendLine("Go 5xx/0:    $($script:GoErr)")
  [void]$sw.AppendLine("Rust 5xx/0:  $($script:RustErr)  (Rust错误率=$rustErrRate%)")
  [void]$sw.AppendLine("")
  [void]$sw.AppendLine("== 响应时间(ms) ==")
  [void]$sw.AppendLine("Go   平均: $goAvg   P99: $goP99")
  [void]$sw.AppendLine("Rust 平均: $rustAvg   P99: $rustP99")
  $text = $sw.ToString()
  $text | Out-File -FilePath $SummaryPath -Encoding UTF8
  Write-Host ""
  Write-Host $text
  Write-Host ("==> 日志: " + $LogPath)
  Write-Host ("==> 汇总: " + $SummaryPath)
}

# ===========================================================================
# 主流程
# ===========================================================================
Write-Host "==> shadow-observe: Go:$GoPort  Rust:$RustPort  请求集:$RequestSet  时长:${DurationMinutes}min  间隔:${IntervalSeconds}s"

# 1. 双服务检查/启动
Write-Host "==> 检查端口..."
$goRunning   = Test-HttpUp $GoBase
$rustRunning = Test-HttpUp $RustBase
if (-not $goRunning) {
  Write-Host "==> 启动 Go 版（内存模式）: $GoDir"
  $script:StartedGo = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile", "-Command", "`$env:MEMORY_STORE_ENABLED='true'; `$env:SERVER_PORT='$GoPort'; `$env:DEFAULT_ADMIN_PASSWORD='$AdminPass'; `$env:RATE_LIMIT_ENABLED='false'; `$env:ALLOW_PUBLIC_REGISTRATION='true'; `$env:SERVER_ENV='development'; Set-Location '$GoDir'; go run . *> '$GoOutLog'" `
    -WorkingDirectory $GoDir -WindowStyle Hidden -PassThru
  Write-Host ("    Go PID = " + $script:StartedGo.Id)
} else { Write-Host "==> Go 版已在 :$GoPort 运行，复用" }
if (-not $rustRunning) {
  Write-Host "==> 启动 Rust 版: $RustDir"
  $script:StartedRust = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile", "-Command", "`$env:SERVER_PORT='$RustPort'; `$env:MEMORY_STORE_ENABLED='true'; `$env:RATE_LIMIT_ENABLED='false'; Set-Location '$RustDir'; cargo run *> '$RustOutLog'" `
    -WorkingDirectory $RustDir -WindowStyle Hidden -PassThru
  Write-Host ("    Rust PID = " + $script:StartedRust.Id)
} else { Write-Host "==> Rust 版已在 :$RustPort 运行，复用" }

Write-Host "==> 等待服务就绪（最多 90s）..."
$goReady   = Wait-Ready $GoBase 90
$rustReady = Wait-Ready $RustBase 90
Write-Host ("    Go ready=$goReady  Rust ready=$rustReady")
if (-not $goReady -or -not $rustReady) {
  Write-Host "!! 服务未就绪" -ForegroundColor Red
  if (Test-Path $GoOutLog)   { Get-Content $GoOutLog -Tail 15 -ErrorAction SilentlyContinue }
  if (Test-Path $RustOutLog) { Get-Content $RustOutLog -Tail 15 -ErrorAction SilentlyContinue }
  exit 1
}

# 2. 登录
Write-Host "==> 登录 admin 双版..."
if (-not (Ensure-Login)) { Write-Host "!! 登录失败" -ForegroundColor Red; exit 2 }
Write-Host ("    Go/Rust admin token: ok  user token: " + $(if ($script:GoUser -and $script:RustUser) { "ok" } else { "(fallback)" }))

# 3. 构建请求集
$ReqSet = Build-RequestSet $RequestSet
Write-Host ("==> 请求集: " + $ReqSet.Count + " 条 (mode=$RequestSet)")

# 写日志头
"# shadow-observe  " + (Get-Date -Format "yyyy-MM-dd HH:mm:ss") + "  Go=$GoBase Rust=$RustBase set=$RequestSet" | Out-File -FilePath $LogPath -Encoding UTF8

$script:StartTime = Get-Date
$deadline = (Get-Date).AddMinutes($DurationMinutes)
Write-Host ("==> 采样开始，截止 " + $deadline.ToString("HH:mm:ss") + " (Ctrl-C 可优雅停止，默认不停止双服务)")

# 4. 主采样循环（try/finally 保证 Ctrl-C 也落汇总）
try {
  while ((Get-Date) -lt $deadline) {
    $script:Round++
    # 长跑 token 刷新
    if (-not (Ensure-Login)) {
      Write-Host ("[" + (Get-Date -Format "HH:mm:ss") + "] token 刷新失败，重试...") -ForegroundColor Yellow
      Start-Sleep -Seconds 5
      continue
    }
    foreach ($req in $ReqSet) {
      if ((Get-Date) -ge $deadline) { break }
      switch ($req.Role) {
        "admin" { $gTok = $script:GoAdmin; $rTok = $script:RustAdmin }
        "none"  { $gTok = "";              $rTok = "" }
        "user"  { $gTok = $script:GoUser;  $rTok = $script:RustUser }
      }
      $g = Invoke-Api $GoBase   $req.M $req.P $gTok $req.B
      $r = Invoke-Api $RustBase $req.M $req.P $rTok $req.B

      # 遇到 401（token 过期）→ 立即重登并跳过本轮该请求
      if (($g.Code -eq 401 -or $r.Code -eq 401) -and $req.Role -ne "none") {
        $script:TokenTime = [DateTime]::MinValue
        continue
      }

      $level = Classify-Diff $req.M $req.P $g $r
      $script:Total++
      switch ($level) {
        "P0"              { $script:P0++ }
        "P1"              { $script:P1++ }
        "P1-KNOWN-EMPTY"  { $script:P1KnownEmpty++ }
        "P2"              { $script:P2++ }
        default           { $script:P3++ }
      }
      if ($g.Code -ge 500 -or $g.Code -eq 0) { $script:GoErr++ }
      if ($r.Code -ge 500 -or $r.Code -eq 0) { $script:RustErr++ }
      $script:GoMsList.Add([int]$g.Ms)
      $script:RustMsList.Add([int]$r.Ms)

      $detail = "go_sha=" + $g.Sha + " rust_sha=" + $r.Sha + " go_keys=" + (Get-EnvKeys $g.Body) + " rust_keys=" + (Get-EnvKeys $r.Body)
      $line = "[{0}] [{1}] {2} {3} go_status={4} rust_status={5} go_ms={6} rust_ms={7} {8}" -f `
        (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $level, $req.M, $req.P, $g.Code, $r.Code, $g.Ms, $r.Ms, $detail
      Add-LogLine $line
    }

    # 每轮控制台实时统计
    $errRate = if ($script:Total -gt 0) { [Math]::Round(100.0 * $script:RustErr / $script:Total, 1) } else { 0 }
    Write-Host ("[{0}] round={1} total={2} p0={3} p1={4} p2={5} rust_err={6}%" -f `
      (Get-Date -Format "HH:mm:ss"), $script:Round, $script:Total, $script:P0, $script:P1, $script:P2, $errRate)

    # 等待下一轮（可被 Ctrl-C 中断）
    $remain = $IntervalSeconds
    while ($remain -gt 0 -and (Get-Date) -lt $deadline) {
      $sleepStep = [Math]::Min(1, $remain)
      Start-Sleep -Seconds $sleepStep
      $remain -= $sleepStep
    }
  }
}
finally {
  Write-Host "==> 采样结束（正常到时或被中断），输出最终统计..."
  Write-FinalSummary
  if ($StopServicesOnExit) {
    if ($script:StartedGo)   { Write-Host "==> 停止脚本启动的 Go 进程 PID=" + $script:StartedGo.Id;   Stop-TrackedProcess $script:StartedGo }
    if ($script:StartedRust) { Write-Host "==> 停止脚本启动的 Rust 进程 PID=" + $script:StartedRust.Id; Stop-TrackedProcess $script:StartedRust }
  } else {
    Write-Host "==> 保留双服务继续运行（如需退出时停止，加 -StopServicesOnExit）"
  }
}
