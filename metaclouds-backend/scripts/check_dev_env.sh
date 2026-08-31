#!/bin/bash
# Metaclouds 本地开发环境检查脚本
# Author: Metaclouds Team
# Version: 1.0.0

set -euo pipefail

# ==================== 颜色输出 ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $*"; }

# ==================== 检查函数 ====================
check_go_version() {
    log_step "检查 Go 版本..."

    if ! command -v go &> /dev/null; then
        log_error "✗ Go 未安装"
        return 1
    fi

    local version=$(go version | awk '{print $3}' | sed 's/go//')
    local major=$(echo "$version" | cut -d'.' -f1)
    local minor=$(echo "$version" | cut -d'.' -f2)

    log_info "✓ Go 版本: $version"

    if [ "$major" -lt 1 ] || ([ "$major" -eq 1 ] && [ "$minor" -lt 21 ]); then
        log_error "✗ Go 版本需要 >= 1.21，当前版本: $version"
        return 1
    fi

    return 0
}

check_git_version() {
    log_step "检查 Git 版本..."

    if ! command -v git &> /dev/null; then
        log_error "✗ Git 未安装"
        return 1
    fi

    local version=$(git --version | awk '{print $3}')
    log_info "✓ Git 版本: $version"

    return 0
}

check_docker() {
    log_step "检查 Docker..."

    if ! command -v docker &> /dev/null; then
        log_warn "✗ Docker 未安装（可选）"
        return 0
    fi

    local version=$(docker --version | awk '{print $3}' | sed 's/,//')
    log_info "✓ Docker 版本: $version"

    if ! docker info &> /dev/null; then
        log_warn "✗ Docker 服务未运行（需要 sudo）"
    else
        log_info "✓ Docker 服务运行正常"
    fi

    # 检查 Docker Compose
    if docker compose version &> /dev/null; then
        local compose_version=$(docker compose version | awk '{print $4}' | sed 's/,//')
        log_info "✓ Docker Compose 版本: $compose_version"
    else
        log_warn "✗ Docker Compose 未安装（可选）"
    fi

    return 0
}

check_env_file() {
    log_step "检查环境配置文件..."

    local env_files=(
        ".env"
        ".env.development"
        ".env.production"
    )

    for env_file in "${env_files[@]}"; do
        if [ -f "$env_file" ]; then
            log_info "✓ $env_file 存在"
        else
            log_warn "✗ $env_file 不存在"
        fi
    done

    return 0
}

check_env_variables() {
    log_step "检查关键环境变量..."

    if [ ! -f ".env" ]; then
        log_warn "跳过环境变量检查（.env 文件不存在）"
        return 0
    fi

    local jwt_secret=$(grep "^JWT_SECRET=" .env | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")
    local server_port=$(grep "^SERVER_PORT=" .env | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")
    local server_env=$(grep "^SERVER_ENV=" .env | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

    # 检查 JWT_SECRET
    if [ -z "$jwt_secret" ]; then
        log_error "✗ JWT_SECRET 未设置"
        return 1
    elif [ ${#jwt_secret} -lt 32 ]; then
        log_error "✗ JWT_SECRET 太短（${#jwt_secret} 字符），需要至少 32 字符"
        return 1
    else
        log_info "✓ JWT_SECRET 长度充足（${#jwt_secret} 字符）"
    fi

    # 检查 SERVER_PORT
    if [ -z "$server_port" ]; then
        log_warn "✗ SERVER_PORT 未设置，将使用默认值 8000"
    elif ! [[ "$server_port" =~ ^[0-9]+$ ]] || [ "$server_port" -lt 1 ] || [ "$server_port" -gt 65535 ]; then
        log_error "✗ SERVER_PORT 无效: $server_port"
        return 1
    else
        log_info "✓ SERVER_PORT: $server_port"
    fi

    # 检查 SERVER_ENV
    if [ -z "$server_env" ]; then
        log_warn "✗ SERVER_ENV 未设置，将使用默认值 development"
    else
        log_info "✓ SERVER_ENV: $server_env"
    fi

    # 检查示例值
    if grep -i "CHANGE\|your-\|example" .env | grep -v "^#" > /dev/null; then
        log_warn "⚠ .env 文件中存在示例值，建议修改"
    fi

    return 0
}

check_dependencies() {
    log_step "检查项目依赖..."

    if [ ! -f "go.mod" ]; then
        log_error "✗ go.mod 不存在"
        return 1
    fi

    log_info "✓ go.mod 存在"

    if [ ! -f "go.sum" ]; then
        log_warn "✗ go.sum 不存在，运行 go mod tidy"
    else
        log_info "✓ go.sum 存在"
    fi

    # 尝试下载依赖
    if go mod download 2>/dev/null; then
        log_info "✓ 依赖下载成功"
    else
        log_warn "⚠ 依赖下载失败（网络问题？）"
    fi

    return 0
}

check_directory_structure() {
    log_step "检查目录结构..."

    local required_dirs=(
        "api"
        "controllers"
        "services"
        "models"
        "middlewares"
        "config"
        "pkg"
        "deploy"
    )

    local missing=0
    for dir in "${required_dirs[@]}"; do
        if [ -d "$dir" ]; then
            log_info "✓ $dir/"
        else
            log_error "✗ $dir/ 不存在"
            missing=1
        fi
    done

    # 检查运行时目录
    local runtime_dirs=(
        "logs"
        "backups"
    )

    for dir in "${runtime_dirs[@]}"; do
        if [ -d "$dir" ]; then
            log_info "✓ $dir/ 存在"
        else
            log_warn "✗ $dir/ 不存在，将自动创建"
            mkdir -p "$dir" 2>/dev/null && log_info "✓ $dir/ 已创建"
        fi
    done

    return $missing
}

check_api_endpoints() {
    log_step "检查 API 端点..."

    # 检查服务是否运行
    if ! curl -s http://localhost:8000/health &> /dev/null; then
        log_warn "✗ 后端服务未运行，请先启动服务"
        return 0
    fi

    # 健康检查
    local health_status=$(curl -s http://localhost:8000/health | grep -o '"status":"[^"]*"' | cut -d'"' -f4)
    if [ "$health_status" = "healthy" ]; then
        log_info "✓ /health - 健康检查通过"
    else
        log_error "✗ /health - 健康检查失败"
        return 1
    fi

    # 读取开发用管理员口令：绝不硬编码，从 .env.development / .env 中读取
    # 已轮换后的口令，避免脚本成为泄露凭据的又一载体。
    local admin_password=""
    if [ -f ".env.development" ]; then
        admin_password=$(grep "^DEFAULT_ADMIN_PASSWORD=" .env.development | cut -d'=' -f2- | tr -d '"' | tr -d "'" || true)
    fi
    if [ -z "$admin_password" ] && [ -f ".env" ]; then
        admin_password=$(grep "^DEFAULT_ADMIN_PASSWORD=" .env | cut -d'=' -f2- | tr -d '"' | tr -d "'" || true)
    fi
    if [ -z "$admin_password" ]; then
        log_warn "✗ 未找到 DEFAULT_ADMIN_PASSWORD，跳过登录检查（请在 .env.development 中设置已轮换的口令）"
        return 0
    fi

    # 检查登录
    local login_response=$(curl -s -X POST http://localhost:8000/api/v1/auth/login \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"admin\",\"password\":\"$admin_password\"}")
    
    if echo "$login_response" | grep -q '"success":true'; then
        log_info "✓ /api/v1/auth/login - 登录成功"
    else
        log_warn "✗ /api/v1/auth/login - 登录失败"
    fi

    return 0
}

check_tests() {
    log_step "检查测试..."

    # 运行简单的测试检查
    if go test ./... 2>&1 | grep -q "FAIL"; then
        log_error "✗ 测试失败"
        return 1
    else
        log_info "✓ 测试通过"
    fi

    return 0
}

print_summary() {
    echo ""
    echo "========================================"
    echo "  Metaclouds 开发环境检查报告"
    echo "========================================"
    echo ""
    echo "时间: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "主机: $(hostname)"
    echo "目录: $(pwd)"
    echo ""
    echo "检查结果:"
    echo "  依赖检查: $dep_check"
    echo "  配置检查: $config_check"
    echo "  代码检查: $code_check"
    echo "  API检查: $api_check"
    echo ""
    echo "快速启动命令:"
    echo "  1. 直接运行: go run main.go"
    echo "  2. Docker启动: docker compose up -d"
    echo "  3. 查看日志: docker compose logs -f backend"
    echo ""
    echo "访问地址:"
    echo "  - API: http://localhost:8000"
    echo "  - 健康检查: http://localhost:8000/health"
    echo "  - Swagger: http://localhost:8000/swagger/index.html"
    echo ""
    echo "默认凭证:"
    echo "  用户名: admin"
    echo "  密码: 见 .env.development 的 DEFAULT_ADMIN_PASSWORD（已轮换，禁止提交明文）"
    echo ""
    echo "========================================"
}

# ==================== 主函数 ====================
main() {
    echo ""
    log_info "========================================"
    log_info "  Metaclouds 开发环境检查"
    log_info "========================================"
    echo ""

    local failed=0
    local dep_check="PASS"
    local config_check="PASS"
    local code_check="PASS"
    local api_check="PASS"

    # 依赖检查
    check_go_version || ((failed++)) || dep_check="FAIL"
    check_git_version || ((failed++)) || dep_check="FAIL"
    check_docker

    echo ""

    # 配置检查
    check_env_file || ((failed++)) || config_check="FAIL"
    check_env_variables || ((failed++)) || config_check="FAIL"

    echo ""

    # 代码检查
    check_dependencies || ((failed++)) || code_check="FAIL"
    check_directory_structure || ((failed++)) || code_check="FAIL"

    echo ""

    # API检查（可选）
    check_api_endpoints || api_check="SKIP"

    echo ""

    # 测试检查（可选）
    if [ $failed -eq 0 ]; then
        log_step "运行测试（可选）..."
        check_tests || ((failed++)) || code_check="FAIL"
    fi

    echo ""

    if [ $failed -eq 0 ]; then
        log_info "✅ 所有检查通过！"
        print_summary
        return 0
    else
        log_error "❌ $failed 项检查失败"
        print_summary
        return 1
    fi
}

main "$@"
