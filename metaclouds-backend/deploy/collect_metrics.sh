#!/bin/bash
# Prometheus Metrics Collection Script for Metaclouds
# Author: Metaclouds Team
# Version: 1.0.0

set -euo pipefail

# ==================== 配置区域 ====================
API_HOST="${API_HOST:-localhost}"
API_PORT="${API_PORT:-8000}"
METRICS_ENDPOINT="${METRICS_ENDPOINT:-http://${API_HOST}:${API_PORT}/metrics}"
PUSHGATEWAY_URL="${PUSHGATEWAY_URL:-http://localhost:9091}"
LOG_FILE="/var/log/metaclouds/metrics.log"

# ==================== 日志函数 ====================
log() {
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[${timestamp}] $*" >> "${LOG_FILE}"
}

# ==================== 指标收集函数 ====================
collect_system_metrics() {
    local timestamp=$(date +%s)

    # CPU 使用率
    local cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | sed 's/%us,//')
    echo "metaclouds_system_cpu_usage ${cpu_usage}" | sed "s/$/ ${timestamp}/"

    # 内存使用率
    local mem_usage=$(free | grep Mem | awk '{printf "%.2f", $3/$2 * 100}')
    echo "metaclouds_system_memory_usage_percent ${mem_usage}" | sed "s/$/ ${timestamp}/"

    # 磁盘使用率
    local disk_usage=$(df / | awk 'NR==2 {print $5}' | sed 's/%//')
    echo "metaclouds_system_disk_usage_percent ${disk_usage}" | sed "s/$/ ${timestamp}/"
}

collect_application_metrics() {
    local timestamp=$(date +%s)

    # 尝试从 API 获取应用指标
    if command -v curl &> /dev/null; then
        # 检查 API 健康状态
        local api_status=$(curl -s -o /dev/null -w '%{http_code}' "${API_HOST}:${API_PORT}/health" 2>/dev/null || echo "0")
        echo "metaclouds_api_health_status ${api_status}" | sed "s/$/ ${timestamp}/"

        # 尝试获取任务数量
        local response=$(curl -s "${API_HOST}:${API_PORT}/api/v1/jobs" 2>/dev/null || echo "")
        if [ -n "${response}" ]; then
            local job_count=$(echo "${response}" | grep -o '"id"' | wc -l)
            echo "metaclouds_jobs_total ${job_count}" | sed "s/$/ ${timestamp}/"
        fi
    fi
}

push_to_pushgateway() {
    local metrics_data="$1"

    if [ -n "${PUSHGATEWAY_URL}" ]; then
        echo "${metrics_data}" | curl -s --data-binary @- "${PUSHGATEWAY_URL}/metrics/job/metaclouds" || log "WARN: Failed to push metrics to PushGateway"
    fi
}

# ==================== 主函数 ====================
main() {
    log "Starting metrics collection..."

    # 确保日志目录存在
    mkdir -p "$(dirname "${LOG_FILE}")"

    # 收集系统指标
    local system_metrics=$(collect_system_metrics)

    # 收集应用指标
    local app_metrics=$(collect_application_metrics)

    # 合并指标
    local all_metrics="${system_metrics}
${app_metrics}"

    # 输出到标准输出（可以被其他工具收集）
    echo "${all_metrics}"

    # 可选：推送到 PushGateway
    if [ -n "${PUSHGATEWAY_URL}" ]; then
        push_to_pushgateway "${all_metrics}"
    fi

    log "Metrics collection completed"
}

main "$@"
