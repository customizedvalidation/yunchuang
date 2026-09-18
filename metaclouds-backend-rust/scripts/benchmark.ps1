#Requires -Version 5.1
<#
.SYNOPSIS
  P4-03 Performance benchmark: Go :8000 vs Rust :8001.
.DESCRIPTION
  Runs concurrent load tests against both Go (in-memory) and Rust (SQLite) backends,
  comparing throughput, latency percentiles, and memory usage.
  Order: Go first all endpoints, then Rust all endpoints (avoid resource contention).
  Cooldown >=10s between rounds. Each round <=30s.
.PARAMETER Concurrency
  Concurrency levels, default @(10, 50).
.PARAMETER DurationSeconds
  Duration per round in seconds, default 20 (max 30).
.PARAMETER GoPort
  Go backend port, default 8000.
.PARAMETER RustPort
  Rust backend port, default 8001.
.EXAMPLE
  powershell -ExecutionPolicy Bypass -File scripts/benchmark.ps1
#>
param(
  [int[]]$Concurrency = @(10, 50),
  [int]$DurationSeconds = 20,
  [int]$GoPort = 8000,
  [int]$RustPort = 8001,
  [int]$Rounds = 1,
  [string]$GoDir = "D:\YCYD\metaclouds-backend",
  [string]$RustDir = "D:\YCYD\metaclouds-backend-rust",
  [string]$WorkDir = "D:\YCYD\metaclouds-backend-rust\scripts\p4work",
  [switch]$KeepServicesRunning
)
$ErrorActionPreference = "Continue"
$ProgressPreference = "SilentlyContinue"

if ($DurationSeconds -gt 30) { $DurationSeconds = 30; Write-Host "[WARN] Duration capped at 30s per resource policy" }

$GoBase   = "http://127.0.0.1:$GoPort"
$RustBase = "http://127.0.0.1:$RustPort"
$AdminUser = "admin"
$AdminPass = "Admin@123456"
$BenchExe = Join-Path $PSScriptRoot "bench\bench.exe"

if (-not (Test-Path $WorkDir)) { New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null }
$CsvPath      = Join-Path $WorkDir "benchmark-results.csv"
$MemCsvPath   = Join-Path $WorkDir "benchmark-memory.csv"
$SummaryPath  = Join-Path $WorkDir "benchmark-summary.txt"
$GoOutLog     = Join-Path $WorkDir "bench-go-out.log"
$RustOutLog   = Join-Path $WorkDir "bench-rust-out.log"

$script:StartedGo = $null
$script:StartedRust = $null
$script:GoProcessId = $null
$script:RustProcessId = $null

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
function Test-HttpUp([string]$base) {
  try {
    $r = Invoke-WebRequest -Uri "$base/health" -TimeoutSec 2 -UseBasicParsing -ErrorAction Stop
    return ($r.StatusCode -eq 200)
  } catch {
    if ($_.Exception.Response) { return $true }
    return $false
  }
}

function Wait-Ready([string]$base, [int]$maxSeconds = 120) {
  $deadline = (Get-Date).AddSeconds($maxSeconds)
  while ((Get-Date) -lt $deadline) {
    if (Test-HttpUp $base) { return $true }
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

function Get-PortProcessId([int]$port) {
  $conn = Get-NetTCPConnection -LocalPort $port -State Listen -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($conn) { return $conn.OwningProcess }
  return $null
}

# ---------------------------------------------------------------------------
# Start services
# ---------------------------------------------------------------------------
Write-Host "==> Checking ports Go:$GoPort / Rust:$RustPort"
$goRunning   = Test-HttpUp $GoBase
$rustRunning = Test-HttpUp $RustBase

if (-not $goRunning) {
  Write-Host "==> Starting Go backend (in-memory): $GoDir"
  $script:StartedGo = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile","-Command","`$env:MEMORY_STORE_ENABLED='true'; `$env:SERVER_PORT='$GoPort'; `$env:DEFAULT_ADMIN_PASSWORD='$AdminPass'; `$env:RATE_LIMIT_ENABLED='false'; `$env:ALLOW_PUBLIC_REGISTRATION='true'; `$env:SERVER_ENV='development'; `$env:LOG_LEVEL='warn'; Set-Location '$GoDir'; go run . *> '$GoOutLog'" `
    -WorkingDirectory $GoDir -WindowStyle Hidden -PassThru
  Write-Host ("    Go wrapper PID = " + $script:StartedGo.Id)
} else {
  Write-Host "==> Go already running on :$GoPort, reusing"
}

if (-not $rustRunning) {
  Write-Host "==> Starting Rust backend (SQLite): $RustDir"
  $script:StartedRust = Start-Process -FilePath "powershell" `
    -ArgumentList "-NoProfile","-Command","`$env:SERVER_PORT='$RustPort'; `$env:MEMORY_STORE_ENABLED='true'; `$env:RATE_LIMIT_ENABLED='false'; `$env:RUST_LOG='warn'; Set-Location '$RustDir'; cargo run --release *> '$RustOutLog'" `
    -WorkingDirectory $RustDir -WindowStyle Hidden -PassThru
  Write-Host ("    Rust wrapper PID = " + $script:StartedRust.Id)
} else {
  Write-Host "==> Rust already running on :$RustPort, reusing"
}

Write-Host "==> Waiting for services (up to 120s, Rust release build may take time)..."
$goReady   = Wait-Ready $GoBase 120
$rustReady = Wait-Ready $RustBase 120
Write-Host ("    Go ready=$goReady  Rust ready=$rustReady")
if (-not $goReady -or -not $rustReady) {
  Write-Host "!! Service not ready. Log tails:" -ForegroundColor Red
  if (Test-Path $GoOutLog)   { Get-Content $GoOutLog -Tail 30 -ErrorAction SilentlyContinue }
  if (Test-Path $RustOutLog) { Get-Content $RustOutLog -Tail 30 -ErrorAction SilentlyContinue }
  exit 1
}

Start-Sleep -Seconds 2
$script:GoProcessId   = Get-PortProcessId $GoPort
$script:RustProcessId = Get-PortProcessId $RustPort
Write-Host "    Go server PID:   $($script:GoProcessId)"
Write-Host "    Rust server PID: $($script:RustProcessId)"

# ---------------------------------------------------------------------------
# Login
# ---------------------------------------------------------------------------
function Login([string]$base) {
  $body = @{ username = $AdminUser; password = $AdminPass } | ConvertTo-Json -Compress
  try {
    $r = Invoke-RestMethod -Uri "$base/api/v1/auth/login" -Method POST -Body $body -ContentType "application/json" -TimeoutSec 10
    return [string]$r.data.token
  } catch {
    Write-Host "    Login failed: $($_.Exception.Message)" -ForegroundColor Red
    return $null
  }
}

Write-Host "==> Logging in as admin on both backends..."
$gToken = Login $GoBase
$rToken = Login $RustBase
Write-Host ("    Go token:   " + $(if($gToken){"ok"}else{"FAIL"}))
Write-Host ("    Rust token: " + $(if($rToken){"ok"}else{"FAIL"}))
if (-not $gToken -or -not $rToken) { Write-Host "!! Login failed" -ForegroundColor Red; exit 2 }

# ---------------------------------------------------------------------------
# Endpoint definitions - write POST bodies to temp files (avoid quoting issues)
# ---------------------------------------------------------------------------
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
$loginBody = @{username=$AdminUser;password=$AdminPass} | ConvertTo-Json -Compress
$createClusterBody = @{name="bench-cluster";provider="aws";region="us-east-1";version="1.28"} | ConvertTo-Json -Compress
$loginBodyFile = Join-Path $WorkDir "body-login.json"
$createClusterBodyFile = Join-Path $WorkDir "body-create-cluster.json"
[System.IO.File]::WriteAllText($loginBodyFile, $loginBody, $utf8NoBom)
[System.IO.File]::WriteAllText($createClusterBodyFile, $createClusterBody, $utf8NoBom)

$Endpoints = @(
  [pscustomobject]@{ Name = "POST /auth/login";    Method = "POST"; Path = "/api/v1/auth/login"; BodyFile = $loginBodyFile;        NeedsAuth = $false; Desc = "lightweight auth" }
  [pscustomobject]@{ Name = "GET /clusters";       Method = "GET";  Path = "/api/v1/clusters";    BodyFile = "";                      NeedsAuth = $true;  Desc = "list query" }
  [pscustomobject]@{ Name = "GET /clusters/1";     Method = "GET";  Path = "/api/v1/clusters/1";  BodyFile = "";                      NeedsAuth = $true;  Desc = "single detail" }
  [pscustomobject]@{ Name = "POST /clusters";      Method = "POST"; Path = "/api/v1/clusters";    BodyFile = $createClusterBodyFile;  NeedsAuth = $true;  Desc = "write create" }
  [pscustomobject]@{ Name = "GET /jobs page=1";    Method = "GET";  Path = "/api/v1/jobs?page=1&page_size=20"; BodyFile = "";            NeedsAuth = $true;  Desc = "paginated list" }
  [pscustomobject]@{ Name = "GET /metrics";         Method = "GET";  Path = "/metrics";            BodyFile = "";                      NeedsAuth = $false; Desc = "metrics no DB" }
)

Write-Host "==> Endpoints to benchmark: $($Endpoints.Count)"
foreach ($ep in $Endpoints) { Write-Host ("    - " + $ep.Name + " (" + $ep.Desc + ")") }

# ---------------------------------------------------------------------------
# Run Go bench.exe for one endpoint
# ---------------------------------------------------------------------------
$scriptResults = New-Object System.Collections.Generic.List[object]
$memSamples    = New-Object System.Collections.Generic.List[object]

function Invoke-BenchOnce {
  param(
    [string]$BaseUrl,
    [string]$Method,
    [string]$Path,
    [string]$BodyFile,
    [string]$Token,
    [int]$Concurrency,
    [int]$DurationSec
  )
  $benchArgs = @(
    "-base", $BaseUrl,
    "-method", $Method,
    "-path", $Path,
    "-c", $Concurrency,
    "-d", $DurationSec
  )
  if ($BodyFile) { $benchArgs += @("-bodyfile", $BodyFile) }
  if ($Token)    { $benchArgs += @("-token", $Token) }

  $outFile = Join-Path $WorkDir "bench-out.tmp"
  & $BenchExe @benchArgs > $outFile 2>&1
  $jsonLine = Get-Content $outFile -Tail 1 -ErrorAction SilentlyContinue
  try {
    return ($jsonLine | ConvertFrom-Json)
  } catch {
    Write-Host "    [WARN] Failed to parse: $jsonLine" -ForegroundColor Yellow
    return $null
  }
}

function Start-MemSampler {
  param([int]$ProcId, [string]$Lang, [string]$EpName, [int]$Conc, [int]$DurSec)
  $job = Start-Job -ScriptBlock {
    param($procId, $lang, $epName, $conc, $durSec)
    $samples = New-Object System.Collections.Generic.List[object]
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt ($durSec + 3)) {
      try {
        $p = Get-Process -Id $procId -ErrorAction Stop
        $samples.Add([pscustomobject]@{
          Lang = $lang
          Endpoint = $epName
          Concurrency = $conc
          TimeSec = [math]::Round($sw.Elapsed.TotalSeconds, 1)
          WorkingSetMB = [math]::Round($p.WorkingSet64 / 1MB, 1)
        })
      } catch { }
      Start-Sleep -Seconds 5
    }
    return $samples
  } -ArgumentList $ProcId, $Lang, $EpName, $Conc, $DurSec
  return $job
}

function Run-BenchmarkForLang {
  param([string]$Lang, [string]$BaseUrl, [string]$Token, [int]$ProcId)
  foreach ($ep in $Endpoints) {
    $epToken = if ($ep.NeedsAuth) { $Token } else { "" }
    foreach ($conc in $Concurrency) {
      for ($r = 1; $r -le $Rounds; $r++) {
        $padName = $ep.Name.PadRight(25)
        $msg = "  [$Lang] " + $padName + " conc=" + $conc.ToString().PadRight(3) + " round=" + $r + "  running " + $DurationSeconds + "s..."
        Write-Host $msg

        $memJob = Start-MemSampler -ProcId $ProcId -Lang $Lang -EpName $ep.Name -Conc $conc -DurSec $DurationSeconds
        $res = Invoke-BenchOnce -BaseUrl $BaseUrl -Method $ep.Method -Path $ep.Path -BodyFile $ep.BodyFile -Token $epToken -Concurrency $conc -DurationSec $DurationSeconds
        $memResult = Receive-Job -Job $memJob -Wait -AutoRemoveJob
        if ($memResult) { foreach ($s in $memResult) { $script:memSamples.Add($s) } }

        if ($res) {
          $failRate = if ($res.total_req -gt 0) { [math]::Round(100.0 * $res.fail_req / $res.total_req, 2) } else { 0 }
          $scriptResults.Add([pscustomobject]@{
            Lang=$Lang; Endpoint=$ep.Name; Concurrency=$conc; DurationSec=$DurationSeconds
            TotalReqs=$res.total_req; SuccessReqs=$res.success_req; FailReqs=$res.fail_req
            FailRatePct=$failRate; ReqPerSec=[math]::Round($res.req_per_sec,1)
            AvgMs=[math]::Round($res.avg_ms,2); P50Ms=$res.p50_ms; P90Ms=$res.p90_ms; P99Ms=$res.p99_ms; MaxMs=$res.max_ms
          })
          Write-Host ("         -> " + [math]::Round($res.req_per_sec,1) + " req/s, avg=" + [math]::Round($res.avg_ms,2) + "ms, P50=" + $res.p50_ms + "ms, P99=" + $res.p99_ms + "ms, fail=" + $failRate + "%")
        } else {
          Write-Host "         -> [FAILED to get results]" -ForegroundColor Red
        }
        Write-Host "         cooling down 10s..."
        Start-Sleep -Seconds 10
      }
    }
  }
}

# ---------------------------------------------------------------------------
# Run benchmark: Go first all endpoints, then Rust
# ---------------------------------------------------------------------------
Write-Host ""
Write-Host "========================================"
Write-Host "  Benchmark: Go backend (:$GoPort)"
Write-Host "========================================"
Run-BenchmarkForLang -Lang "Go" -BaseUrl $GoBase -Token $gToken -ProcId $script:GoProcessId

Write-Host ""
Write-Host "========================================"
Write-Host "  Benchmark: Rust backend (:$RustPort)"
Write-Host "========================================"
Run-BenchmarkForLang -Lang "Rust" -BaseUrl $RustBase -Token $rToken -ProcId $script:RustProcessId

# ---------------------------------------------------------------------------
# Output CSV
# ---------------------------------------------------------------------------
$scriptResults | Export-Csv -Path $CsvPath -NoTypeInformation -Encoding UTF8
if ($memSamples.Count -gt 0) {
  $memSamples | Export-Csv -Path $MemCsvPath -NoTypeInformation -Encoding UTF8
}

Write-Host ""
Write-Host "==> Benchmark complete, $($scriptResults.Count) records"
Write-Host "    CSV: $CsvPath"
Write-Host "    Memory CSV: $MemCsvPath"

# ---------------------------------------------------------------------------
# Summary report
# ---------------------------------------------------------------------------
$sw = New-Object System.Text.StringBuilder
[void]$sw.AppendLine("P4-03 Performance Benchmark Report (Go vs Rust)")
[void]$sw.AppendLine("Generated: " + (Get-Date).ToString("yyyy-MM-dd HH:mm:ss zzz"))
[void]$sw.AppendLine("")

[void]$sw.AppendLine("== Test Environment ==")
$cpu = Get-CimInstance -ClassName Win32_Processor
[void]$sw.AppendLine("CPU: $($cpu.Name) ($($cpu.NumberOfCores) cores / $($cpu.NumberOfLogicalProcessors) threads)")
$osInfo = Get-CimInstance -ClassName Win32_OperatingSystem
[void]$sw.AppendLine("OS: $($osInfo.Caption) Build $($osInfo.BuildNumber)")
[void]$sw.AppendLine("RAM: $([math]::Round($osInfo.TotalVisibleMemorySize/1MB,1)) GB")
[void]$sw.AppendLine("Go: $(go version)")
$rustcVer = (rustc --version) -replace 'rustc ', ''
$cargoVer = (cargo --version) -replace 'cargo ', ''
[void]$sw.AppendLine("Rust: rustc $rustcVer / cargo $cargoVer")
[void]$sw.AppendLine("Rust build mode: release (cargo run --release)")
[void]$sw.AppendLine("Go storage: in-memory store (MEMORY_STORE_ENABLED=true)")
[void]$sw.AppendLine("Rust storage: SQLite (sqlite:metaclouds.db)")
[void]$sw.AppendLine("")

[void]$sw.AppendLine("== Test Method ==")
[void]$sw.AppendLine("Endpoints: $($Endpoints.Count)")
foreach ($ep in $Endpoints) { [void]$sw.AppendLine("  - $($ep.Method) $($ep.Path) ($($ep.Desc))") }
[void]$sw.AppendLine("Concurrency: $($Concurrency -join ', ')")
[void]$sw.AppendLine("Duration per round: $DurationSeconds seconds")
[void]$sw.AppendLine("Rounds per endpoint/concurrency: $Rounds")
[void]$sw.AppendLine("Cooldown between rounds: 10 seconds")
[void]$sw.AppendLine("Memory sampling: every 5 seconds (WorkingSet)")
[void]$sw.AppendLine("Order: Go all endpoints first, then Rust all endpoints")
[void]$sw.AppendLine("")

# Throughput comparison
[void]$sw.AppendLine("== Throughput Comparison (req/s) ==")
[void]$sw.AppendLine("Endpoint,Concurrency,Go_req/s,Rust_req/s,Rust_Go_ratio_pct")
foreach ($ep in $Endpoints) {
  foreach ($conc in $Concurrency) {
    $g = $scriptResults | Where-Object { $_.Lang -eq "Go" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    $r = $scriptResults | Where-Object { $_.Lang -eq "Rust" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    if ($g -and $r) {
      $ratio = if ($g.ReqPerSec -gt 0) { [math]::Round(100.0 * $r.ReqPerSec / $g.ReqPerSec, 1) } else { 0 }
      [void]$sw.AppendLine("$($ep.Name),$conc,$($g.ReqPerSec),$($r.ReqPerSec),$ratio")
    }
  }
}
[void]$sw.AppendLine("")

# Latency comparison
[void]$sw.AppendLine("== Latency Comparison (ms) ==")
[void]$sw.AppendLine("Endpoint,Conc,Go_Avg,Go_P50,Go_P90,Go_P99,Rust_Avg,Rust_P50,Rust_P90,Rust_P99")
foreach ($ep in $Endpoints) {
  foreach ($conc in $Concurrency) {
    $g = $scriptResults | Where-Object { $_.Lang -eq "Go" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    $r = $scriptResults | Where-Object { $_.Lang -eq "Rust" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    if ($g -and $r) {
      [void]$sw.AppendLine("$($ep.Name),$conc,$($g.AvgMs),$($g.P50Ms),$($g.P90Ms),$($g.P99Ms),$($r.AvgMs),$($r.P50Ms),$($r.P90Ms),$($r.P99Ms)")
    }
  }
}
[void]$sw.AppendLine("")

# Error rate
[void]$sw.AppendLine("== Error Rate Comparison ==")
[void]$sw.AppendLine("Endpoint,Conc,Go_fails,Go_fail_pct,Rust_fails,Rust_fail_pct")
foreach ($ep in $Endpoints) {
  foreach ($conc in $Concurrency) {
    $g = $scriptResults | Where-Object { $_.Lang -eq "Go" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    $r = $scriptResults | Where-Object { $_.Lang -eq "Rust" -and $_.Endpoint -eq $ep.Name -and $_.Concurrency -eq $conc } | Select-Object -First 1
    if ($g -and $r) {
      [void]$sw.AppendLine("$($ep.Name),$conc,$($g.FailReqs),$($g.FailRatePct),$($r.FailReqs),$($r.FailRatePct)")
    }
  }
}
[void]$sw.AppendLine("")

# Memory comparison
[void]$sw.AppendLine("== Memory Comparison (WorkingSet MB) ==")
if ($memSamples.Count -gt 0) {
  $goMem = $memSamples | Where-Object { $_.Lang -eq "Go" }
  $rustMem = $memSamples | Where-Object { $_.Lang -eq "Rust" }
  if ($goMem) {
    $goMemAvg = [math]::Round(($goMem | Measure-Object -Property WorkingSetMB -Average).Average, 1)
    $goMemPeak = ($goMem | Measure-Object -Property WorkingSetMB -Maximum).Maximum
    [void]$sw.AppendLine("Go  avg memory: $goMemAvg MB,  peak: $goMemPeak MB")
  }
  if ($rustMem) {
    $rustMemAvg = [math]::Round(($rustMem | Measure-Object -Property WorkingSetMB -Average).Average, 1)
    $rustMemPeak = ($rustMem | Measure-Object -Property WorkingSetMB -Maximum).Maximum
    [void]$sw.AppendLine("Rust avg memory: $rustMemAvg MB,  peak: $rustMemPeak MB")
  }
  if ($goMem -and $rustMem -and $rustMemAvg -gt 0 -and $goMemAvg -gt 0) {
    [void]$sw.AppendLine("Rust/Go avg memory ratio: $([math]::Round(100.0*$rustMemAvg/$goMemAvg,1))%")
  }
} else {
  [void]$sw.AppendLine("(no memory samples collected)")
}

$summaryText = $sw.ToString()
$summaryText | Out-File -FilePath $SummaryPath -Encoding UTF8
Write-Host ""
Write-Host $summaryText
Write-Host ("==> Summary: " + $SummaryPath)

# ---------------------------------------------------------------------------
# Cleanup
# ---------------------------------------------------------------------------
if (-not $KeepServicesRunning) {
  Write-Host ""
  Write-Host "==> Stopping both backends..."
  if ($script:StartedGo)   { Write-Host "    Stopping Go wrapper PID=$($script:StartedGo.Id)";   Stop-TrackedProcess $script:StartedGo }
  if ($script:StartedRust) { Write-Host "    Stopping Rust wrapper PID=$($script:StartedRust.Id)"; Stop-TrackedProcess $script:StartedRust }
  Start-Sleep -Seconds 2
  foreach ($p in @($GoPort, $RustPort)) {
    $procId = Get-PortProcessId $p
    if ($procId) {
      Write-Host "    Cleaning port $p residual PID=$procId"
      try { taskkill /PID $procId /T /F 2>$null | Out-Null } catch { }
    }
  }
  Write-Host "==> Both backends stopped"
} else {
  Write-Host "==> -KeepServicesRunning set, backends left running"
}
