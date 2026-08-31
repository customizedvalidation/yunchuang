#!/bin/bash
# Metaclouds Deployment Verification Script
# Author: Metaclouds Team
# Version: 1.0.0

set -euo pipefail

# ==================== 颜色输出 ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# ==================== 日志函数 ====================
log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $*"; }

# ==================== 脚本目录 ====================
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REQUIRED_COMMANDS=(
    "bash"
    "date"
    "mkdir"
    "rm"
    "cat"
    "cut"
    "head"
    "tail"
    "awk"
    "sed"
    "grep"
    "find"
    "stat"
    "sha256sum"
    "curl"
)

OPTIONAL_COMMANDS=(
    "pg_dump"
    "pg_isready"
    "pg_restore"
    "gpg"
    "aws"
    "redis-cli"
    "sendmail"
)

# ==================== 检查函数 ====================
check_required_commands() {
    log_step "检查必需命令..."
    local missing=0

    for cmd in "${REQUIRED_COMMANDS[@]}"; do
        if command -v "$cmd" &> /dev/null; then
            log_info "✓ $cmd"
        else
            log_error "✗ $cmd (缺失)"
            missing=1
        fi
    done

    if [ $missing -eq 1 ]; then
        log_error "缺少必需命令，请先安装"
        return 1
    fi

    log_info "所有必需命令已安装"
    return 0
}

check_optional_commands() {
    log_step "检查可选命令..."
    local missing=0

    for cmd in "${OPTIONAL_COMMANDS[@]}"; do
        if command -v "$cmd" &> /dev/null; then
            log_info "✓ $cmd (已安装)"
        else
            log_warn "✗ $cmd (未安装，部分功能可能不可用)"
        fi
    done

    return 0
}

check_directory_structure() {
    log_step "检查目录结构..."

    local dirs=(
        "/var/log/metaclouds"
        "/var/backups/metaclouds"
        "/run/metaclouds"
    )

    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            log_info "✓ $dir (存在)"
        else
            log_warn "$dir (不存在，将自动创建)"
            if mkdir -p "$dir" 2>/dev/null; then
                log_info "✓ $dir (已创建)"
            else
                log_error "✗ 无法创建 $dir (权限不足，请使用 sudo)"
                return 1
            fi
        fi

        if [ -w "$dir" ]; then
            log_info "  ✓ $dir (可写)"
        else
            log_warn "  ✗ $dir (不可写)"
        fi
    done

    return 0
}

check_postgresql_connection() {
    log_step "检查 PostgreSQL 连接..."

    if ! command -v pg_isready &> /dev/null; then
        log_warn "pg_isready 未安装，跳过数据库检查"
        return 0
    fi

    local db_host="${DATABASE_HOST:-localhost}"
    local db_port="${DATABASE_PORT:-5432}"
    local db_user="${DATABASE_USER:-metaclouds_user}"

    if PGPASSWORD="${DATABASE_PASSWORD:-}" pg_isready -h "$db_host" -p "$db_port" -U "$db_user" &> /dev/null; then
        log_info "✓ PostgreSQL 连接正常 ($db_host:$db_port)"
    else
        log_warn "✗ PostgreSQL 连接失败 (请检查 DATABASE_HOST, DATABASE_PORT, DATABASE_USER, DATABASE_PASSWORD)"
    fi

    return 0
}

check_redis_connection() {
    log_step "检查 Redis 连接..."

    if ! command -v redis-cli &> /dev/null; then
        log_warn "redis-cli 未安装，跳过 Redis 检查"
        return 0
    fi

    local redis_host="${REDIS_HOST:-localhost}"
    local redis_port="${REDIS_PORT:-6379}"

    if redis-cli -h "$redis_host" -p "$redis_port" ping &> /dev/null; then
        log_info "✓ Redis 连接正常 ($redis_host:$redis_port)"
    else
        log_warn "✗ Redis 连接失败 (请检查 REDIS_HOST, REDIS_PORT, REDIS_PASSWORD)"
    fi

    return 0
}

check_api_health() {
    log_step "检查 API 健康状态..."

    local api_host="${API_HOST:-localhost}"
    local api_port="${API_PORT:-8000}"

    local status=$(curl -s -o /dev/null -w '%{http_code}' "http://${api_host}:${api_port}/health" 2>/dev/null || echo "000")

    if [ "$status" = "200" ]; then
        log_info "✓ API 健康检查通过 (http://${api_host}:${api_port}/health)"
    else
        log_warn "✗ API 健康检查失败 (HTTP $status)"
        log_warn "  请确保 Metaclouds 服务正在运行"
    fi

    return 0
}

check_script_permissions() {
    log_step "检查脚本权限..."

    local scripts=(
        "backup_postgresql.sh"
        "verify_backup.sh"
        "collect_metrics.sh"
        "health_check.sh"
        "check_disk_space.sh"
    )

    for script in "${scripts[@]}"; do
        local script_path="$SCRIPT_DIR/$script"
        if [ -f "$script_path" ]; then
            if [ -x "$script_path" ]; then
                log_info "✓ $script (可执行)"
            else
                log_warn "$script (不可执行，将设置为可执行)"
                chmod +x "$script_path"
                log_info "✓ $script (已设置为可执行)"
            fi
        else
            log_error "✗ $script (文件不存在)"
        fi
    done

    return 0
}

check_environment_variables() {
    log_step "检查环境变量配置..."

    local required_vars=(
        "DATABASE_HOST"
        "DATABASE_PORT"
        "DATABASE_NAME"
        "DATABASE_USER"
        "DATABASE_PASSWORD"
    )

    local missing=0
    for var in "${required_vars[@]}"; do
        if [ -n "${!var}" ]; then
            log_info "✓ $var (已设置)"
        else
            log_warn "✗ $var (未设置)"
            missing=1
        fi
    done

    if [ $missing -eq 1 ]; then
        log_warn "部分环境变量未设置，将使用默认值"
    fi

    return 0
}

check_disk_space() {
    log_step "检查磁盘空间..."

    local usage=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')

    log_info "根目录使用率: ${usage}%"

    if [ "$usage" -ge 90 ]; then
        log_error "✗ 磁盘空间不足 (使用率 ${usage}%)"
        return 1
    elif [ "$usage" -ge 80 ]; then
        log_warn "⚠ 磁盘空间警告 (使用率 ${usage}%)"
    else
        log_info "✓ 磁盘空间充足"
    fi

    return 0
}

check_backup_directory() {
    log_step "检查备份目录..."

    local backup_dir="${BACKUP_DIR:-/var/backups/metaclouds}"

    if [ ! -d "$backup_dir" ]; then
        log_warn "备份目录不存在: $backup_dir"
        if mkdir -p "$backup_dir" 2>/dev/null; then
            log_info "✓ 备份目录已创建: $backup_dir"
        else
            log_error "✗ 无法创建备份目录 (权限不足)"
            return 1
        fi
    fi

    local backup_count=$(find "$backup_dir" -name "*.dump.gz*" -type f 2>/dev/null | wc -l)
    local backup_size=$(du -sh "$backup_dir" 2>/dev/null | cut -f1 || echo "未知")

    log_info "备份数量: $backup_count"
    log_info "备份大小: $backup_size"

    return 0
}

print_summary() {
    log_step "部署验证总结"
    echo ""
    echo "========================================"
    echo "  Metaclouds 部署验证报告"
    echo "========================================"
    echo ""
    echo "时间: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "主机: $(hostname)"
    echo "脚本目录: $SCRIPT_DIR"
    echo ""
    echo "后续步骤:"
    echo "  1. 设置环境变量: export DATABASE_PASSWORD=your_password"
    echo "  2. 测试备份: ./backup_postgresql.sh list"
    echo "  3. 启动服务: ./start.sh"
    echo ""
    echo "========================================"
}

# ==================== 主函数 ====================
main() {
    echo ""
    log_info "========================================"
    log_info "  Metaclouds 部署验证脚本"
    log_info "========================================"
    echo ""

    local failed=0

    check_required_commands || ((failed++))
    check_optional_commands
    check_directory_structure || ((failed++))
    check_disk_space || ((failed++))
    check_script_permissions
    check_environment_variables
    check_postgresql_connection
    check_redis_connection
    check_api_health
    check_backup_directory

    echo ""
    if [ $failed -eq 0 ]; then
        log_info "✓ 所有检查通过！"
        print_summary
        return 0
    else
        log_warn "⚠ 部分检查失败，请修复上述问题"
        return 1
    fi
}

main "$@"
