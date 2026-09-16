<#
.SYNOPSIS
    抓取 Go 后端 API Golden 基线快照（用于 Rust 重构对等验收）。

.DESCRIPTION
    本脚本完成以下工作：
      1. 检查 Go 工具链与输出目录；探测目标端口是否已被健康的 Go 后端占用。
      2. （按需）在内存模式下后台启动 Go 后端（go run .），并轮询 /health 直到就绪。
      3. 从 api/routes.go 用正则解析出全部 /api/v1 下的路由（含路由组前缀与 Vue3 别名路由），
         跳过静态文件、/health、/metrics、/api/docs 等非业务路由。
      4. 对每个端点抓取 2~3 个场景：
           - unauthenticated : 不带 Authorization 头调用（公开端点如 login 除外）
           - admin           : 以 admin 登录后带 Bearer Token 调用
           - regular_user    : 仅写操作（POST/PUT/DELETE）；尝试用 /auth/register 建普通用户，
                               若注册关闭（404）则自动跳过该场景。
      5. 每个场景落盘一个 JSON 快照，并生成 index.json 与 summary.md。

    前置条件：
      - Go 后端必须已 `go build` 通过，或可直接在 metaclouds-backend 目录下 `go run .`。
      - 脚本默认在 D:\YCYD\metaclouds-backend 下启动，使用内存存储（不依赖外部数据库）。

.PARAMETER Port
    监听端口，默认 8000。

.PARAMETER OutputDir
    快照输出目录，默认 D:\YCYD\docs\golden-api-baseline\。

.PARAMETER SkipStart
    不启动后端，假定目标端口上已有可用的 Go 后端（仅做健康检查）。

.PARAMETER BackendDir
    Go 后端工作目录，默认 D:\YCYD\metaclouds-backend。

.PARAMETER RoutesFile
    路由定义文件，默认 <BackendDir>\api\routes.go。

.PARAMETER AdminUser
    管理员用户名，默认 admin。

.PARAMETER AdminPass
    管理员密码，默认 Admin@123456。

.EXAMPLE
    .\capture-go-api-baseline.ps1
    # 自动启动后端、抓取全部端点快照到默认目录。

.EXAMPLE
    .\capture-go-api-baseline.ps1 -SkipStart -Port 8000
    # 复用已经在 8000 端口跑着的后端，只做抓取。

.EXAMPLE
    .\capture-go-api-baseline.ps1 -OutputDir D:\tmp\baseline -Verbose
    # 输出到自定义目录，并打印详细过程。
#>

[CmdletBinding()]
param(
    [int]$Port = 8000,
    [string]$OutputDir = "D:\YCYD\docs\golden-api-baseline",
    [switch]$SkipStart,
    [string]$BackendDir = "D:\YCYD\metaclouds-backend",
    [string]$RoutesFile = "D:\YCYD\metaclouds-backend\api\routes.go",
    [string]$AdminUser = "admin",
    [string]$AdminPass = "Admin@123456"
)

$ErrorActionPreference = 'Continue'
$ProgressPreference    = 'SilentlyContinue'

# ---------------------------------------------------------------------------
# 0. 公共工具函数
# ---------------------------------------------------------------------------

function Join-ApiPath {
    # 拼接 Gin 路由组前缀与子路径，规范斜杠。
    param([string]$A, [string]$B)
    $a = $A.TrimEnd('/')
    if ([string]::IsNullOrEmpty($B)) { return $a }
    $b = $B.TrimStart('/')
    if ([string]::IsNullOrEmpty($b)) { return $a }
    $combined = "$a/$b"
    if (-not $combined.StartsWith('/')) { $combined = '/' + $combined }
    return $combined
}

function Get-MinimumBody {
    # 根据方法与路径推断最小合法 JSON 请求体。无法推断时返回 "{}"。
    param([string]$Method, [string]$Path)
    if ($Method -notin @('POST', 'PUT', 'PATCH')) { return $null }

    switch -Regex ($Path) {
        '/auth/login'              { return '{"username":"' + $AdminUser + '","password":"' + $AdminPass + '"}' }
        '/auth/register'           { return '{"username":"goldenuser","password":"User@123456","email":"golden@test.local","role":"user"}' }
        '/auth/(refresh|logout)'   { return '{}' }
        '/clusters$'               { return '{"name":"golden-cluster","description":"golden baseline cluster"}' }
        '/clusters/.+'             { return '{"name":"golden-cluster-updated","description":"updated"}' }
        '/jobs$'                   { return '{"name":"golden-job","cluster_id":1,"image":"busybox:latest","command":["echo","hi"]}' }
        '/jobs/.+/(cancel|submit)' { return '{}' }
        '/jobs/.+'                 { return '{"name":"golden-job-updated"}' }
        '/resources$'              { return '{"name":"golden-resource","type":"cpu","total":100}' }
        '/resources/.+'            { return '{"name":"golden-resource-updated"}' }
        '/tenants$'                { return '{"name":"golden-tenant","description":"golden tenant"}' }
        '/tenants/.+'              { return '{"name":"golden-tenant-updated"}' }
        '/acceleration$'           { return '{"name":"golden-suite","type":"cuda","version":"12.2"}' }
        '/acceleration/.+'         { return '{"name":"golden-suite-updated"}' }
        '/security/policies$'      { return '{"name":"golden-policy","rule":"deny","priority":1}' }
        '/security/policies/.+'   { return '{"name":"golden-policy-updated"}' }
        '/gpu/devices$'            { return '{"name":"golden-gpu","model":"A100","vendor":"nvidia","total":8}' }
        '/gpu/devices/.+'          { return '{"name":"golden-gpu-updated"}' }
        '/gpu/allocations$'        { return '{"device_id":1,"job_id":1,"count":1}' }
        '/gpus$'                   { return '{"name":"golden-gpu","model":"A100","vendor":"nvidia","total":8}' }
        '/gpus/allocations$'       { return '{"device_id":1,"job_id":1,"count":1}' }
        '/gpus/.+'                 { return '{"name":"golden-gpu-updated"}' }
        '/partitions$'             { return '{"name":"golden-partition","cluster_id":1}' }
        '/partitions/.+/priority'    { return '{"priority":10}' }
        '/partitions/.+/max-runtime' { return '{"max_runtime_seconds":3600}' }
        '/partitions/.+/permissions' { return '{"user_id":1,"permission":"read"}' }
        '/partitions/.+'           { return '{"name":"golden-partition-updated"}' }
        '/quotas$'                 { return '{"tenant_id":1,"cpu_limit":100,"memory_limit":"200Gi"}' }
        '/quotas/check'            { return '{"resource_type":"cpu","quantity":1}' }
        '/quotas/.+'               { return '{"cpu_limit":200}' }
        '/schedulers$'             { return '{"name":"golden-scheduler","type":"kube-scheduler","endpoint":"http://localhost:9090"}' }
        '/schedulers/.+/sync'      { return '{}' }
        '/schedulers/.+'           { return '{"name":"golden-scheduler-updated"}' }
        '/topology/score'          { return '{"node_a":"n1","node_b":"n2","bandwidth_gbps":400}' }
        '/topology(/nodes)?$'       { return '{"name":"golden-topo-node","cluster_id":1,"ip":"10.0.0.1"}' }
        '/topology(/nodes)?/.+'     { return '{"name":"golden-topo-node-updated"}' }
        '/datasets$'               { return '{"name":"golden-dataset","description":"golden dataset"}' }
        '/datasets/.+/caches$'     { return '{"name":"golden-cache","image":"nginx:latest","size_gb":10}' }
        '/datasets/.+/fluid-caches$' { return '{"name":"golden-cache","image":"nginx:latest","size_gb":10}' }
        '/datasets/.+/caches/.+/(enable|disable|prefetch)' { return '{}' }
        '/fluid-caches/.+/(enable|disable|prefetch)'       { return '{}' }
        '/fluid-caches/.+'         { return '{"name":"golden-cache-updated"}' }
        '/datasets/.+'             { return '{"name":"golden-dataset-updated"}' }
        '/checkpoints$'            { return '{"job_id":1,"path":"/tmp/ckpt","size_bytes":1024}' }
        '/monitoring/alerts/.+/resolve' { return '{}' }
        default                    { return '{}' }
    }
}

function ConvertTo-ParsedJson {
    # 尝试把字符串解析为对象；失败则原样返回字符串。
    param([string]$Raw)
    if ([string]::IsNullOrEmpty($Raw)) { return $null }
    try { return ($Raw | ConvertFrom-Json -Depth 20 -ErrorAction Stop) }
    catch { return $Raw }
}

function Save-JsonUtf8 {
    # 以无 BOM 的 UTF-8 写 JSON。
    param([string]$Path, $Obj)
    $json = $Obj | ConvertTo-Json -Depth 30
    [System.IO.File]::WriteAllText($Path, $json, [System.Text.UTF8Encoding]::new($false))
}

function ConvertTo-HeaderDict {
    # 兼容 PS5.1 HttpWebResponse.Headers (AllKeys) 与 PS7 字典 (Keys)。
    param($Headers)
    $dict = [ordered]@{}
    if ($null -eq $Headers) { return $dict }
    $keys = @()
    if ($Headers.PSObject.Properties['AllKeys']) { $keys = @($Headers.AllKeys) }
    elseif ($Headers.Keys) { $keys = @($Headers.Keys) }
    foreach ($k in $keys) {
        if ([string]::IsNullOrEmpty($k)) { continue }
        try { $dict[$k] = [string]$Headers[$k] } catch {}
    }
    return $dict
}

function Invoke-GoldenRequest {
    # 统一的请求发送器：无论 HTTP 状态码如何都返回结构化结果，不中断抓取流程。
    param(
        [string]$Method,
        [string]$Url,
        [hashtable]$Headers,
        [string]$Body,
        [int]$TimeoutSec = 20
    )
    $out = [ordered]@{
        status_code      = $null
        response_headers = @{}
        response_body    = $null
        transport_error = $null
    }
    $params = @{
        Uri             = $Url
        Method          = $Method
        TimeoutSec      = $TimeoutSec
        UseBasicParsing = $true
    }
    if ($Headers -and $Headers.Count -gt 0) { $params['Headers'] = $Headers }
    if ($Body -and $Method -in @('POST','PUT','PATCH','DELETE')) {
        $params['Body'] = $Body
        $params['ContentType'] = 'application/json'
    }
    $isPS7 = ($PSVersionTable.PSVersion.Major -ge 7)
    if ($isPS7) { $params['SkipHttpErrorCheck'] = $true }

    try {
        $resp = Invoke-WebRequest @params
        $out.status_code = [int]$resp.StatusCode
        $out.response_headers = ConvertTo-HeaderDict $resp.Headers
        $out.response_body = ConvertTo-ParsedJson $resp.Content
    } catch {
        if ($isPS7) {
            $out.transport_error = $_.Exception.Message
        } else {
            $errResp = $_.Exception.Response
            if ($null -ne $errResp) {
                try { $out.status_code = [int]$errResp.StatusCode } catch {}
                try { $out.response_headers = ConvertTo-HeaderDict $errResp.Headers } catch {}
                try {
                    $stream = $errResp.GetResponseStream()
                    $reader = New-Object System.IO.StreamReader($stream)
                    $rawBody = $reader.ReadToEnd()
                    $reader.Close()
                    $out.response_body = ConvertTo-ParsedJson $rawBody
                } catch {
                    $out.transport_error = "read body failed: " + $_.Exception.Message
                }
            } else {
                $out.transport_error = $_.Exception.Message
            }
        }
    }
    return [pscustomobject]$out
}

function Test-BackendHealth {
    param([int]$Port)
    try {
        $h = Invoke-WebRequest -Uri "http://localhost:$Port/health" -UseBasicParsing -TimeoutSec 2 -ErrorAction Stop
        return ($h.StatusCode -eq 200)
    } catch { return $false }
}

# ---------------------------------------------------------------------------
# 1. 环境准备
# ---------------------------------------------------------------------------

Write-Host "==> [1/7] 环境检查" -ForegroundColor Cyan

# 1.1 Go 工具链
try {
    $goVer = & go version 2>&1
    Write-Host ("    Go 工具链: " + $goVer)
} catch {
    Write-Error "未检测到 go 命令，请先安装 Go 工具链并加入 PATH。"
    exit 1
}

# 1.2 输出目录
if (-not (Test-Path -LiteralPath $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}
Write-Host ("    快照输出目录: " + $OutputDir)

# 1.3 端口 / 后端探测
$baseUrl = "http://localhost:$Port"
$startedByUs = $false

if ($SkipStart) {
    Write-Host "    -SkipStart 已指定，跳过启动逻辑，假定后端已在 $Port 端口运行。"
    if (-not (Test-BackendHealth -Port $Port)) {
        Write-Error "健康检查失败：$baseUrl/health 未返回 200。请先启动后端。"
        exit 1
    }
} else {
    $healthy = Test-BackendHealth -Port $Port
    if ($healthy) {
        Write-Host "    端口 $Port 已有健康的 Go 后端（/health=200），复用现有进程。" -ForegroundColor Green
    } else {
        $listen = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue
        if ($listen) {
            Write-Error ("端口 $Port 被其他进程占用，但 /health 未通过健康检查。" +
                         " 请确认是否为 Go 后端进程；若是其他服务，请改用 -Port 指定其他端口。")
            exit 1
        }
        Write-Host "==> [2/7] 后台启动 Go 后端 (go run .)" -ForegroundColor Cyan
        if (-not (Test-Path -LiteralPath $BackendDir)) {
            Write-Error "后端目录不存在: $BackendDir"
            exit 1
        }

        $env:MEMORY_STORE_ENABLED      = "true"
        $env:USE_SQLITE                = "true"
        $env:JWT_SECRET                = "golden-baseline-secret-key-min-32-chars-long"
        $env:DEFAULT_ADMIN_PASSWORD    = $AdminPass
        $env:SERVER_PORT               = "$Port"
        $env:ALLOW_PUBLIC_REGISTRATION = "true"

        $logOut = Join-Path $env:TEMP "metaclouds-baseline-stdout.log"
        $logErr = Join-Path $env:TEMP "metaclouds-baseline-stderr.log"

        $proc = Start-Process -FilePath "go" `
            -ArgumentList "run", "." `
            -WorkingDirectory $BackendDir `
            -PassThru -WindowStyle Hidden `
            -RedirectStandardOutput $logOut `
            -RedirectStandardError  $logErr

        $startedByUs = $true
        Write-Host ("    已启动 go run (PID=" + $proc.Id + ")，日志: " + $logErr)

        $deadline = (Get-Date).AddSeconds(30)
        $ready = $false
        while ((Get-Date) -lt $deadline) {
            if (Test-BackendHealth -Port $Port) { $ready = $true; break }
            Start-Sleep -Milliseconds 500
        }
        if (-not $ready) {
            Write-Error ("后端 30 秒内未就绪。请查看日志: " + $logErr)
            exit 1
        }
        Write-Host "    健康检查通过。" -ForegroundColor Green
    }
}

# ---------------------------------------------------------------------------
# 2. 路由提取
# ---------------------------------------------------------------------------

Write-Host "==> [3/7] 解析路由文件: $RoutesFile" -ForegroundColor Cyan

if (-not (Test-Path -LiteralPath $RoutesFile)) {
    Write-Error "路由文件不存在: $RoutesFile"
    exit 1
}
$lines = Get-Content -LiteralPath $RoutesFile

# 2.1 建立 组变量 -> 前缀 映射
$groupPrefix = @{}
$groupPrefix['r'] = ''   # gin.Engine 根

foreach ($line in $lines) {
    # 匹配:  name := parent.Group("/prefix")
    if ($line -match '^\s*(\w+)\s*:?=\s*(\w+)\.Group\("([^"]*)"') {
        $var    = $matches[1]
        $parent = $matches[2]
        $prefix = $matches[3]
        $parentPrefix = if ($groupPrefix.ContainsKey($parent)) { $groupPrefix[$parent] } else { '' }
        $groupPrefix[$var] = Join-ApiPath $parentPrefix $prefix
    }
}
Write-Verbose ("    识别到的路由组: " + (($groupPrefix.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join '; '))

# 2.2 跳过的非业务路由
$skipExact = @(
    '/', '/static', '/assets', '/app', '/backend',
    '/health', '/metrics', '/api/docs'
)

$routes = New-Object System.Collections.Generic.List[object]
$seen   = @{}

foreach ($line in $lines) {
    if ($line -match '^\s*//') { continue }
    if ($line -match '^\s*(\w+)\.(GET|POST|PUT|DELETE|PATCH)\("([^"]+)"') {
        $var    = $matches[1]
        $method = $matches[2]
        $sub    = $matches[3]
        if (-not $groupPrefix.ContainsKey($var)) { continue }
        $full = Join-ApiPath $groupPrefix[$var] $sub

        if ($skipExact -contains $full) { continue }
        if ($full -match '^/(static|assets|app|backend|health|metrics)') { continue }
        if ($full -like '/api/docs*') { continue }

        $key = "$method $full"
        if ($seen.ContainsKey($key)) { continue }
        $seen[$key] = $true

        $concrete = $full -replace ':([A-Za-z]\w*)', '1'

        $routes.Add([pscustomobject]@{
            Method   = $method
            Path     = $full
            Concrete = $concrete
            IsWrite  = ($method -in @('POST','PUT','PATCH','DELETE'))
        })
    }
}

Write-Host ("    共解析出 " + $routes.Count + " 个唯一 API 端点。")

# ---------------------------------------------------------------------------
# 3. 登录：管理员 & 普通用户
# ---------------------------------------------------------------------------

Write-Host "==> [4/7] 登录获取 Token" -ForegroundColor Cyan

$adminToken = $null
$loginBody = '{"username":"' + $AdminUser + '","password":"' + $AdminPass + '"}'
$r = Invoke-GoldenRequest -Method POST -Url "$baseUrl/api/v1/auth/login" `
    -Headers @{} -Body $loginBody -TimeoutSec 10
if ($r.status_code -eq 200 -and $r.response_body -and $r.response_body.data -and $r.response_body.data.token) {
    $adminToken = $r.response_body.data.token
    Write-Host "    admin 登录成功。" -ForegroundColor Green
} else {
    Write-Warning ("admin 登录失败 status=" + $r.status_code + " body=" + ($r.response_body | Out-String))
}

if (-not $adminToken) {
    Write-Error "无法获取 admin Token，脚本终止。"
    exit 1
}

$userToken = $null
try {
    $regBody = '{"username":"goldenuser","password":"User@123456","email":"golden@test.local","role":"user"}'
    $rr = Invoke-GoldenRequest -Method POST -Url "$baseUrl/api/v1/auth/register" `
        -Headers @{} -Body $regBody -TimeoutSec 10
    if ($rr.status_code -eq 200 -or $rr.status_code -eq 201) {
        $lu = Invoke-GoldenRequest -Method POST -Url "$baseUrl/api/v1/auth/login" `
            -Headers @{} -Body '{"username":"goldenuser","password":"User@123456"}' -TimeoutSec 10
        if ($lu.status_code -eq 200 -and $lu.response_body -and $lu.response_body.data -and $lu.response_body.data.token) {
            $userToken = $lu.response_body.data.token
            Write-Host "    普通用户 goldenuser 登录成功（启用 403 场景）。" -ForegroundColor Green
        }
    } else {
        Write-Host ("    注册接口返回 " + $rr.status_code + "，跳过 regular_user 场景。") -ForegroundColor Yellow
    }
} catch {
    Write-Warning ("普通用户注册/登录异常，跳过 regular_user 场景: " + $_.Exception.Message)
}

# ---------------------------------------------------------------------------
# 4. 逐端点抓取
# ---------------------------------------------------------------------------

Write-Host "==> [5/7] 开始抓取快照" -ForegroundColor Cyan

$capturedAt = (Get-Date).ToString("yyyy-MM-ddTHH:mm:ssK")
$indexEndpoints   = New-Object System.Collections.Generic.List[object]
$totalSnapshots   = 0
$successSnapshots = 0
$failedSnapshots   = 0

foreach ($route in $routes) {
    $method   = $route.Method
    $path     = $route.Path
    $concrete = $route.Concrete
    $url      = $baseUrl + $concrete
    $body     = Get-MinimumBody -Method $method -Path $path

    $epRecord = [ordered]@{
        method    = $method
        path      = $path
        concrete  = $concrete
        is_write  = $route.IsWrite
        scenarios = [ordered]@{}
    }

    $scenarioList = New-Object System.Collections.Generic.List[object]
    $scenarioList.Add([pscustomobject]@{ Name='unauthenticated'; Token=$null })
    $scenarioList.Add([pscustomobject]@{ Name='admin';           Token=$adminToken })
    if ($route.IsWrite -and $userToken) {
        $scenarioList.Add([pscustomobject]@{ Name='regular_user';  Token=$userToken })
    }

    foreach ($sc in $scenarioList) {
        $headers = @{}
        if ($sc.Token) { $headers['Authorization'] = "Bearer $($sc.Token)" }

        $reqHeadersForLog = [ordered]@{}
        foreach ($k in $headers.Keys) { $reqHeadersForLog[$k] = $headers[$k] }

        $resp = Invoke-GoldenRequest -Method $method -Url $url -Headers $headers -Body $body -TimeoutSec 20

        $parsedReqBody = $null
        if ($body) { $parsedReqBody = ConvertTo-ParsedJson $body }

        $snapshot = [ordered]@{
            method           = $method
            url              = $url
            scenario         = $sc.Name
            request_headers  = $reqHeadersForLog
            request_body     = $parsedReqBody
            status_code      = $resp.status_code
            response_headers = $resp.response_headers
            response_body    = $resp.response_body
            transport_error  = $resp.transport_error
            captured_at      = $capturedAt
        }

        $safePath = ($concrete -replace '/', '_').Trim('_')
        $fileName = ("{0}_{1}_{2}.json" -f $method, $safePath, $sc.Name)
        $filePath = Join-Path $OutputDir $fileName

        try {
            Save-JsonUtf8 -Path $filePath -Obj $snapshot
            $successSnapshots++
            $epRecord.scenarios[$sc.Name] = [ordered]@{
                file        = $fileName
                status_code = $resp.status_code
                error       = $resp.transport_error
            }
        } catch {
            $failedSnapshots++
            $epRecord.scenarios[$sc.Name] = [ordered]@{
                file        = $null
                status_code = $resp.status_code
                error       = $_.Exception.Message
            }
        }
        $totalSnapshots++
    }

    $indexEndpoints.Add([pscustomobject]$epRecord)
}

# ---------------------------------------------------------------------------
# 5. 生成 index.json 与 summary.md
# ---------------------------------------------------------------------------

Write-Host "==> [6/7] 生成索引与摘要" -ForegroundColor Cyan

$statusCount = @{}
foreach ($ep in $indexEndpoints) {
    foreach ($prop in $ep.scenarios.GetEnumerator()) {
        $code = [string]$prop.Value.status_code
        if (-not $statusCount.ContainsKey($code)) { $statusCount[$code] = 0 }
        $statusCount[$code]++
    }
}

$indexObj = [ordered]@{
    captured_at         = $capturedAt
    base_url            = $baseUrl
    total_endpoints     = $indexEndpoints.Count
    total_snapshots     = $totalSnapshots
    success_snapshots   = $successSnapshots
    failed_snapshots    = $failedSnapshots
    status_code_summary = $statusCount
    endpoints           = $indexEndpoints
}
Save-JsonUtf8 -Path (Join-Path $OutputDir 'index.json') -Obj $indexObj

$sb = New-Object System.Text.StringBuilder
[void]$sb.AppendLine("# Go API Golden 基线快照")
[void]$sb.AppendLine("")
[void]$sb.AppendLine("- 抓取时间: $capturedAt")
[void]$sb.AppendLine("- 基地址:   $baseUrl")
[void]$sb.AppendLine("- 端点数:   $($indexEndpoints.Count)")
[void]$sb.AppendLine("- 快照数:   $totalSnapshots (成功 $successSnapshots / 失败 $failedSnapshots)")
[void]$sb.AppendLine("")
[void]$sb.AppendLine("## 状态码分布")
[void]$sb.AppendLine("")
[void]$sb.AppendLine("| 状态码 | 次数 |")
[void]$sb.AppendLine("|---|---|")
foreach ($k in ($statusCount.Keys | Sort-Object)) {
    [void]$sb.AppendLine("| $k | $($statusCount[$k]) |")
}
[void]$sb.AppendLine("")
[void]$sb.AppendLine("## 端点明细")
[void]$sb.AppendLine("")
[void]$sb.AppendLine("| 方法 | 路径 | 未认证 | 管理员 | 普通用户 | 备注 |")
[void]$sb.AppendLine("|---|---|---|---|---|---|")
foreach ($ep in $indexEndpoints) {
    $unauth = if ($ep.scenarios['unauthenticated']) { $ep.scenarios['unauthenticated'].status_code } else { '-' }
    $admin  = if ($ep.scenarios['admin'])           { $ep.scenarios['admin'].status_code }           else { '-' }
    $user   = if ($ep.scenarios['regular_user'])    { $ep.scenarios['regular_user'].status_code }    else { '-' }
    $note   = if ($ep.is_write) { 'write' } else { 'read' }
    [void]$sb.AppendLine("| $($ep.method) | ``$($ep.path)`` | $unauth | $admin | $user | $note |")
}
[System.IO.File]::WriteAllText((Join-Path $OutputDir 'summary.md'), $sb.ToString(), [System.Text.UTF8Encoding]::new($false))

# ---------------------------------------------------------------------------
# 6. 收尾
# ---------------------------------------------------------------------------

Write-Host "==> [7/7] 完成" -ForegroundColor Cyan
Write-Host ("    总端点数:   " + $indexEndpoints.Count)
Write-Host ("    成功快照:   " + $successSnapshots)
Write-Host ("    失败快照:   " + $failedSnapshots)
Write-Host ("    输出目录:   " + $OutputDir)

if ($startedByUs) {
    Write-Host ""
    Write-Host "提示: 本次脚本启动的 Go 后端 (go run) 仍在运行中。" -ForegroundColor Yellow
    Write-Host "      如需停止，请在任务管理器中结束对应的 go / metaclouds-backend 进程，" -ForegroundColor Yellow
    Write-Host "      或直接关闭弹出的控制台窗口。脚本不会自动 kill 它。" -ForegroundColor Yellow
}
