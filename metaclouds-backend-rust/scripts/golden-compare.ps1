#Requires -Version 5.1
<#
.SYNOPSIS
  P4-01 Golden 全量对等回归对比脚本（Go :8000 vs Rust :8001）。
.DESCRIPTION
  对端点清单执行三档对比：
    L1 — 状态码 + 信封顶层字段（{success,data,timestamp} 成功 / {success,error} 失败）
    L2 — GET 端点 data 字段形状（键集合 + 类型，不对比具体值；分页结构识别）
    L3 — RBAC 矩阵：admin / manager / user / 未认证 四身份下的状态码
  RBAC 中间件先于 handler 校验，故 L3 用空 body 即可判定 401/403 vs 放行。
  Go 无用户管理端点且注册强制 role=user，Go 侧 manager 列标记 N/A。
.PARAMETER OutCsv
  结果 CSV 路径。
#>
param(
  [string]$GoBase   = "http://localhost:8000",
  [string]$RustBase = "http://localhost:8001",
  [string]$TokensJson = "D:\YCYD\metaclouds-backend-rust\scripts\p4work\tokens.json",
  [string]$OutCsv   = "D:\YCYD\metaclouds-backend-rust\scripts\p4work\results.csv",
  [int]$DelayMs = 120
)
$ErrorActionPreference = "Continue"
$tok = Get-Content $TokensJson -Raw | ConvertFrom-Json

# 端点清单：Method, Path(模板, {id}/{perm_id}/{cacheId}/{jobId}->1), Required(权限或 jwt/public), Read
$Eps = @(
 @{M="POST";P="/auth/login";Req="public";Read=$false}
 @{M="POST";P="/auth/logout";Req="jwt";Read=$false}
 @{M="POST";P="/auth/refresh";Req="jwt";Read=$false}
 @{M="GET"; P="/auth/profile";Req="jwt";Read=$true}
 @{M="GET"; P="/auth/csrf";Req="jwt";Read=$true}
 @{M="PUT"; P="/auth/change-password";Req="jwt";Read=$false}
 @{M="POST";P="/auth/register";Req="public";Read=$false}

 @{M="GET"; P="/users";Req="admin";Read=$true}
 @{M="POST";P="/users";Req="admin";Read=$false}
 @{M="GET"; P="/users/{id}";Req="admin";Read=$true}
 @{M="PUT"; P="/users/{id}";Req="admin";Read=$false}
 @{M="DELETE";P="/users/{id}";Req="admin";Read=$false}

 @{M="GET"; P="/tenants";Req="tenant:read";Read=$true}
 @{M="POST";P="/tenants";Req="tenant:write";Read=$false}
 @{M="GET"; P="/tenants/{id}";Req="tenant:read";Read=$true}
 @{M="PUT"; P="/tenants/{id}";Req="tenant:write";Read=$false}
 @{M="DELETE";P="/tenants/{id}";Req="tenant:write";Read=$false}

 @{M="GET"; P="/clusters";Req="jwt";Read=$true}
 @{M="POST";P="/clusters";Req="cluster:write";Read=$false}
 @{M="GET"; P="/clusters/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/clusters/{id}";Req="cluster:write";Read=$false}
 @{M="DELETE";P="/clusters/{id}";Req="cluster:write";Read=$false}
 @{M="GET"; P="/clusters/{id}/status";Req="jwt";Read=$true}

 @{M="GET"; P="/resources";Req="jwt";Read=$true}
 @{M="GET"; P="/resources/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/resources/{id}";Req="resource:write";Read=$false}
 @{M="GET"; P="/resources/gpu";Req="jwt";Read=$true}

 @{M="GET"; P="/topology";Req="jwt";Read=$true}
 @{M="POST";P="/topology";Req="topology:write";Read=$false}
 @{M="GET"; P="/topology/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/topology/{id}";Req="topology:write";Read=$false}
 @{M="DELETE";P="/topology/{id}";Req="topology:write";Read=$false}
 @{M="GET"; P="/topology/nodes";Req="jwt";Read=$true}
 @{M="POST";P="/topology/nodes";Req="topology:write";Read=$false}
 @{M="GET"; P="/topology/nodes/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/topology/nodes/{id}";Req="topology:write";Read=$false}
 @{M="DELETE";P="/topology/nodes/{id}";Req="topology:write";Read=$false}
 @{M="POST";P="/topology/score";Req="jwt";Read=$false}

 @{M="GET"; P="/k8s/clusters/{id}/pods";Req="jwt";Read=$true}
 @{M="GET"; P="/k8s/clusters/{id}/nodes";Req="jwt";Read=$true}
 @{M="GET"; P="/k8s/clusters/{id}/health";Req="jwt";Read=$true}

 @{M="GET"; P="/jobs";Req="jwt";Read=$true}
 @{M="POST";P="/jobs";Req="job:write";Read=$false}
 @{M="GET"; P="/jobs/stats";Req="jwt";Read=$true}
 @{M="GET"; P="/jobs/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/jobs/{id}";Req="job:write";Read=$false}
 @{M="DELETE";P="/jobs/{id}";Req="job:write";Read=$false}
 @{M="POST";P="/jobs/{id}/cancel";Req="job:write";Read=$false}
 @{M="POST";P="/jobs/{id}/submit";Req="job:submit";Read=$false}
 @{M="GET"; P="/jobs/{id}/status";Req="jwt";Read=$true}

 @{M="GET"; P="/gpus";Req="jwt";Read=$true}
 @{M="POST";P="/gpus";Req="gpu:write";Read=$false}
 @{M="GET"; P="/gpus/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/gpus/{id}";Req="gpu:write";Read=$false}
 @{M="DELETE";P="/gpus/{id}";Req="gpu:write";Read=$false}
 @{M="GET"; P="/gpus/allocations";Req="jwt";Read=$true}
 @{M="POST";P="/gpus/allocations";Req="job:write";Read=$false}
 @{M="DELETE";P="/gpus/allocations/{id}";Req="job:write";Read=$false}
 @{M="GET"; P="/gpus/utilization";Req="jwt";Read=$true}

 @{M="GET"; P="/gpu/devices";Req="jwt";Read=$true}
 @{M="POST";P="/gpu/devices";Req="gpu:write";Read=$false}
 @{M="GET"; P="/gpu/devices/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/gpu/devices/{id}";Req="gpu:write";Read=$false}
 @{M="DELETE";P="/gpu/devices/{id}";Req="gpu:write";Read=$false}
 @{M="GET"; P="/gpu/allocations";Req="jwt";Read=$true}
 @{M="POST";P="/gpu/allocations";Req="job:write";Read=$false}
 @{M="POST";P="/gpu/allocations/{id}/release";Req="job:write";Read=$false}
 @{M="GET"; P="/gpu/utilization";Req="jwt";Read=$true}

 @{M="GET"; P="/partitions";Req="jwt";Read=$true}
 @{M="POST";P="/partitions";Req="partition:write";Read=$false}
 @{M="GET"; P="/partitions/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/partitions/{id}";Req="partition:write";Read=$false}
 @{M="DELETE";P="/partitions/{id}";Req="partition:write";Read=$false}
 @{M="GET"; P="/partitions/{id}/resources";Req="jwt";Read=$true}
 @{M="PUT"; P="/partitions/{id}/priority";Req="partition:write";Read=$false}
 @{M="PUT"; P="/partitions/{id}/max-runtime";Req="partition:write";Read=$false}
 @{M="GET"; P="/partitions/{id}/permissions";Req="jwt";Read=$true}
 @{M="POST";P="/partitions/{id}/permissions";Req="partition:write";Read=$false}
 @{M="DELETE";P="/partitions/{id}/permissions/{perm_id}";Req="partition:write";Read=$false}

 @{M="GET"; P="/quotas";Req="jwt";Read=$true}
 @{M="POST";P="/quotas";Req="quota:write";Read=$false}
 @{M="GET"; P="/quotas/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/quotas/{id}";Req="quota:write";Read=$false}
 @{M="DELETE";P="/quotas/{id}";Req="quota:write";Read=$false}
 @{M="GET"; P="/quotas/usage";Req="jwt";Read=$true}
 @{M="POST";P="/quotas/check";Req="jwt";Read=$false}
 @{M="POST";P="/quotas/{id}/check";Req="jwt";Read=$false}

 @{M="GET"; P="/schedulers";Req="jwt";Read=$true}
 @{M="POST";P="/schedulers";Req="scheduler:write";Read=$false}
 @{M="GET"; P="/schedulers/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/schedulers/{id}";Req="scheduler:write";Read=$false}
 @{M="DELETE";P="/schedulers/{id}";Req="scheduler:write";Read=$false}
 @{M="GET"; P="/schedulers/{id}/queues";Req="jwt";Read=$true}
 @{M="GET"; P="/schedulers/{id}/nodes";Req="jwt";Read=$true}
 @{M="POST";P="/schedulers/{id}/sync";Req="scheduler:write";Read=$false}
 @{M="POST";P="/schedulers/{id}/test-connection";Req="scheduler:write";Read=$false}
 @{M="GET"; P="/schedulers/{id}/health";Req="jwt";Read=$true}

 @{M="GET"; P="/datasets";Req="jwt";Read=$true}
 @{M="POST";P="/datasets";Req="dataset:write";Read=$false}
 @{M="GET"; P="/datasets/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/datasets/{id}";Req="dataset:write";Read=$false}
 @{M="DELETE";P="/datasets/{id}";Req="dataset:write";Read=$false}
 @{M="GET"; P="/datasets/{id}/caches";Req="jwt";Read=$true}
 @{M="POST";P="/datasets/{id}/caches";Req="dataset:write";Read=$false}
 @{M="GET"; P="/datasets/{id}/fluid-caches";Req="jwt";Read=$true}
 @{M="POST";P="/datasets/{id}/fluid-caches";Req="dataset:write";Read=$false}
 @{M="PUT"; P="/fluid-caches/{cacheId}";Req="dataset:write";Read=$false}
 @{M="DELETE";P="/fluid-caches/{cacheId}";Req="dataset:write";Read=$false}
 @{M="POST";P="/fluid-caches/{cacheId}/enable";Req="dataset:write";Read=$false}
 @{M="POST";P="/fluid-caches/{cacheId}/disable";Req="dataset:write";Read=$false}
 @{M="POST";P="/fluid-caches/{cacheId}/prefetch";Req="dataset:write";Read=$false}

 @{M="GET"; P="/checkpoints";Req="jwt";Read=$true}
 @{M="POST";P="/checkpoints";Req="checkpoint:write";Read=$false}
 @{M="GET"; P="/checkpoints/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/checkpoints/{id}";Req="checkpoint:write";Read=$false}
 @{M="DELETE";P="/checkpoints/{id}";Req="checkpoint:write";Read=$false}
 @{M="GET"; P="/checkpoints/latest/{jobId}";Req="jwt";Read=$true}

 @{M="GET"; P="/acceleration";Req="jwt";Read=$true}
 @{M="POST";P="/acceleration";Req="acceleration:write";Read=$false}
 @{M="GET"; P="/acceleration/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/acceleration/{id}";Req="acceleration:write";Read=$false}
 @{M="DELETE";P="/acceleration/{id}";Req="acceleration:write";Read=$false}
 @{M="POST";P="/acceleration/{id}/start";Req="acceleration:write";Read=$false}
 @{M="POST";P="/acceleration/{id}/stop";Req="acceleration:write";Read=$false}

 @{M="GET"; P="/alerts";Req="jwt";Read=$true}
 @{M="GET"; P="/alerts/stats";Req="jwt";Read=$true}
 @{M="POST";P="/alerts";Req="alert:write";Read=$false}
 @{M="GET"; P="/alerts/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/alerts/{id}";Req="alert:write";Read=$false}
 @{M="DELETE";P="/alerts/{id}";Req="alert:write";Read=$false}
 @{M="POST";P="/alerts/{id}/acknowledge";Req="alert:write";Read=$false}
 @{M="POST";P="/alerts/{id}/resolve";Req="alert:write";Read=$false}

 @{M="GET"; P="/monitoring/metrics";Req="jwt";Read=$true}
 @{M="GET"; P="/monitoring/alerts";Req="jwt";Read=$true}
 @{M="PUT"; P="/monitoring/alerts/{id}/resolve";Req="monitoring:write";Read=$false}
 @{M="GET"; P="/monitoring/dashboard";Req="jwt";Read=$true}
 @{M="GET"; P="/monitoring/alert-rules";Req="jwt";Read=$true}
 @{M="POST";P="/monitoring/alert-rules/evaluate";Req="monitoring:write";Read=$false}

 @{M="GET"; P="/security/policies";Req="jwt";Read=$true}
 @{M="POST";P="/security/policies";Req="security:write";Read=$false}
 @{M="GET"; P="/security/policies/{id}";Req="jwt";Read=$true}
 @{M="PUT"; P="/security/policies/{id}";Req="security:write";Read=$false}
 @{M="DELETE";P="/security/policies/{id}";Req="security:write";Read=$false}
 @{M="POST";P="/security/policies/{id}/enable";Req="security:write";Read=$false}
 @{M="POST";P="/security/policies/{id}/disable";Req="security:write";Read=$false}
)

function Resolve([string]$t){ $p=$t -replace '\{id\}','1' -replace '\{perm_id\}','1' -replace '\{cacheId\}','1' -replace '\{jobId\}','1'; return $p }

function Probe([string]$base,[string]$method,[string]$path,[string]$token){
  $uri = $base + "/api/v1" + (Resolve $path)
  $h = @{ "Accept"="application/json" }
  if ($token) { $h["Authorization"] = "Bearer $token" }
  try {
    $p = @{ Uri=$uri; Method=$method; Headers=$h; TimeoutSec=12; UseBasicParsing=$true; ErrorAction="Stop" }
    if ($method -ne "GET" -and $method -ne "DELETE") { $p["Body"]="{}"; $p["ContentType"]="application/json" }
    $r = Invoke-WebRequest @p
    $body = $r.Content
    $html = ($body -is [string] -and $body.TrimStart().StartsWith("<"))
    return [pscustomobject]@{ Code=[int]$r.StatusCode; Body=$body; Html=$html }
  } catch {
    $code=0; $body=""; $html=$false
    if ($_.Exception.Response) {
      try { $code=[int]$_.Exception.Response.StatusCode
            $sr=New-Object IO.StreamReader($_.Exception.Response.GetResponseStream()); $body=$sr.ReadToEnd()
            $html = ($body.TrimStart().StartsWith("<")) } catch {}
    }
    return [pscustomobject]@{ Code=$code; Body=$body; Html=$html }
  }
}
function Keys($j){ if($null -eq $j){return ""}; return ($j.PSObject.Properties.Name | Sort-Object) -join "," }
function Shape($n,$d=0){
  if($d -gt 4){return "..."}
  if($null -eq $n){return "null"}
  if($n -is [array]){ if($n.Count -eq 0){return "[]"} else {return "[ " + (Shape $n[0] ($d+1)) + " ]"} }
  if($n -is [pscustomobject]){ return "{" + (($n.PSObject.Properties.Name|Sort-Object) -join ",") + "}" }
  if($n -is [bool]){return "bool"}
  if(($n -is [int]) -or ($n -is [long]) -or ($n -is [double]) -or ($n -is [decimal])){return "num"}
  if($n -is [string]){return "str"}
  return $n.GetType().Name
}

$rows = New-Object System.Collections.Generic.List[object]
$n=0
foreach($e in $Eps){
  $n++
  Write-Host ("[{0}/{1}] {2} {3}" -f $n,$Eps.Count,$e.M,$e.P)
  # L1/L2 admin
  $gA = Probe $GoBase $e.M $e.P $tok.g_admin;      Start-Sleep -Milliseconds $DelayMs
  $rA = Probe $RustBase $e.M $e.P $tok.r_admin;    Start-Sleep -Milliseconds $DelayMs
  $gj=$null; try{$gj=$gA.Body|ConvertFrom-Json}catch{}
  $rj=$null; try{$rj=$rA.Body|ConvertFrom-Json}catch{}
  $gShape=""; $rShape=""; $l2="n/a"
  if($e.Read -and $gj -and $rj){
    $gShape = Shape $gj.data; $rShape = Shape $rj.data
    $l2 = ($gShape -eq $rShape)
  }
  # L3 identities
  $ids = @(
    @{k="admin"; gt=$tok.g_admin; rt=$tok.r_admin},
    @{k="manager"; gt="";          rt=$tok.r_manager},
    @{k="user";   gt=$tok.g_testuser; rt=$tok.r_user},
    @{k="none";   gt="";          rt=""}
  )
  $row = [pscustomobject]@{
    Method=$e.M; Path=$e.P; Req=$e.Req
    GoAdmin=$gA.Code; RustAdmin=$rA.Code
    GoAdminHtml=$gA.Html; RustAdminHtml=$rA.Html
    GoEnv=(Keys $gj); RustEnv=(Keys $rj)
    GoDataShape=$gShape; RustDataShape=$rShape; L2=$l2
  }
  foreach($id in $ids){
    $g = Probe $GoBase $e.M $e.P $id.gt; Start-Sleep -Milliseconds $DelayMs
    $rs = Probe $RustBase $e.M $e.P $id.rt; Start-Sleep -Milliseconds $DelayMs
    $row | Add-Member -NotePropertyName ("G_"+$id.k) -NotePropertyValue $g.Code
    $row | Add-Member -NotePropertyName ("R_"+$id.k) -NotePropertyValue $rs.Code
  }
  $rows.Add($row)
}
$rows | Export-Csv -Path $OutCsv -NoTypeInformation -Encoding UTF8
Write-Host ("WROTE " + $OutCsv + " rows=" + $rows.Count)
