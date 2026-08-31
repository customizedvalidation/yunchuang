#!/bin/bash
# Metaclouds Environment Configuration Validator
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

# ==================== 必需配置检查 ====================
check_required_configs() {
    log_step "检查必填配置..."

    local env_file="${1:-.env.production}"
    local failed=0

    # 检查文件是否存在
    if [ ! -f "$env_file" ]; then
        log_error "配置文件不存在: $env_file"
        log_info "提示: 复制 .env.production.example 为 .env.production"
        return 1
    fi

    # 必需的配置项（正则表达式）
    declare -A required_configs=(
        ["SECURE_JWT_SECRET"]="CHANGE_THIS|your-|your_|example|default|test"
        ["SECURE_DEFAULT_ADMIN_PASSWORD"]="CHANGE_THIS|your-|your_|example|default|Admin@123!|ChangeMe|ChangeThis"
        ["SECURE_DATABASE_PASSWORD"]="CHANGE_THIS|your-|your_|example|default|test|ChangeMe|ChangeThis"
    )

    # 检查配置项
    for config in "${!required_configs[@]}"; do
        local pattern="${required_configs[$config]}"
        local value=$(grep "^${config}=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

        if [ -z "$value" ]; then
            log_error "✗ $config 未设置"
            failed=1
        elif echo "$value" | grep -qiE "$pattern"; then
            log_error "✗ $config 使用了示例值: ${value:0:30}..."
            failed=1
        else
            log_info "✓ $config (已设置)"
        fi
    done

    if [ $failed -eq 1 ]; then
        log_error "配置文件验证失败！"
        log_info "请修改 $env_file 中的示例值"
        return 1
    fi

    log_info "所有必填配置已正确设置"
    return 0
}

# ==================== JWT 密钥强度检查 ====================
check_jwt_secret_strength() {
    log_step "检查 JWT 密钥强度..."

    local env_file="${1:-.env.production}"
    local jwt_secret=$(grep "^SECURE_JWT_SECRET=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

    if [ -z "$jwt_secret" ]; then
        log_error "JWT_SECRET 未设置"
        return 1
    fi

    local length=${#jwt_secret}

    if [ $length -lt 32 ]; then
        log_error "JWT_SECRET 太短 ($length 字符)，至少需要 32 字符"
        log_info "建议: 使用 64+ 字符的随机字符串"
        return 1
    elif [ $length -lt 64 ]; then
        log_warn "JWT_SECRET 长度 ($length 字符) 可以接受，建议使用 64+ 字符"
    else
        log_info "✓ JWT_SECRET 长度充足 ($length 字符)"
    fi

    # 检查是否包含足够的字符类型
    local has_lower=$(echo "$jwt_secret" | grep -qo '[a-z]' && echo 1 || echo 0)
    local has_upper=$(echo "$jwt_secret" | grep -qo '[A-Z]' && echo 1 || echo 0)
    local has_digit=$(echo "$jwt_secret" | grep -qo '[0-9]' && echo 1 || echo 0)

    if [ $((has_lower + has_upper + has_digit)) -lt 2 ]; then
        log_warn "JWT_SECRET 应该包含字母和数字"
    fi

    return 0
}

# ==================== 密码强度检查 ====================
check_password_strength() {
    log_step "检查密码强度..."

    local env_file="${1:-.env.production}"
    local failed=0

    # 检查管理员密码
    local admin_password=$(grep "^SECURE_DEFAULT_ADMIN_PASSWORD=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

    if [ ${#admin_password} -lt 8 ]; then
        log_error "管理员密码太短 (${#admin_password} 字符)，至少需要 8 字符"
        failed=1
    elif echo "$admin_password" | grep -qv '[A-Z]' || echo "$admin_password" | grep -qv '[a-z]' || echo "$admin_password" | grep -qv '[0-9]'; then
        log_warn "管理员密码应该包含大小写字母和数字"
    else
        log_info "✓ 管理员密码强度充足"
    fi

    return $failed
}

# ==================== CORS 配置检查 ====================
check_cors_config() {
    log_step "检查 CORS 配置..."

    local env_file="${1:-.env.production}"
    local server_env=$(grep "^SERVER_ENV=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

    if [ "$server_env" = "production" ]; then
        local allowed_origins=$(grep "^ALLOWED_ORIGINS=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")

        if [ -z "$allowed_origins" ]; then
            log_error "生产环境必须设置 ALLOWED_ORIGINS"
            return 1
        elif echo "$allowed_origins" | grep -qi "localhost"; then
            log_error "生产环境不应使用 localhost"
            return 1
        else
            log_info "✓ CORS 配置正确"
            log_info "  允许的源: $allowed_origins"
        fi
    else
        log_info "开发环境 CORS 配置检查跳过"
    fi

    return 0
}

# ==================== 数据库配置检查 ====================
check_database_config() {
    log_step "检查数据库配置..."

    local env_file="${1:-.env.production}"
    local failed=0

    # 检查生产环境不应使用 SQLite
    local server_env=$(grep "^SERVER_ENV=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")
    local use_sqlite=$(grep "^USE_SQLITE=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "false")
    local memory_store=$(grep "^MEMORY_STORE_ENABLED=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "false")

    if [ "$server_env" = "production" ]; then
        if [ "$use_sqlite" = "true" ]; then
            log_error "生产环境不应使用 SQLite"
            failed=1
        fi

        if [ "$memory_store" = "true" ]; then
            log_error "生产环境不应使用内存存储"
            failed=1
        fi

        if [ "$use_sqlite" = "false" ] && [ "$memory_store" = "false" ]; then
            log_info "✓ 数据库配置正确 (PostgreSQL)"
        fi
    else
        log_info "开发环境数据库配置检查跳过"
    fi

    return $failed
}

# ==================== SSL 配置检查 ====================
check_ssl_config() {
    log_step "检查 SSL 配置..."

    local env_file="${1:-.env.production}"
    local server_env=$(grep "^SERVER_ENV=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "")
    local ssl_mode=$(grep "^DATABASE_SSL_MODE=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || echo "disable")

    if [ "$server_env" = "production" ]; then
        if [ "$ssl_mode" = "disable" ]; then
            log_error "生产环境数据库必须启用 SSL"
            return 1
        else
            log_info "✓ 数据库 SSL 配置正确: $ssl_mode"
        fi
    else
        log_info "开发环境 SSL 配置检查跳过"
    fi

    return 0
}

# ==================== 敏感信息检查 ====================
check_sensitive_info() {
    log_step "检查敏感信息..."

    local env_file="${1:-.env.production}"
    local sensitive_patterns=(
        "password.*=.*123456"
        "password.*=.*admin"
        "password.*=.*password"
        "secret.*=.*secret"
        "key.*=.*key"
    )

    local found=0
    for pattern in "${sensitive_patterns[@]}"; do
        if grep -iE "^.*${pattern}" "$env_file" | grep -v "^#" > /dev/null 2>&1; then
            log_warn "发现弱密码模式: $pattern"
            found=1
        fi
    done

    if [ $found -eq 0 ]; then
        log_info "✓ 未发现明显的弱密码模式"
    fi

    return 0
}

# ==================== 打印配置摘要 ====================
print_config_summary() {
    log_step "配置摘要"

    local env_file="${1:-.env.production}"

    echo ""
    echo "========================================"
    echo "  环境: $(grep "^SERVER_ENV=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'")"
    echo "  数据库: $(grep "^DATABASE_HOST=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'"):$(grep "^DATABASE_PORT=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'")"
    _jwt=$(grep "^SECURE_JWT_SECRET=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'" || true)
    echo "  JWT 密钥长度: ${#_jwt} 字符"
    echo "  Redis: $(grep "^REDIS_ENABLED=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'")"
    echo "  Prometheus: $(grep "^PROMETHEUS_ENABLED=" "$env_file" | cut -d'=' -f2- | tr -d '"' | tr -d "'")"
    echo "========================================"
    echo ""
}

# ==================== 使用说明 ====================
usage() {
    echo "Metaclouds 环境配置验证脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  -f, --file FILE    指定配置文件 (默认: .env.production)"
    echo "  -h, --help         显示帮助"
    echo ""
    echo "示例:"
    echo "  $0                          # 检查 .env.production"
    echo "  $0 -f .env.development      # 检查开发环境配置"
}

# ==================== 主函数 ====================
main() {
    local env_file=".env.production"

    # 解析参数
    while [[ $# -gt 0 ]]; do
        case $1 in
            -f|--file)
                env_file="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                log_error "未知选项: $1"
                usage
                exit 1
                ;;
        esac
    done

    echo ""
    log_info "========================================"
    log_info "  Metaclouds 配置验证"
    log_info "========================================"
    echo ""
    log_info "配置文件: $env_file"
    echo ""

    local failed=0

    check_required_configs "$env_file" || ((failed++))
    check_jwt_secret_strength "$env_file" || ((failed++))
    check_password_strength "$env_file" || ((failed++))
    check_cors_config "$env_file" || ((failed++))
    check_database_config "$env_file" || ((failed++))
    check_ssl_config "$env_file" || ((failed++))
    check_sensitive_info "$env_file" || ((failed++))

    print_config_summary "$env_file"

    if [ $failed -eq 0 ]; then
        log_info "✓ 所有检查通过！"
        return 0
    else
        log_error "✗ $failed 项检查失败"
        return 1
    fi
}

main "$@"
