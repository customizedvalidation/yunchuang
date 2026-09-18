#Requires -Version 5.1
<#
.SYNOPSIS
  P4-02 影子流量双轨对比脚本（Go :8000 vs Rust :8001）。
.DESCRIPTION
  启动（如未运行）Go 内存版与 Rust SQLite 版，双发同一混合请求集，逐请求对比：
    L0 状态码 / L1 信封键集 / L2 数据形状 / 耗时 / 体哈希
  diff 分级 P0(状态码不一致) / P1(信封或形状不一致) / P2(耗时差>200%) / P3(可接受)
  循环回放直到 -DurationMinutes 耗尽，输出 CSV 与汇总报告。
.PARAMETER DurationMinutes
  采样总时长（分钟），默认 5。
.PARAMETER RequestIntervalMs
  每个请求之间的间隔毫秒，默认 200。
.PARAMETER GoPort
  Go 版端口，默认 8000。
.PARAMETER RustPort
  Rust 版端口，默认 8001。
.PARAMETER KeepServicesRunning
  采样结束后保留双服务进程（默认仅停止本脚本启动的进程）。
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/shadow-compare.ps1 -DurationMinutes 5
#>
param(
  [int]$DurationMinutes = 5,
  [int]$RequestIntervalMs = 200,
  [int]$GoPort = 8000,
  [int]$RustPort = 8001,
  [string]$GoDir = "D:\YCYD\metaclouds-backend",
  [string]$RustDir = "D:\YCYD\metaclouds-backend-rust",
  [string]$WorkDir = "D:\YCYD\metaclouds-backend-rust\scripts\p4work",
  [switch]$KeepServicesRunning
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"

$GoBase   = "http://127.0.0.1:$GoPort"
$RustBase = "http://127.0.0.1:$RustPort"
$AdminUser = "admin"
$AdminPass = "Admin@123456"
$TestUser  = "shadowuser"
$TestPass  = "User@123456"

if (-not (Test-Path $WorkDir)) { New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null }
$CsvPath    = Join-Path $WorkDir "shadow-results.csv"
$SummaryPath= Join-Path $WorkDir "shadow-summary.txt"
$GoOutLog   = Join-Path $WorkDir "shadow-go-out.log"
$GoErrLog   = Join-Path $WorkDir "shadow-go-err.log"
$RustOutLog = Join-Path $WorkDir "shadow-rust-out.log"
$RustErrLog = Join-Path $WorkDir "shadow-rust-err.log"

$script:StartedGo = $null
$script:StartedRust = $null

# ---------------------------------------------------------------------------
# 工具函数
# ---------------------------------------------------------------------------
function Test-HttpUp([string]$base) {
  try {
    $r = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
    return $true
  } catch {
    if ($_.Exception.Response) { return $true }  # 任意 HTTP 响应即视为已起
    return $false
  }
}

function Wait-Ready([string]$base,[int]$port,[int]$maxSeconds = 90) {
  $deadline = (Get-Date).AddSeconds($maxSeconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-HttpUp $base) {
      # 再探一次 HTTP，确保进程已就绪接受请求
      try {
        $r = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 3 -UseBasicParsing -ErrorAction Stop
        if ($r.StatusCode -eq 200) { return $true }
      } catch {
        # health 不存在时退一步：只要端口监听即认为就绪
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
    # 杀掉进程树（go run / cargo run 会 fork 子进程）
    $cmd = "taskkill /PID $($proc.Id) /T /F 2>`$null"
    Invoke-Expression $cmd | Out-Null
  } catch { }
}

function Stop-PortOwner([int]$port) {
  try {
    $conns = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue
    foreach ($c in $conns) {
      try { taskkill /PID $c.OwningProcess /T /F 2>$null | Out-Null } catch { }
    }
  } catch { }
}

# ---------------------------------------------------------------------------
# 服务启动
# ---------------------------------------------------------------------------
Write-Host "==> 检查端口 Go:$GoPort / Rust:$RustPort"
$goRunning   = Test-HttpUp $GoBase
$rustRunning = Test-HttpUp $RustBase

if (-not $goRunning) {
  Write-Host "==> 启动 Go 版（内存模式）: $GoDir"
  # 注意：不在脚本进程里设 $env:，避免泄漏给 Rust 子进程（dotenvy 不覆盖已存在 env）。
  $script:StartedGo = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile","-Command","`$env:MEMORY_STORE_ENABLED='true'; `$env:SERVER_PORT='$GoPort'; `$env:DEFAULT_ADMIN_PASSWORD='$AdminPass'; `$env:RATE_LIMIT_ENABLED='false'; `$env:ALLOW_PUBLIC_REGISTRATION='true'; `$env:SERVER_ENV='development'; Set-Location '$GoDir'; go run . *> '$GoOutLog'" `
    -WorkingDirectory $GoDir -WindowStyle Hidden -PassThru
  Write-Host ("    Go PID = " + $script:StartedGo.Id + " (日志: $GoOutLog)")
} else {
  Write-Host "==> Go 版已在 :$GoPort 运行，复用"
}

if (-not $rustRunning) {
  Write-Host "==> 启动 Rust 版: $RustDir"
  $script:StartedRust = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile","-Command","`$env:SERVER_PORT='$RustPort'; `$env:MEMORY_STORE_ENABLED='true'; `$env:RATE_LIMIT_ENABLED='false'; Set-Location '$RustDir'; cargo run *> '$RustOutLog'" `
    -WorkingDirectory $RustDir -WindowStyle Hidden -PassThru
  Write-Host ("    Rust PID = " + $script:StartedRust.Id + " (日志: $RustOutLog)")
} else {
  Write-Host "==> Rust 版已在 :$RustPort 运行，复用"
}

Write-Host "==> 等待服务就绪（最多 60s）..."
$goReady   = Wait-Ready $GoBase $GoPort
$rustReady = Wait-Ready $RustBase $RustPort
Write-Host ("    Go ready=$goReady  Rust ready=$rustReady")
if (-not $goReady -or -not $rustReady) {
  Write-Host "!! 服务未就绪，GoOut 末尾：" -ForegroundColor Red
  if (Test-Path $GoOutLog)   { Get-Content $GoOutLog -Tail 20 -ErrorAction SilentlyContinue }
  if (Test-Path $RustOutLog) { Get-Content $RustOutLog -Tail 20 -ErrorAction SilentlyContinue }
  # 不自动清理，交由人工判断
  exit 1
}

# ---------------------------------------------------------------------------
# 登录
# ---------------------------------------------------------------------------
function Invoke-Api([string]$base,[string]$method,[string]$path,[string]$token,[string]$body) {
  $uri = $base + $path
  $headers = @{ "Accept" = "application/json" }
  if ($token) { $headers["Authorization"] = "Bearer $token" }
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $result = [ordered]@{
    Code = 0; Ms = 0; Body = ""; Sha = ""; CType = ""; Trace = ""; HasSetCookie = $false; Err = ""
  }
  try {
    $p = @{ Uri = $uri; Method = $method; Headers = $headers; TimeoutSec = 15; UseBasicParsing = $true; ErrorAction = "Stop" }
    if ($body -and $method -ne "GET") { $p["Body"] = $body; $p["ContentType"] = "application/json" }
    $r = Invoke-WebRequest @p
    $sw.Stop()
    $result.Code = [int]$r.StatusCode
    $result.Ms = [int]$sw.ElapsedMilliseconds
    $result.Body = [string]$r.Content
    if ($r.Headers["Content-Type"]) { $result.CType = [string]$r.Headers["Content-Type"] }
    if ($r.Headers["X-Trace-Id"])   { $result.Trace = [string]$r.Headers["X-Trace-Id"] }
    elseif ($r.Headers["X-Request-ID"]) { $result.Trace = [string]$r.Headers["X-Request-ID"] }
    if ($r.Headers["Set-Cookie"])   { $result.HasSetCookie = $true }
  } catch {
    $sw.Stop()
    $result.Ms = [int]$sw.ElapsedMilliseconds
    $result.Err = $_.Exception.Message
    if ($_.Exception.Response) {
      try {
        $result.Code = [int]$_.Exception.Response.StatusCode
        $sr = New-Object IO.StreamReader($_.Exception.Response.GetResponseStream())
        $result.Body = $sr.ReadToEnd()
        if ($_.Exception.Response.Headers["Content-Type"]) { $result.CType = [string]$_.Exception.Response.Headers["Content-Type"] }
      } catch { }
    }
  }
  if ($result.Body) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($result.Body)
    $hash = $sha.ComputeHash($bytes)
    $result.Sha = ([System.BitConverter]::ToString($hash) -replace "-","").Substring(0,16).ToLower()
  }
  return [pscustomobject]$result
}

function Login([string]$base) {
  $body = @{ username = $AdminUser; password = $AdminPass } | ConvertTo-Json -Compress
  $r = Invoke-Api $base "POST" "/api/v1/auth/login" "" $body
  if ($r.Code -ne 200 -and $r.Code -ne 201) { return $null }
  try {
    $j = $r.Body | ConvertFrom-Json
    return [string]$j.data.token
  } catch { return $null }
}

Write-Host "==> 登录 admin 双版..."
$gAdmin = Login $GoBase
$rAdmin = Login $RustBase
Write-Host ("    Go admin token:   " + $(if($gAdmin){"ok"}else{"FAIL"}))
Write-Host ("    Rust admin token: " + $(if($rAdmin){"ok"}else{"FAIL"}))
if (-not $gAdmin -or -not $rAdmin) { Write-Host "!! 登录失败" -ForegroundColor Red; exit 2 }

# 获取普通用户 token（403 场景用）。Go 走自助注册，Rust 走 admin 创建。
function Get-UserToken([string]$base,[string]$adminToken,[string]$which) {
  if ($which -eq "go") {
    $reg = @{ username = $TestUser; password = $TestPass; email = "$TestUser@example.com" } | ConvertTo-Json -Compress
    $null = Invoke-Api $base "POST" "/api/v1/auth/register" "" $reg
  } else {
    $u = @{ username = $TestUser; password = $TestPass; email = "$TestUser@example.com"; role = "user" } | ConvertTo-Json -Compress
    $null = Invoke-Api $base "POST" "/api/v1/users" $adminToken $u
  }
  $login = @{ username = $TestUser; password = $TestPass } | ConvertTo-Json -Compress
  $r = Invoke-Api $base "POST" "/api/v1/auth/login" "" $login
  try { $j = $r.Body | ConvertFrom-Json; return [string]$j.data.token } catch { return "" }
}
$gUser = Get-UserToken $GoBase $gAdmin "go"
$rUser = Get-UserToken $RustBase $rAdmin "rust"
Write-Host ("    Go user token:    " + $(if($gUser){"ok"}else{"(fallback)"}))
Write-Host ("    Rust user token:  " + $(if($rUser){"ok"}else{"(fallback)"}))

# ---------------------------------------------------------------------------
# 混合请求集（>=30，覆盖各域 CRUD/列表/分页/搜索/401/403）
# 每条: @{M=方法; P=路径(含query); B=请求体; Role=admin|none|user}
# ---------------------------------------------------------------------------
$EmptyBody = "{}"
$ReqSet = @(
  @{M="GET";  P="/health";                          B="";         Role="none"}
  @{M="GET";  P="/api/v1/auth/profile";             B="";         Role="admin"}
  @{M="GET";  P="/api/v1/auth/csrf";               B="";         Role="admin"}
  @{M="GET";  P="/api/v1/clusters";                B="";         Role="admin"}
  @{M="GET";  P="/api/v1/clusters?page=1&page_size=5"; B="";      Role="admin"}
  @{M="GET";  P="/api/v1/resources";               B="";         Role="admin"}
  @{M="GET";  P="/api/v1/tenants";                  B="";         Role="admin"}
  @{M="GET";  P="/api/v1/jobs";                    B="";         Role="admin"}
  @{M="GET";  P="/api/v1/jobs?status=running";     B="";         Role="admin"}
  @{M="GET";  P="/api/v1/gpus";                    B="";         Role="admin"}
  @{M="GET";  P="/api/v1/gpus/allocations";         B="";         Role="admin"}
  @{M="GET";  P="/api/v1/gpus/utilization";        B="";         Role="admin"}
  @{M="GET";  P="/api/v1/partitions";               B="";         Role="admin"}
  @{M="GET";  P="/api/v1/quotas";                   B="";         Role="admin"}
  @{M="GET";  P="/api/v1/schedulers";               B="";         Role="admin"}
  @{M="GET";  P="/api/v1/datasets";                B="";         Role="admin"}
  @{M="GET";  P="/api/v1/checkpoints";             B="";         Role="admin"}
  @{M="GET";  P="/api/v1/acceleration";             B="";         Role="admin"}
  @{M="GET";  P="/api/v1/security/policies";        B="";         Role="admin"}
  @{M="GET";  P="/api/v1/monitoring/metrics";      B="";         Role="admin"}
  @{M="GET";  P="/api/v1/topology";                 B="";         Role="admin"}
  # 写（空体，预期 Go 宽松 201 / Rust 严格 422 → P3 白名单）
  @{M="POST"; P="/api/v1/clusters";                B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/tenants";                 B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/jobs";                    B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/gpus";                    B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/partitions";              B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/quotas";                  B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/schedulers";              B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/datasets";                B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/checkpoints";            B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/acceleration";            B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/security/policies";       B=$EmptyBody; Role="admin"}
  @{M="POST"; P="/api/v1/quotas/check";           B=$EmptyBody; Role="admin"}
  # 详情路由（数据独立，404/200 差异记 P3）
  @{M="GET";  P="/api/v1/clusters/1";              B="";         Role="admin"}
  @{M="GET";  P="/api/v1/jobs/1";                  B="";         Role="admin"}
  @{M="GET";  P="/api/v1/gpus/1";                  B="";         Role="admin"}
  # 401：无 token 访问受保护端点
  @{M="GET";  P="/api/v1/clusters";                B="";         Role="none"}
  @{M="GET";  P="/api/v1/auth/profile";            B="";         Role="none"}
  # 403：普通用户 token 写操作
  @{M="POST"; P="/api/v1/clusters";                B=$EmptyBody; Role="user"}
  @{M="POST"; P="/api/v1/tenants";                B=$EmptyBody; Role="user"}
)
Write-Host ("==> 混合请求集: " + $ReqSet.Count + " 条")

# ---------------------------------------------------------------------------
# 信封/形状提取
# ---------------------------------------------------------------------------
function Get-EnvKeys($body) {
  try {
    $j = $body | ConvertFrom-Json
    return ($j.PSObject.Properties.Name | Sort-Object) -join ","
  } catch { return "non-json" }
}
function Get-Shape($node,$depth=0) {
  if ($depth -gt 4) { return "..." }
  if ($null -eq $node) { return "null" }
  if ($node -is [array]) {
    if ($node.Count -eq 0) { return "[]" } else { return "[ " + (Get-Shape $node[0] ($depth+1)) + " ]" }
  }
  if ($node -is [pscustomobject]) { return "{" + (($node.PSObject.Properties.Name | Sort-Object) -join ",") + "}" }
  if ($node -is [bool]) { return "bool" }
  if (($node -is [int]) -or ($node -is [long]) -or ($node -is [double]) -or ($node -is [decimal])) { return "num" }
  if ($node -is [string]) { return "str" }
  return $node.GetType().Name
}
function Get-DataShape($body) {
  try {
    $j = $body | ConvertFrom-Json
    if ($null -eq $j) { return "null" }
    return Get-Shape $j.data 0
  } catch { return "non-json" }
}
function Get-SuccessBool($body) {
  try { $j = $body | ConvertFrom-Json; if ($null -ne $j.success) { return [bool]$j.success } } catch { }
  return $null
}

# P3 白名单：空体写 POST（Go 201 / Rust 422）；详情路由 /{域}/1（数据独立）
function Is-KnownP3([string]$method,[string]$path,[int]$gCode,[int]$rCode) {
  # /health：Rust 未实现该路由（单侧已知遗留，非业务契约）
  if ($path -eq "/health" -and $gCode -eq 200 -and $rCode -eq 404) { return $true }
  # 空体 POST：Go 宽松(201/400/409)、Rust 严格(422) —— P4-01 §6.2 已裁决校验严格度可接受
  if ($method -eq "POST" -and $gCode -ne $rCode -and $rCode -eq 422) {
    $gOk = (($gCode -ge 200 -and $gCode -lt 300) -or ($gCode -ge 400 -and $gCode -lt 500))
    if ($gOk) { return $true }
  }
  # /gpus/utilization：Go 返回 Prometheus/text 非 JSON，Rust 返回 JSON 信封 —— 监控输出口径差异（同 /monitoring/metrics）
  if ($path -eq "/api/v1/gpus/utilization" -and $gCode -eq 200 -and $rCode -eq 200) { return $true }
  # 详情路由数据独立（Go 有种子 -> 200，Rust 空库 -> 404）
  if ($method -eq "GET" -and $path -match "^/api/v1/(clusters|jobs|gpus)/\d+$") {
    if (($gCode -eq 200 -and $rCode -eq 404) -or ($gCode -eq 404 -and $rCode -eq 200)) { return $true }
  }
  return $false
}

# 形状归一化：空数组 [] 与非空数组 [ {obj} ] 同视为 "array"（数据填充独立，非契约差异）
function ShapeNorm($s) {
  if ($s -eq "null") { return "null" }
  if ($s.StartsWith("[")) { return "array" }
  return $s
}

function Classify-Diff([string]$method,[string]$path,[pscustomobject]$g,[pscustomobject]$r) {
  # P0: 状态码不一致（白名单外）
  if ($g.Code -ne $r.Code) {
    if (Is-KnownP3 $method $path $g.Code $r.Code) { return "P3" }
    return "P0"
  }
  # 信封 success 翻转 → P1
  $gS = Get-SuccessBool $g.Body
  $rS = Get-SuccessBool $r.Body
  if ($null -ne $gS -and $null -ne $rS -and $gS -ne $rS) { return "P1" }
  # 信封键集不一致 → P1（时间戳/具体值不算，键集算）
  $gKeys = Get-EnvKeys $g.Body
  $rKeys = Get-EnvKeys $r.Body
  if ($gKeys -ne $rKeys) {
    # logout 字段名差异是 P3 白名单（仅 message vs data 差一关键字段）
    if ($path -eq "/api/v1/auth/logout") { return "P3" }
    return "P1"
  }
  # data 形状不一致 → P1
  if ($gS -eq $true -and $rS -eq $true) {
    $gShape = Get-DataShape $g.Body
    $rShape = Get-DataShape $r.Body
    if ((ShapeNorm $gShape) -ne (ShapeNorm $rShape)) {
      # monitoring/metrics 口径差异 → P3
      if ($path -eq "/api/v1/monitoring/metrics" -or $path -eq "/api/v1/gpus/utilization") { return "P3" }
      # Go 空序列序列化为 null、Rust 空序列序列化为 []：数据相关已知遗留（仅空集合出现）
      if ((ShapeNorm $gShape) -eq "null" -and (ShapeNorm $rShape) -eq "array") { return "P1-KNOWN-EMPTY" }
      return "P1"
    }
  }
  # P2: 延迟差 >200%（基线 <5ms 忽略）
  if ($g.Ms -ge 5 -and $r.Ms -ge 5) {
    $max = [Math]::Max($g.Ms,$r.Ms)
    $min = [Math]::Min($g.Ms,$r.Ms)
    if ($min -gt 0 -and ($max / $min) -gt 2.0) { return "P2" }
  }
  return "P3"
}

# ---------------------------------------------------------------------------
# 主循环
# ---------------------------------------------------------------------------
$rows = New-Object System.Collections.Generic.List[object]
$deadline = (Get-Date).AddMinutes($DurationMinutes)
$round = 0
Write-Host ("==> 采样开始，时长 " + $DurationMinutes + " 分钟，间隔 " + $RequestIntervalMs + "ms")
$startTime = Get-Date

while ((Get-Date) -lt $deadline) {
  $round++
  $idx = 0
  foreach ($req in $ReqSet) {
    if ((Get-Date) -ge $deadline) { break }
    $idx++
    switch ($req.Role) {
      "admin" { $gTok = $gAdmin; $rTok = $rAdmin }
      "none"  { $gTok = "";      $rTok = "" }
      "user"  { $gTok = $gUser; $rTok = $rUser }
    }
    $g = Invoke-Api $GoBase   $req.M $req.P $gTok $req.B
    $r = Invoke-Api $RustBase $req.M $req.P $rTok $req.B
    $level = Classify-Diff $req.M $req.P $g $r
    $rows.Add([pscustomobject]@{
      Round   = $round
      Seq     = $idx
      Method  = $req.M
      Path    = $req.P
      Role    = $req.Role
      GoCode  = $g.Code; GoMs = $g.Ms; GoSha = $g.Sha
      RustCode= $r.Code; RustMs= $r.Ms; RustSha= $r.Sha
      Diff    = $level
      GoEnvKeys  = (Get-EnvKeys $g.Body)
      RustEnvKeys= (Get-EnvKeys $r.Body)
      GoShape    = (Get-DataShape $g.Body)
      RustShape  = (Get-DataShape $r.Body)
    })
    Start-Sleep -Milliseconds $RequestIntervalMs
  }
  if ($round % 5 -eq 0) {
    Write-Host ("    ... round " + $round + " rows=" + $rows.Count + " elapsed=" + [int]((Get-Date)-$startTime).TotalSeconds + "s")
  }
}

Write-Host ("==> 采样结束，共 " + $rows.Count + " 条记录，" + $round + " 轮")

# ---------------------------------------------------------------------------
# 统计
# ---------------------------------------------------------------------------
$total = $rows.Count
$p0 = @($rows | Where-Object { $_.Diff -eq "P0" })
$p1 = @($rows | Where-Object { $_.Diff -eq "P1" })
$p1known = @($rows | Where-Object { $_.Diff -eq "P1-KNOWN-EMPTY" })
$p2 = @($rows | Where-Object { $_.Diff -eq "P2" })
$p3 = @($rows | Where-Object { $_.Diff -eq "P3" })

$goMsAll   = @($rows | ForEach-Object { $_.GoMs })
$rustMsAll = @($rows | ForEach-Object { $_.RustMs })
$goAvg   = if ($goMsAll.Count -gt 0)   { [Math]::Round(($goMsAll   | Measure-Object -Average).Average,1) }   else { 0 }
$rustAvg = if ($rustMsAll.Count -gt 0) { [Math]::Round(($rustMsAll | Measure-Object -Average).Average,1) } else { 0 }
function Pctile($arr,$p) {
  if (-not $arr -or $arr.Count -eq 0) { return 0 }
  $sorted = $arr | Sort-Object
  $idx = [Math]::Max(0, [Math]::Min($sorted.Count-1, [int][Math]::Ceiling($p/100.0*$sorted.Count)-1))
  return [int]$sorted[$idx]
}
$goP99   = Pctile $goMsAll   99
$rustP99 = Pctile $rustMsAll 99

# Rust 错误率（5xx 或 0=未捕获）
$rustErr = @($rows | Where-Object { $_.RustCode -ge 500 -or $_.RustCode -eq 0 }).Count
$goErr   = @($rows | Where-Object { $_.GoCode   -ge 500 -or $_.GoCode   -eq 0 }).Count
$rustErrRate = if ($total -gt 0) { [Math]::Round(100.0*$rustErr/$total,3) } else { 0 }

# 各域 diff 分布（按路径首段聚合）
$domainDist = $rows | Group-Object { ($_.Path -split '/')[2] } | ForEach-Object {
  [pscustomobject]@{
    Domain = $_.Name; Count = $_.Count
    P0 = @($_.Group | Where-Object { $_.Diff -eq "P0" }).Count
    P1 = @($_.Group | Where-Object { $_.Diff -eq "P1" }).Count
    P2 = @($_.Group | Where-Object { $_.Diff -eq "P2" }).Count
  }
} | Sort-Object Domain

# 写 CSV
$rows | Export-Csv -Path $CsvPath -NoTypeInformation -Encoding UTF8

# 写汇总
$sw = New-Object System.Text.StringBuilder
[void]$sw.AppendLine("P4-02 影子双轨本机短采样汇总")
[void]$sw.AppendLine("生成时间: " + (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz"))
[void]$sw.AppendLine("采样时长(分): " + $DurationMinutes + "  间隔(ms): " + $RequestIntervalMs + "  轮次: " + $round)
[void]$sw.AppendLine("Go: $GoBase  Rust: $RustBase")
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 总览 ==")
[void]$sw.AppendLine("总请求数:       $total")
[void]$sw.AppendLine("P0 diff 数:     " + $p0.Count)
[void]$sw.AppendLine("P1 diff 数:     " + $p1.Count + "  (已知遗留 P1-KNOWN-EMPTY 空集合序列化: " + $p1known.Count + ")")
[void]$sw.AppendLine("P2 diff 数:     " + $p2.Count + "  (P2率=" + $(if($total){[Math]::Round(100.0*$p2.Count/$total,2)}else{0}) + "%)")
[void]$sw.AppendLine("P3 diff 数:     " + $p3.Count)
[void]$sw.AppendLine("Go 5xx/0 数:    $goErr")
[void]$sw.AppendLine("Rust 5xx/0 数:  $rustErr  (Rust错误率=$rustErrRate%)")
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 响应时间(ms) ==")
[void]$sw.AppendLine("Go   平均: $goAvg   P99: $goP99")
[void]$sw.AppendLine("Rust 平均: $rustAvg   P99: $rustP99")
if ($goP99 -gt 0) { [void]$sw.AppendLine("Rust/Go P99 比: " + [Math]::Round(100.0*$rustP99/$goP99,1) + "%") }
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== 各域 diff 分布 ==")
foreach ($d in $domainDist) {
  [void]$sw.AppendLine(("{0,-18} n={1,-5} P0={2,-4} P1={3,-4} P2={4}" -f $d.Domain,$d.Count,$d.P0,$d.P1,$d.P2))
}
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== P0 明细（应为空）==")
if ($p0.Count -eq 0) { [void]$sw.AppendLine("(无)") }
foreach ($x in $p0) { [void]$sw.AppendLine(("  {0} {1}  Go={2} Rust={3}  GoShape={4} RustShape={5}" -f $x.Method,$x.Path,$x.GoCode,$x.RustCode,$x.GoShape,$x.RustShape)) }
[void]$sw.AppendLine("")
[void]$sw.AppendLine("== P1 明细 ==")
if ($p1.Count -eq 0) { [void]$sw.AppendLine("(无)") }
foreach ($x in ($p1 | Select-Object -First 20)) { [void]$sw.AppendLine(("  {0} {1}  Go={2}({3}) Rust={4}({5})" -f $x.Method,$x.Path,$x.GoCode,$x.GoShape,$x.RustCode,$x.RustShape)) }
if ($p1.Count -gt 20) { [void]$sw.AppendLine("  ... 其余 " + ($p1.Count-20) + " 条见 CSV") }

$summaryText = $sw.ToString()
$summaryText | Out-File -FilePath $SummaryPath -Encoding UTF8
Write-Host ""
Write-Host $summaryText
Write-Host ("==> CSV: " + $CsvPath)
Write-Host ("==> 汇总: " + $SummaryPath)

# ---------------------------------------------------------------------------
# 清理
# ---------------------------------------------------------------------------
if (-not $KeepServicesRunning) {
  if ($script:StartedGo)   { Write-Host "==> 停止脚本启动的 Go 进程 PID=" + $script:StartedGo.Id;   Stop-TrackedProcess $script:StartedGo }
  if ($script:StartedRust) { Write-Host "==> 停止脚本启动的 Rust 进程 PID=" + $script:StartedRust.Id; Stop-TrackedProcess $script:StartedRust }
} else {
  Write-Host "==> -KeepServicesRunning 已指定，保留双服务"
}
