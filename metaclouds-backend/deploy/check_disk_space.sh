#!/bin/bash
# Disk Space Check Script for Metaclouds
# Author: Metaclouds Team
# Version: 1.1.0

set -euo pipefail

# ==================== 兼容性别名 ====================
get_file_size() {
    local file="$1"
    if stat -c%s "$file" &> /dev/null; then
        stat -c%s "$file"
    else
        stat -f%z "$file" 2>/dev/null || echo "0"
    fi
}

get_dir_size() {
    local dir="$1"
    if du -sb "$dir" &> /dev/null; then
        du -sb "$dir" | cut -f1
    else
        echo "0"
    fi
}

# ==================== 配置区域 ====================
THRESHOLD_WARNING=80
THRESHOLD_CRITICAL=90
LOG_FILE="/var/log/metaclouds/disk_space.log"
ALERT_EMAIL="${ALERT_EMAIL:-admin@yourcompany.com}"

# ==================== 日志函数 ====================
log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] $*" >> "${LOG_FILE}"
}

# ==================== 检查函数 ====================
check_disk() {
    local path="$1"
    local usage=$(df -h "${path}" | awk 'NR==2 {print $5}' | sed 's/%//')
    local available=$(df -h "${path}" | awk 'NR==2 {print $4}')
    local total=$(df -h "${path}" | awk 'NR==2 {print $2}')
    
    log "Path: ${path}, Usage: ${usage}%, Available: ${available}, Total: ${total}"
    
    if [ "$usage" -ge "${THRESHOLD_CRITICAL}" ]; then
        log "ALERT: CRITICAL - ${path} usage is ${usage}%"
        send_alert "CRITICAL: ${path} disk usage is ${usage}% (Available: ${available})"
        return 2
    elif [ "$usage" -ge "${THRESHOLD_WARNING}" ]; then
        log "WARNING: ${path} usage is ${usage}%"
        send_alert "WARNING: ${path} disk usage is ${usage}% (Available: ${available})"
        return 1
    fi
    
    return 0
}

check_backup_dir() {
    local backup_dir="${BACKUP_DIR:-/var/backups/metaclouds}"
    
    if [ -d "${backup_dir}" ]; then
        local size=$(du -sh "${backup_dir}" 2>/dev/null | cut -f1)
        log "Backup directory size: ${size}"
        
        # 检查备份目录大小是否超过限制
        local max_size_gb="${MAX_BACKUP_SIZE_GB:-100}"
        local current_size=$(get_dir_size "${backup_dir}")
        local max_size_bytes=$((max_size_gb * 1024 * 1024 * 1024))
        
        if [ "$current_size" -gt "$max_size_bytes" ]; then
            log "WARNING: Backup directory size (${size}) exceeds limit (${max_size_gb}GB)"
            send_alert "WARNING: Backup directory size (${size}) exceeds limit (${max_size_gb}GB)"
        fi
    else
        log "WARNING: Backup directory ${backup_dir} does not exist"
    fi
}

send_alert() {
    local message="$1"
    
    log "ALERT: ${message}"
    
    # Send email
    if [ -n "${ALERT_EMAIL}" ] && command -v sendmail &> /dev/null; then
        echo -e "Subject: [Metaclouds] Disk Space Alert\n\n${message}" | sendmail "${ALERT_EMAIL}"
    fi
}

# ==================== 主函数 ====================
main() {
    log "========== Disk Space Check Started =========="
    
    # 检查关键路径
    check_disk "/"
    check_disk "/var"
    check_disk "/var/log"
    check_disk "/var/backups"
    check_disk "/tmp"
    
    # 检查备份目录
    check_backup_dir
    
    # 清理临时文件
    log "Cleaning up temporary files..."
    find /tmp -name "metaclouds-*.tmp" -type f -mtime +1 -delete 2>/dev/null || true
    find /var/tmp -name "metaclouds-*" -type f -mtime +7 -delete 2>/dev/null || true
    
    log "========== Disk Space Check Completed =========="
}

main "$@"
