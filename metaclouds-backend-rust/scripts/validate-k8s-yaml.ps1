# ==============================================================================
# P3-06 K8s YAML Structure Validator (PowerShell)
# ==============================================================================
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts/validate-k8s-yaml.ps1
#
# This script does static structural validation. CI/target env must also run:
#   kubectl apply --dry-run=client -k k8s/
# ==============================================================================

param(
    [string]$K8sDir = (Join-Path $PSScriptRoot "..\k8s")
)

$ErrorActionPreference = "Stop"

$script:PassCount = 0
$script:FailCount = 0
$script:WarnCount = 0
$script:Results = @()

function Add-Result {
    param([string]$File, [string]$Status, [string]$Check, [string]$Detail)
    $script:Results += [PSCustomObject]@{
        File   = $File
        Status = $Status
        Check  = $Check
        Detail = $Detail
    }
    if ($Status -eq "PASS") { $script:PassCount++ }
    elseif ($Status -eq "FAIL") { $script:FailCount++ }
    elseif ($Status -eq "WARN") { $script:WarnCount++ }
}

function Test-Pattern {
    param([string]$Content, [string]$Pattern)
    return ($Content -match $Pattern)
}

Write-Host ""
Write-Host "======================================================================"
Write-Host " P3-06 K8s YAML Structure Validator"
Write-Host "======================================================================"
Write-Host " K8s directory: $K8sDir"
Write-Host ""

if (-not (Test-Path $K8sDir)) {
    Write-Host "FAIL: K8s directory not found: $K8sDir"
    exit 1
}

$yamlFiles = Get-ChildItem -Path $K8sDir -Filter "*.yaml" | Sort-Object Name
Write-Host " Found $($yamlFiles.Count) YAML files"
Write-Host ""

# -- 1. File count --
if ($yamlFiles.Count -ge 13) {
    Add-Result "(global)" "PASS" "FileCount" "Found $($yamlFiles.Count) files (>= 13 required)"
} else {
    Add-Result "(global)" "FAIL" "FileCount" "Found $($yamlFiles.Count) files, need >= 13"
}

# -- 2. Per-file basic checks --
foreach ($file in $yamlFiles) {
    $content = Get-Content -Path $file.FullName -Raw -Encoding UTF8
    $name = $file.Name

    if ($name -eq "13-kustomization.yaml") {
        if (Test-Pattern $content "kind:\s*Kustomization") {
            Add-Result $name "PASS" "Kind" "kind: Kustomization present"
        } else {
            Add-Result $name "FAIL" "Kind" "kind: Kustomization missing"
        }
        continue
    }

    if (Test-Pattern $content "apiVersion:\s*\S+") {
        Add-Result $name "PASS" "ApiVersion" "apiVersion present"
    } else {
        Add-Result $name "FAIL" "ApiVersion" "apiVersion missing"
    }

    if (Test-Pattern $content "kind:\s*\S+") {
        Add-Result $name "PASS" "Kind" "kind present"
    } else {
        Add-Result $name "FAIL" "Kind" "kind missing"
    }

    if (Test-Pattern $content "metadata:") {
        if (Test-Pattern $content "name:\s*\S+") {
            Add-Result $name "PASS" "MetadataName" "metadata.name present"
        } else {
            Add-Result $name "FAIL" "MetadataName" "metadata.name missing"
        }
    } else {
        Add-Result $name "FAIL" "MetadataName" "metadata section missing"
    }
}

# -- 3. Deployment checks --
$deployFile = Join-Path $K8sDir "05-deployment.yaml"
if (Test-Path $deployFile) {
    $deploy = Get-Content $deployFile -Raw -Encoding UTF8
    $dn = "05-deployment.yaml"

    if (Test-Pattern $deploy "startupProbe:") { Add-Result $dn "PASS" "StartupProbe" "present" } else { Add-Result $dn "FAIL" "StartupProbe" "missing" }
    if (Test-Pattern $deploy "livenessProbe:") { Add-Result $dn "PASS" "LivenessProbe" "present" } else { Add-Result $dn "FAIL" "LivenessProbe" "missing" }
    if (Test-Pattern $deploy "readinessProbe:") { Add-Result $dn "PASS" "ReadinessProbe" "present" } else { Add-Result $dn "FAIL" "ReadinessProbe" "missing" }

    if (Test-Pattern $deploy "runAsNonRoot:\s*true") { Add-Result $dn "PASS" "RunAsNonRoot" "runAsNonRoot: true" } else { Add-Result $dn "FAIL" "RunAsNonRoot" "missing" }
    if (Test-Pattern $deploy "runAsUser:\s*10001") { Add-Result $dn "PASS" "RunAsUser" "runAsUser: 10001" } else { Add-Result $dn "FAIL" "RunAsUser" "missing" }
    if (Test-Pattern $deploy "allowPrivilegeEscalation:\s*false") { Add-Result $dn "PASS" "NoPrivEsc" "allowPrivilegeEscalation: false" } else { Add-Result $dn "FAIL" "NoPrivEsc" "missing" }
    if (Test-Pattern $deploy "readOnlyRootFilesystem:\s*true") { Add-Result $dn "PASS" "RoRootFs" "readOnlyRootFilesystem: true" } else { Add-Result $dn "FAIL" "RoRootFs" "missing" }

    if ((Test-Pattern $deploy "resources:") -and (Test-Pattern $deploy "requests:") -and (Test-Pattern $deploy "limits:")) {
        Add-Result $dn "PASS" "Resources" "requests/limits present"
    } else {
        Add-Result $dn "FAIL" "Resources" "requests/limits missing"
    }

    if (Test-Pattern $deploy "topologySpreadConstraints:") { Add-Result $dn "PASS" "Topology" "present" } else { Add-Result $dn "WARN" "Topology" "not found" }
    if ((Test-Pattern $deploy "maxSurge:") -and (Test-Pattern $deploy "maxUnavailable:")) { Add-Result $dn "PASS" "RollingUpdate" "present" } else { Add-Result $dn "WARN" "RollingUpdate" "not found" }
    if (Test-Pattern $deploy "path:\s*/metrics") { Add-Result $dn "PASS" "ProbePath" "probes use /metrics" } else { Add-Result $dn "WARN" "ProbePath" "/metrics not found" }
}

# -- 4. Service port --
$svcFile = Join-Path $K8sDir "06-service.yaml"
if (Test-Path $svcFile) {
    $svc = Get-Content $svcFile -Raw -Encoding UTF8
    $sn = "06-service.yaml"
    if (Test-Pattern $svc "port:\s*8000") {
        Add-Result $sn "PASS" "ServicePort" "port 8000"
    } else {
        Add-Result $sn "FAIL" "ServicePort" "port 8000 missing"
    }
}

# -- 5. HPA --
$hpaFile = Join-Path $K8sDir "07-hpa.yaml"
if (Test-Path $hpaFile) {
    $hpa = Get-Content $hpaFile -Raw -Encoding UTF8
    $hn = "07-hpa.yaml"
    if (Test-Pattern $hpa "minReplicas:") { Add-Result $hn "PASS" "HpaMin" "present" } else { Add-Result $hn "FAIL" "HpaMin" "missing" }
    if (Test-Pattern $hpa "maxReplicas:") { Add-Result $hn "PASS" "HpaMax" "present" } else { Add-Result $hn "FAIL" "HpaMax" "missing" }
    if (Test-Pattern $hpa "metrics:") { Add-Result $hn "PASS" "HpaMetrics" "present" } else { Add-Result $hn "FAIL" "HpaMetrics" "missing" }
}

# -- 6. PDB --
$pdbFile = Join-Path $K8sDir "08-pdb.yaml"
if (Test-Path $pdbFile) {
    $pdb = Get-Content $pdbFile -Raw -Encoding UTF8
    $pn = "08-pdb.yaml"
    if ((Test-Pattern $pdb "minAvailable:") -or (Test-Pattern $pdb "maxUnavailable:")) {
        Add-Result $pn "PASS" "PdbRule" "minAvailable or maxUnavailable present"
    } else {
        Add-Result $pn "FAIL" "PdbRule" "minAvailable/maxUnavailable missing"
    }
}

# -- 7. NetworkPolicy --
$npFile = Join-Path $K8sDir "09-networkpolicy.yaml"
if (Test-Path $npFile) {
    $np = Get-Content $npFile -Raw -Encoding UTF8
    $nn = "09-networkpolicy.yaml"
    if (Test-Pattern $np "podSelector:") { Add-Result $nn "PASS" "NpPodSel" "present" } else { Add-Result $nn "FAIL" "NpPodSel" "missing" }
    if (Test-Pattern $np "policyTypes:") { Add-Result $nn "PASS" "NpPolTypes" "present" } else { Add-Result $nn "FAIL" "NpPolTypes" "missing" }
}

# -- 8. Ingress --
$ingFile = Join-Path $K8sDir "10-ingress.yaml"
if (Test-Path $ingFile) {
    $ing = Get-Content $ingFile -Raw -Encoding UTF8
    $in = "10-ingress.yaml"
    if (Test-Pattern $ing "rules:") { Add-Result $in "PASS" "IngRules" "present" } else { Add-Result $in "FAIL" "IngRules" "missing" }
    if (Test-Pattern $ing "service:") { Add-Result $in "PASS" "IngBackend" "present" } else { Add-Result $in "FAIL" "IngBackend" "missing" }
}

# -- 9. ServiceMonitor --
$smFile = Join-Path $K8sDir "11-servicemonitor.yaml"
if (Test-Path $smFile) {
    $sm = Get-Content $smFile -Raw -Encoding UTF8
    $smn = "11-servicemonitor.yaml"
    if (Test-Pattern $sm "path:\s*/metrics") {
        Add-Result $smn "PASS" "SmMetrics" "path: /metrics"
    } else {
        Add-Result $smn "FAIL" "SmMetrics" "/metrics missing"
    }
}

# -- 10. PrometheusRule alert count --
$prFile = Join-Path $K8sDir "12-prometheusrule.yaml"
if (Test-Path $prFile) {
    $pr = Get-Content $prFile -Raw -Encoding UTF8
    $prn = "12-prometheusrule.yaml"
    $alertCount = ([regex]::Matches($pr, "(?m)-\s*alert:")).Count
    if ($alertCount -ge 16) {
        Add-Result $prn "PASS" "AlertCount" "$alertCount alerts (>= 16)"
    } else {
        Add-Result $prn "FAIL" "AlertCount" "only $alertCount alerts, need >= 16"
    }
}

# -- Report --
Write-Host ""
Write-Host "======================================================================"
Write-Host " Validation Report"
Write-Host "======================================================================"
Write-Host ""

$script:Results | Format-Table -AutoSize -Property File, Status, Check, Detail

Write-Host ""
Write-Host "----------------------------------------------------------------------"
Write-Host " Summary: PASS=$($script:PassCount)  WARN=$($script:WarnCount)  FAIL=$($script:FailCount)"
Write-Host "----------------------------------------------------------------------"
Write-Host ""

if ($script:FailCount -gt 0) {
    Write-Host " RESULT: FAILURES DETECTED"
    exit 1
} else {
    Write-Host " RESULT: ALL CHECKS PASSED"
    Write-Host " NOTE: Structural check only. Run in CI: kubectl apply --dry-run=client -k k8s/"
    exit 0
}
