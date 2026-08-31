#!/bin/bash
# Health Check Script for Metaclouds
# Author: Metaclouds Team
# Version: 1.1.0

set -euo pipefail

# ==================== 兼容性别名 ====================
get_memory_usage_percent() {
    if command -v free &> /dev/null; then
        # Linux
        if free -m &> /dev/null; then
            local available=$(free -m | awk 'NR==2 {print $7}')
            local total=$(free -m | awk 'NR==2 {print $2}')
            echo $((100 - available * 100 / total))
        else
            # macOS 或其他
            echo "0"
        fi
    else
        echo "0"
    fi
}

# ==================== 配置区域 ====================
API_HOST="${API_HOST:-localhost}"
API_PORT="${API_PORT:-8000}"
ALERT_EMAIL="${ALERT_EMAIL:-admin@yourcompany.com}"
LOG_FILE="/var/log/metaclouds/health_check.log"

# ==================== 日志函数 ====================
log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] $*" >> "${LOG_FILE}"
}

# ==================== 检查函数 ====================
check_api() {
    local status=$(curl -s -o /dev/null -w '%{http_code}' http://${API_HOST}:${API_PORT}/health 2>/dev/null || echo "000")
    
    if [ "$status" = "200" ]; then
        log "API Health: OK (HTTP ${status})"
        return 0
    else
        log "API Health: FAILED (HTTP ${status})"
        return 1
    fi
}

check_database() {
    local db_host="${DATABASE_HOST:-localhost}"
    local db_port="${DATABASE_PORT:-5432}"
    
    if command -v pg_isready &> /dev/null; then
        if PGPASSWORD="${DATABASE_PASSWORD}" pg_isready -h "${db_host}" -p "${db_port}" -U "${DATABASE_USER:-metaclouds}" -d "${DATABASE_NAME:-metaclouds}"; then
            log "Database: OK"
            return 0
        else
            log "Database: FAILED"
            return 1
        fi
    else
        log "Database: pg_isready not found, skipping check"
        return 0
    fi
}

check_redis() {
    local redis_host="${REDIS_HOST:-localhost}"
    local redis_port="${REDIS_PORT:-6379}"
    
    if command -v redis-cli &> /dev/null; then
        if redis-cli -h "${redis_host}" -p "${redis_port}" -a "${REDIS_PASSWORD:-}" ping 2>/dev/null | grep -q "PONG"; then
            log "Redis: OK"
            return 0
        else
            log "Redis: FAILED"
            return 1
        fi
    else
        log "Redis: redis-cli not found, skipping check"
        return 0
    fi
}

check_disk_space() {
    local usage=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')
    
    if [ "$usage" -gt 90 ]; then
        log "Disk Space: CRITICAL (${usage}% used)"
        return 1
    elif [ "$usage" -gt 80 ]; then
        log "Disk Space: WARNING (${usage}% used)"
        return 0
    else
        log "Disk Space: OK (${usage}% used)"
        return 0
    fi
}

check_memory() {
    local available=$(free -m | awk 'NR==2 {print $7}')
    local total=$(free -m | awk 'NR==2 {print $2}')
    local usage=$((100 - available * 100 / total))
    
    if [ "$usage" -gt 90 ]; then
        log "Memory: CRITICAL (${usage}% used)"
        return 1
    elif [ "$usage" -gt 80 ]; then
        log "Memory: WARNING (${usage}% used)"
        return 0
    else
        log "Memory: OK (${usage}% used)"
        return 0
    fi
}

send_alert() {
    local message="$1"
    local severity="$2"
    
    log "ALERT [${severity}]: ${message}"
    
    # Send email alert (optional)
    if [ -n "${ALERT_EMAIL}" ] && command -v sendmail &> /dev/null; then
        echo -e "Subject: [${severity}] Metaclouds Alert\n\n${message}" | sendmail "${ALERT_EMAIL}"
    fi
    
    # Send Slack/Teams webhook (optional)
    if [ -n "${SLACK_WEBHOOK_URL}" ]; then
        curl -s -X POST "${SLACK_WEBHOOK_URL}" \
            -H 'Content-Type: application/json' \
            -d "{\"text\":\"[${severity}] Metaclouds: ${message}\"}" > /dev/null
    fi
}

# ==================== 主函数 ====================
main() {
    log "========== Health Check Started =========="
    
    local failed=0
    
    # 执行各项检查
    check_api || ((failed++))
    check_database || ((failed++))
    check_redis || ((failed++))
    check_disk_space || ((failed++))
    check_memory || ((failed++))
    
    # 根据检查结果决定是否发送告警
    if [ $failed -gt 0 ]; then
        send_alert "${failed} health check(s) failed" "CRITICAL"
        log "Health Check Completed: FAILED (${failed} issues)"
        exit 1
    else
        log "Health Check Completed: ALL OK"
        exit 0
    fi
}

main "$@"
