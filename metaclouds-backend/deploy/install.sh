#!/bin/bash
# Metaclouds Scripts Installation Script
# Author: Metaclouds Team
# Version: 1.0.0

set -euo pipefail

# ==================== 颜色输出 ====================
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${METACLOUDS_INSTALL_DIR:-/opt/metaclouds}"
BIN_DIR="${METACLOUDS_BIN_DIR:-/usr/local/bin}"
CONFIG_DIR="${METACLOUDS_CONFIG_DIR:-/etc/metaclouds}"

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

usage() {
    echo "Metaclouds 安装脚本"
    echo ""
    echo "用法: $0 [选项]"
    echo ""
    echo "选项:"
    echo "  --install-dir DIR     安装目录 (默认: $INSTALL_DIR)"
    echo "  --bin-dir DIR         bin 目录 (默认: $BIN_DIR)"
    echo "  --config-dir DIR      配置目录 (默认: $CONFIG_DIR)"
    echo "  --skip-sudo           跳过 sudo 检查"
    echo "  -h, --help            显示帮助"
    echo ""
    echo "示例:"
    echo "  $0                                    # 默认安装"
    echo "  $0 --install-dir /opt/metaclouds      # 自定义安装目录"
    echo "  sudo $0                              # 使用 sudo 安装到系统目录"
}

parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install-dir)
                INSTALL_DIR="$2"
                shift 2
                ;;
            --bin-dir)
                BIN_DIR="$2"
                shift 2
                ;;
            --config-dir)
                CONFIG_DIR="$2"
                shift 2
                ;;
            --skip-sudo)
                SKIP_SUDO=1
                shift
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
}

check_root() {
    if [ "$(id -u)" -ne 0 ] && [ -z "${SKIP_SUDO:-}" ]; then
        log_warn "建议使用 sudo 运行此脚本以安装到系统目录"
        log_info "或者使用 --skip-sudo 安装到用户目录"
    fi
}

create_directories() {
    log_info "创建目录..."

    local dirs=(
        "$INSTALL_DIR"
        "$INSTALL_DIR/scripts"
        "$INSTALL_DIR/config"
        "$INSTALL_DIR/logs"
        "$INSTALL_DIR/backups"
        "$CONFIG_DIR"
        "/var/log/metaclouds"
        "/var/backups/metaclouds"
        "/run/metaclouds"
    )

    for dir in "${dirs[@]}"; do
        if [ -d "$dir" ]; then
            log_info "  ✓ $dir (已存在)"
        else
            if mkdir -p "$dir" 2>/dev/null; then
                log_info "  ✓ $dir (已创建)"
            else
                log_warn "  ✗ $dir (权限不足，将跳过)"
            fi
        fi
    done
}

install_scripts() {
    log_info "安装脚本..."

    local scripts=(
        "backup_postgresql.sh"
        "verify_backup.sh"
        "collect_metrics.sh"
        "health_check.sh"
        "check_disk_space.sh"
        "verify_deployment.sh"
    )

    for script in "${scripts[@]}"; do
        local source="$SCRIPT_DIR/$script"
        local target="$INSTALL_DIR/scripts/$script"

        if [ -f "$source" ]; then
            if cp "$source" "$target" 2>/dev/null; then
                chmod +x "$target"
                log_info "  ✓ $script"
            else
                log_warn "  ✗ $script (复制失败)"
            fi
        else
            log_warn "  - $script (源文件不存在)"
        fi
    done
}

install_logrotate() {
    log_info "安装 logrotate 配置..."

    if [ -f "$SCRIPT_DIR/metaclouds.logrotate" ]; then
        if [ -w "/etc/logrotate.d" ] || sudo test -w "/etc/logrotate.d" 2>/dev/null; then
            if sudo cp "$SCRIPT_DIR/metaclouds.logrotate" /etc/logrotate.d/metaclouds 2>/dev/null; then
                log_info "  ✓ metaclouds.logrotate"
            else
                log_warn "  ✗ metaclouds.logrotate (需要 sudo)"
            fi
        else
            log_warn "  - /etc/logrotate.d 不可写，跳过"
        fi
    else
        log_warn "  - metaclouds.logrotate 不存在"
    fi
}

install_cron() {
    log_info "安装 cron 配置..."

    if [ -f "$SCRIPT_DIR/metaclouds-cron" ]; then
        if [ -w "/etc/cron.d" ] || sudo test -w "/etc/cron.d" 2>/dev/null; then
            if sudo cp "$SCRIPT_DIR/metaclouds-cron" /etc/cron.d/metaclouds-backup 2>/dev/null; then
                sudo chmod 644 /etc/cron.d/metaclouds-backup
                log_info "  ✓ metaclouds-cron"
            else
                log_warn "  ✗ metaclouds-cron (需要 sudo)"
            fi
        else
            log_warn "  - /etc/cron.d 不可写，跳过"
        fi
    else
        log_warn "  - metaclouds-cron 不存在"
    fi
}

create_config_template() {
    log_info "创建配置模板..."

    local config_file="$CONFIG_DIR/backup.env.example"

    if [ -d "$CONFIG_DIR" ] || mkdir -p "$CONFIG_DIR" 2>/dev/null; then
        cat > "$config_file" << 'EOF'
# Metaclouds Backup Configuration
# Copy this file to backup.env and configure the values

# Database Configuration
DATABASE_HOST=localhost
DATABASE_PORT=5432
DATABASE_NAME=metaclouds
DATABASE_USER=metaclouds_user
DATABASE_PASSWORD=your_secure_password

# Backup Configuration
BACKUP_DIR=/var/backups/metaclouds
RETENTION_DAYS=30

# S3 Configuration (optional)
# S3_BUCKET=your-s3-bucket
# S3_PREFIX=metaclouds/backups

# Encryption (optional)
# BACKUP_ENCRYPTION_KEY=your_encryption_key

# Alert Configuration
ALERT_EMAIL=admin@yourcompany.com
EOF
        chmod 600 "$config_file"
        log_info "  ✓ $config_file"
    else
        log_warn "  - 无法创建配置目录"
    fi
}

print_installation_summary() {
    echo ""
    echo "========================================"
    echo "  安装完成!"
    echo "========================================"
    echo ""
    echo "安装目录: $INSTALL_DIR"
    echo "配置目录: $CONFIG_DIR"
    echo ""
    echo "脚本位置:"
    echo "  $INSTALL_DIR/scripts/"
    echo ""
    echo "后续步骤:"
    echo "  1. 配置环境变量:"
    echo "     cp $CONFIG_DIR/backup.env.example $CONFIG_DIR/backup.env"
    echo "     nano $CONFIG_DIR/backup.env"
    echo ""
    echo "  2. 测试备份脚本:"
    echo "     $INSTALL_DIR/scripts/verify_deployment.sh"
    echo "     $INSTALL_DIR/scripts/backup_postgresql.sh list"
    echo ""
    echo "  3. 查看日志:"
    echo "     tail -f /var/log/metaclouds/backup.log"
    echo ""
    echo "========================================"
}

main() {
    parse_args "$@"
    check_root

    echo ""
    log_info "========================================"
    log_info "  Metaclouds 安装脚本"
    log_info "========================================"
    echo ""

    create_directories
    install_scripts
    install_logrotate
    install_cron
    create_config_template
    print_installation_summary
}

main "$@"
