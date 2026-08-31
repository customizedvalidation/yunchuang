<#
.SYNOPSIS
Metaclouds 本地开发环境检查脚本（PowerShell 版本）

.DESCRIPTION
检查所有开发环境依赖项和配置是否正确

.EXAMPLE
.\check_dev_env.ps1

.NOTES
Author: Metaclouds Team
Version: 1.0.0
#>

$ErrorActionPreference = "SilentlyContinue"

# ==================== 颜色输出 ====================
function Write-Info { Write-Host "[INFO] $_" -ForegroundColor Green }
function Write-Warn { Write-Host "[WARN] $_" -ForegroundColor Yellow }
function Write-Error { Write-Host "[ERROR] $_" -ForegroundColor Red }
function Write-Step { Write-Host "[STEP] $_" -ForegroundColor Blue }

# ==================== 检查函数 ====================
function Test-GoVersion {
    Write-Step "检查 Go 版本..."
    
    $go = Get-Command go -ErrorAction SilentlyContinue
    if (-not $go) {
        Write-Error "Go 未安装"
        return $false
    }
    
    $version = go version
    $versionMatch = [regex]::Match($version, 'go(\d+)\.(\d+)')
    $major = [int]$versionMatch.Groups[1].Value
    $minor = [int]$versionMatch.Groups[2].Value
    
    Write-Info "Go 版本: $($versionMatch.Value)"
    
    if ($major -lt 1 -or ($major -eq 1 -and $minor -lt 21)) {
        Write-Error "Go 版本需要 >= 1.21，当前版本: $($versionMatch.Value)"
        return $false
    }
    
    return $true
}

function Test-GitVersion {
    Write-Step "检查 Git 版本..."
    
    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git) {
        Write-Error "Git 未安装"
        return $false
    }
    
    $version = git --version
    $versionMatch = [regex]::Match($version, 'git version (\d+\.\d+\.\d+)')
    Write-Info "Git 版本: $($versionMatch.Groups[1].Value)"
    
    return $true
}

function Test-Docker {
    Write-Step "检查 Docker..."
    
    $docker = Get-Command docker -ErrorAction SilentlyContinue
    if (-not $docker) {
        Write-Warn "Docker 未安装（可选）"
        return $true
    }
    
    $version = docker --version
    $versionMatch = [regex]::Match($version, 'Docker version (\d+\.\d+\.\d+)')
    Write-Info "Docker 版本: $($versionMatch.Groups[1].Value)"
    
    try {
        docker info | Out-Null
        Write-Info "Docker 服务运行正常"
    } catch {
        Write-Warn "Docker 服务未运行"
    }
    
    try {
        $composeVersion = docker compose version
        $composeMatch = [regex]::Match($composeVersion, 'v(\d+\.\d+\.\d+)')
        Write-Info "Docker Compose 版本: $($composeMatch.Groups[1].Value)"
    } catch {
        Write-Warn "Docker Compose 未安装（可选）"
    }
    
    return $true
}

function Test-EnvFile {
    Write-Step "检查环境配置文件..."
    
    $envFiles = @(".env", ".env.development", ".env.production")
    foreach ($file in $envFiles) {
        if (Test-Path $file) {
            Write-Info "$file 存在"
        } else {
            Write-Warn "$file 不存在"
        }
    }
    
    return $true
}

function Test-EnvVariables {
    Write-Step "检查关键环境变量..."
    
    if (-not (Test-Path ".env")) {
        Write-Warn "跳过环境变量检查（.env 文件不存在）"
        return $true
    }
    
    $content = Get-Content ".env" -Raw
    
    # 检查 JWT_SECRET
    $jwtMatch = [regex]::Match($content, '^JWT_SECRET=(.*)$', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    $jwtSecret = $jwtMatch.Groups[1].Value.Trim('"', "'")
    
    if (-not $jwtSecret) {
        Write-Error "JWT_SECRET 未设置"
        return $false
    } elseif ($jwtSecret.Length -lt 32) {
        Write-Error "JWT_SECRET 太短（$($jwtSecret.Length) 字符），需要至少 32 字符"
        return $false
    } else {
        Write-Info "JWT_SECRET 长度充足（$($jwtSecret.Length) 字符）"
    }
    
    # 检查 SERVER_PORT
    $portMatch = [regex]::Match($content, '^SERVER_PORT=(.*)$', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    $serverPort = $portMatch.Groups[1].Value.Trim('"', "'")
    
    if (-not $serverPort) {
        Write-Warn "SERVER_PORT 未设置，将使用默认值 8000"
    } elseif (-not ($serverPort -match '^\d+$') -or [int]$serverPort -lt 1 -or [int]$serverPort -gt 65535) {
        Write-Error "SERVER_PORT 无效: $serverPort"
        return $false
    } else {
        Write-Info "SERVER_PORT: $serverPort"
    }
    
    # 检查 SERVER_ENV
    $envMatch = [regex]::Match($content, '^SERVER_ENV=(.*)$', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    $serverEnv = $envMatch.Groups[1].Value.Trim('"', "'")
    
    if (-not $serverEnv) {
        Write-Warn "SERVER_ENV 未设置，将使用默认值 development"
    } else {
        Write-Info "SERVER_ENV: $serverEnv"
    }
    
    # 检查示例值
    if ($content -match '(CHANGE|your-|example)' -and $content -notmatch '^#') {
        Write-Warn ".env 文件中存在示例值，建议修改"
    }
    
    return $true
}

function Test-Dependencies {
    Write-Step "检查项目依赖..."
    
    if (-not (Test-Path "go.mod")) {
        Write-Error "go.mod 不存在"
        return $false
    }
    
    Write-Info "go.mod 存在"
    
    if (-not (Test-Path "go.sum")) {
        Write-Warn "go.sum 不存在，运行 go mod tidy"
    } else {
        Write-Info "go.sum 存在"
    }
    
    try {
        go mod download | Out-Null
        Write-Info "依赖下载成功"
    } catch {
        Write-Warn "依赖下载失败（网络问题？）"
    }
    
    return $true
}

function Test-DirectoryStructure {
    Write-Step "检查目录结构..."
    
    $requiredDirs = @("api", "controllers", "services", "models", "middlewares", "config", "pkg", "deploy")
    $missing = $false
    
    foreach ($dir in $requiredDirs) {
        if (Test-Path $dir -PathType Container) {
            Write-Info "$dir/"
        } else {
            Write-Error "$dir/ 不存在"
            $missing = $true
        }
    }
    
    # 检查运行时目录
    $runtimeDirs = @("logs", "backups")
    foreach ($dir in $runtimeDirs) {
        if (Test-Path $dir -PathType Container) {
            Write-Info "$dir/ 存在"
        } else {
            Write-Warn "$dir/ 不存在，将自动创建"
            New-Item -ItemType Directory -Path $dir | Out-Null
            Write-Info "$dir/ 已创建"
        }
    }
    
    return -not $missing
}

function Test-ApiEndpoints {
    Write-Step "检查 API 端点..."
    
    try {
        $response = Invoke-RestMethod http://localhost:8000/health -ErrorAction Stop
        if ($response.status -eq "healthy") {
            Write-Info "/health - 健康检查通过"
        } else {
            Write-Error "/health - 健康检查失败"
            return $false
        }
    } catch {
        Write-Warn "后端服务未运行，请先启动服务"
        return $true
    }
    
    try {
        $adminPassword = $null
        if (Test-Path ".env.development") {
            $line = Get-Content ".env.development" | Where-Object { $_ -match '^DEFAULT_ADMIN_PASSWORD=' } | Select-Object -First 1
            if ($line) { $adminPassword = ($line -replace '^DEFAULT_ADMIN_PASSWORD=') -replace '"','' -replace "'",'' }
        }
        if (-not $adminPassword) {
            Write-Warn "未找到 DEFAULT_ADMIN_PASSWORD，跳过登录检查（请在 .env.development 中设置已轮换的口令）"
            return $true
        }
        $body = @{username="admin"; password=$adminPassword} | ConvertTo-Json
        $response = Invoke-RestMethod -Method Post -Uri http://localhost:8000/api/v1/auth/login `
            -Body $body -ContentType "application/json" -ErrorAction Stop
        if ($response.success) {
            Write-Info "/api/v1/auth/login - 登录成功"
        } else {
            Write-Warn "/api/v1/auth/login - 登录失败"
        }
    } catch {
        Write-Warn "/api/v1/auth/login - 登录失败"
    }
    
    return $true
}

function Test-Tests {
    Write-Step "运行测试..."
    
    try {
        $output = go test ./... 2>&1
        if ($output -match "FAIL") {
            Write-Error "测试失败"
            return $false
        } else {
            Write-Info "测试通过"
        }
    } catch {
        Write-Warn "测试运行失败"
        return $false
    }
    
    return $true
}

function Write-Summary {
    Write-Host ""
    Write-Host "========================================"
    Write-Host "  Metaclouds 开发环境检查报告"
    Write-Host "========================================"
    Write-Host ""
    Write-Host "时间: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
    Write-Host "主机: $($env:COMPUTERNAME)"
    Write-Host "目录: $(Get-Location)"
    Write-Host ""
    Write-Host "快速启动命令:"
    Write-Host "  1. 直接运行: go run main.go"
    Write-Host "  2. Docker启动: docker compose up -d"
    Write-Host "  3. 查看日志: docker compose logs -f backend"
    Write-Host ""
    Write-Host "访问地址:"
    Write-Host "  - API: http://localhost:8000"
    Write-Host "  - 健康检查: http://localhost:8000/health"
    Write-Host "  - Swagger: http://localhost:8000/swagger/index.html"
    Write-Host ""
    Write-Host "默认凭证:"
    Write-Host "  用户名: admin"
    Write-Host "  密码: 见 .env.development 的 DEFAULT_ADMIN_PASSWORD（已轮换，禁止提交明文）"
    Write-Host ""
    Write-Host "========================================"
}

# ==================== 主函数 ====================
function Main {
    Write-Host ""
    Write-Info "========================================"
    Write-Info "  Metaclouds 开发环境检查"
    Write-Info "========================================"
    Write-Host ""
    
    $failed = 0
    
    # 依赖检查
    if (-not (Test-GoVersion)) { $failed++ }
    if (-not (Test-GitVersion)) { $failed++ }
    Test-Docker | Out-Null
    
    Write-Host ""
    
    # 配置检查
    Test-EnvFile | Out-Null
    if (-not (Test-EnvVariables)) { $failed++ }
    
    Write-Host ""
    
    # 代码检查
    if (-not (Test-Dependencies)) { $failed++ }
    if (-not (Test-DirectoryStructure)) { $failed++ }
    
    Write-Host ""
    
    # API检查
    Test-ApiEndpoints | Out-Null
    
    Write-Host ""
    
    # 测试检查
    if ($failed -eq 0) {
        if (-not (Test-Tests)) { $failed++ }
    }
    
    Write-Host ""
    
    if ($failed -eq 0) {
        Write-Info "✅ 所有检查通过！"
        Write-Summary
        return 0
    } else {
        Write-Error "❌ $failed 项检查失败"
        Write-Summary
        return 1
    }
}

Main
