#!/bin/bash
# ==============================================
# 端口工具函数库 (Linux/macOS)
# 用于处理端口占用、进程管理等通用功能
# ==============================================

# 颜色定义
PORT_UTILS_RED='\033[0;31m'
PORT_UTILS_GREEN='\033[0;32m'
PORT_UTILS_YELLOW='\033[1;33m'
PORT_UTILS_BLUE='\033[0;34m'
PORT_UTILS_PURPLE='\033[0;35m'
PORT_UTILS_NC='\033[0m'

# ==============================================
# 日志级别配置模块
# ==============================================
# 支持的日志级别 (优先级从高到低)
# ERROR > WARN > INFO > DEBUG
# ==============================================

# 默认日志级别
PORT_UTILS_LOG_LEVEL="${PORT_UTILS_LOG_LEVEL:-INFO}"

# 日志级别常量
PORT_UTILS_LOG_LEVEL_DEBUG="DEBUG"
PORT_UTILS_LOG_LEVEL_INFO="INFO"
PORT_UTILS_LOG_LEVEL_WARN="WARN"
PORT_UTILS_LOG_LEVEL_ERROR="ERROR"

# 日志级别优先级映射
declare -A PORT_UTILS_LOG_LEVEL_PRIORITY=(
    ["DEBUG"]=4
    ["INFO"]=3
    ["WARN"]=2
    ["ERROR"]=1
)

# ==============================================
# 日志配置管理函数
# ==============================================

# 设置日志级别
# 参数: log_level (DEBUG, INFO, WARN, ERROR)
port_utils_set_log_level() {
    local new_level="${1:-INFO}"
    new_level=$(echo "$new_level" | tr '[:lower:]' '[:upper:]')
    
    if port_utils_is_valid_log_level "$new_level"; then
        PORT_UTILS_LOG_LEVEL="$new_level"
        port_utils_log_info "Log level set to: $PORT_UTILS_LOG_LEVEL"
        return 0
    else
        port_utils_log_error "Invalid log level: $new_level. Valid levels: DEBUG, INFO, WARN, ERROR"
        return 1
    fi
}

# 获取当前日志级别
port_utils_get_log_level() {
    echo "$PORT_UTILS_LOG_LEVEL"
}

# 验证日志级别是否有效
port_utils_is_valid_log_level() {
    local level="$1"
    [[ -n "${PORT_UTILS_LOG_LEVEL_PRIORITY[$level]}" ]]
}

# 检查是否应该输出指定级别的日志
# 参数: log_level
# 返回: 0=应该输出, 1=不应该输出
port_utils_should_log() {
    local level="$1"
    local current_level="$PORT_UTILS_LOG_LEVEL"
    
    local level_priority="${PORT_UTILS_LOG_LEVEL_PRIORITY[$level]}"
    local current_priority="${PORT_UTILS_LOG_LEVEL_PRIORITY[$current_level]}"
    
    if [[ -z "$level_priority" || -z "$current_priority" ]]; then
        return 1
    fi
    
    [[ "$level_priority" -le "$current_priority" ]]
}

# ==============================================
# 日志输出函数
# ==============================================

port_utils_log_debug() {
    if port_utils_should_log "DEBUG"; then
        echo -e "${PORT_UTILS_PURPLE}[DEBUG]${PORT_UTILS_NC} $1"
    fi
}

port_utils_log_info() {
    if port_utils_should_log "INFO"; then
        echo -e "${PORT_UTILS_BLUE}[INFO]${PORT_UTILS_NC} $1"
    fi
}

port_utils_log_warn() {
    if port_utils_should_log "WARN"; then
        echo -e "${PORT_UTILS_YELLOW}[WARNING]${PORT_UTILS_NC} $1"
    fi
}

port_utils_log_error() {
    if port_utils_should_log "ERROR"; then
        echo -e "${PORT_UTILS_RED}[ERROR]${PORT_UTILS_NC} $1"
    fi
}

port_utils_log_success() {
    if port_utils_should_log "INFO"; then
        echo -e "${PORT_UTILS_GREEN}[SUCCESS]${PORT_UTILS_NC} $1"
    fi
}

# ==============================================
# 日志级别快捷切换函数
# ==============================================

# 设置为调试级别
port_utils_enable_debug() {
    port_utils_set_log_level "DEBUG"
}

# 设置为信息级别（默认）
port_utils_enable_info() {
    port_utils_set_log_level "INFO"
}

# 设置为警告级别
port_utils_enable_warn() {
    port_utils_set_log_level "WARN"
}

# 设置为错误级别
port_utils_enable_error() {
    port_utils_set_log_level "ERROR"
}

# ==============================================
# 端口工具函数
# ==============================================

# 从 .env 文件读取端口配置
port_utils_get_port_from_env() {
    local env_file="${1:-.env}"
    port_utils_log_debug "port_utils_get_port_from_env called with env_file=$env_file"
    
    if [ -f "$env_file" ]; then
        port_utils_log_debug "Environment file exists: $env_file"
        local port=$(grep -E "^SERVER_PORT=" "$env_file" | cut -d'=' -f2 | tr -d '"')
        if [ -n "$port" ]; then
            port_utils_log_debug "Found SERVER_PORT=$port in $env_file"
            echo "$port"
        else
            port_utils_log_warn "SERVER_PORT not found in $env_file, using default 8000"
            echo "8000"
        fi
    else
        port_utils_log_warn "Environment file $env_file not found, using default 8000"
        echo "8000"
    fi
}

# 检测端口是否被占用
# 返回 0: 被占用, 1: 空闲
port_utils_is_port_in_use() {
    local port="$1"
    port_utils_log_debug "port_utils_is_port_in_use called with port=$port"
    
    if [ -z "$port" ]; then
        port_utils_log_error "Port parameter is empty"
        return 1
    fi
    
    local method_used=""
    local result=1
    
    if command -v lsof >/dev/null 2>&1; then
        method_used="lsof"
        port_utils_log_debug "Using lsof to check port $port"
        lsof -i ":$port" >/dev/null 2>&1
        result=$?
    elif command -v netstat >/dev/null 2>&1; then
        method_used="netstat"
        port_utils_log_debug "Using netstat to check port $port"
        netstat -tuln >/dev/null 2>&1 | grep -q ":$port "
        result=$?
    elif command -v ss >/dev/null 2>&1; then
        method_used="ss"
        port_utils_log_debug "Using ss to check port $port"
        ss -tuln >/dev/null 2>&1 | grep -q ":$port "
        result=$?
    else
        port_utils_log_warn "No port checking tool available (lsof/netstat/ss)"
        return 1
    fi
    
    if [ $result -eq 0 ]; then
        port_utils_log_debug "Port $port is in use (method: $method_used)"
    else
        port_utils_log_debug "Port $port is available (method: $method_used)"
    fi
    
    return $result
}

# 获取占用端口的 PID
port_utils_get_pid_by_port() {
    local port="$1"
    port_utils_log_debug "port_utils_get_pid_by_port called with port=$port"
    
    if [ -z "$port" ]; then
        port_utils_log_error "Port parameter is empty"
        return 1
    fi
    
    if command -v lsof >/dev/null 2>&1; then
        local pid=$(lsof -ti ":$port" 2>/dev/null)
        if [ -n "$pid" ]; then
            port_utils_log_debug "Found PID $pid using port $port"
            echo "$pid"
        else
            port_utils_log_debug "No process found using port $port"
        fi
    else
        port_utils_log_warn "lsof not available, cannot get PID"
    fi
}

# 获取占用端口的进程信息
port_utils_get_process_info() {
    local port="$1"
    port_utils_log_debug "port_utils_get_process_info called with port=$port"
    
    local pid=$(port_utils_get_pid_by_port "$port")
    if [ -n "$pid" ]; then
        port_utils_log_debug "Getting process info for PID $pid"
        local info=$(ps -p "$pid" -o pid,user,command 2>/dev/null)
        if [ -n "$info" ]; then
            echo "$info"
            port_utils_log_debug "Process info retrieved successfully"
        else
            port_utils_log_warn "Failed to get process info for PID $pid"
        fi
    else
        port_utils_log_debug "No PID found for port $port"
    fi
}

# 强制终止指定 PID 的进程
port_utils_kill_pid() {
    local pid="$1"
    port_utils_log_debug "port_utils_kill_pid called with pid=$pid"
    
    if [ -z "$pid" ]; then
        port_utils_log_error "PID parameter is empty"
        return 1
    fi
    
    if ! [[ "$pid" =~ ^[0-9]+$ ]]; then
        port_utils_log_error "Invalid PID format: $pid"
        return 1
    fi
    
    port_utils_log_debug "Attempting to kill process with PID $pid"
    kill -9 "$pid" >/dev/null 2>&1
    local result=$?
    
    if [ $result -eq 0 ]; then
        port_utils_log_debug "Successfully killed process $pid"
    else
        port_utils_log_warn "Failed to kill process $pid (exit code: $result)"
    fi
    
    return $result
}

# 终止占用端口的进程
# 参数: port [silent]
# 返回 0: 成功, 1: 失败
port_utils_kill_process_by_port() {
    local port="$1"
    local silent="${2:-false}"
    port_utils_log_debug "port_utils_kill_process_by_port called with port=$port, silent=$silent"
    
    local pid=$(port_utils_get_pid_by_port "$port")
    if [ -z "$pid" ]; then
        if [ "$silent" = "false" ]; then
            port_utils_log_warn "No process found using port $port"
        fi
        return 1
    fi
    
    if [ "$silent" = "false" ]; then
        port_utils_log_warn "Killing process $pid using port $port"
    fi
    
    if port_utils_kill_pid "$pid"; then
        port_utils_log_debug "Waiting 2 seconds for port to be released..."
        sleep 2
        
        if port_utils_is_port_in_use "$port"; then
            if [ "$silent" = "false" ]; then
                port_utils_log_error "Failed to release port $port. Please kill the process manually."
            fi
            return 1
        fi
        
        if [ "$silent" = "false" ]; then
            port_utils_log_success "Port $port released successfully"
        fi
        return 0
    else
        if [ "$silent" = "false" ]; then
            port_utils_log_error "Failed to kill process $pid"
        fi
        return 1
    fi
}

# 交互式处理端口占用
# 参数: port [kill_on_prompt]
port_utils_handle_port_conflict_interactive() {
    local port="$1"
    local kill_on_prompt="${2:-false}"
    port_utils_log_debug "port_utils_handle_port_conflict_interactive called with port=$port, kill_on_prompt=$kill_on_prompt"
    
    if ! port_utils_is_port_in_use "$port"; then
        port_utils_log_debug "Port $port is available, no conflict to handle"
        return 0
    fi
    
    if [ "$silent" = "false" ]; then
        echo ""
        port_utils_log_warn "Port $port is already in use!"
        
        # 显示占用端口的进程信息
        port_utils_log_info "Process using port $port:"
        port_utils_get_process_info "$port"
        echo ""
    fi
    
    if [ "$kill_on_prompt" = "true" ]; then
        port_utils_log_debug "Auto-kill mode enabled, calling kill_process_by_port"
        port_utils_kill_process_by_port "$port"
        return $?
    fi
    
    if [ "$silent" = "false" ]; then
        echo ""
        echo "Options:"
        echo "1) Kill the existing process and start anyway"
        echo "2) Change SERVER_PORT in .env file"
        echo "3) Cancel deployment"
        echo ""
        read -p "Please select an option [1-3]: " choice
        
        case $choice in
            1)
                port_utils_log_debug "User selected option 1: Kill process"
                port_utils_kill_process_by_port "$port"
                return $?
                ;;
            2)
                port_utils_log_debug "User selected option 2: Change port"
                local new_port=""
                read -p "Enter new port number: " new_port
                if [ -z "$new_port" ] || ! [[ "$new_port" =~ ^[0-9]+$ ]]; then
                    port_utils_log_error "Invalid port number: $new_port"
                    return 1
                fi
                port_utils_update_env_port "$new_port"
                export SERVER_PORT="$new_port"
                port_utils_log_success "Updated .env file with port $new_port"
                return 0
                ;;
            3)
                port_utils_log_info "User selected option 3: Cancel deployment"
                return 2
                ;;
            *)
                port_utils_log_error "Invalid option selected: $choice"
                return 1
                ;;
        esac
    fi
}

# 更新 .env 文件中的端口配置
port_utils_update_env_port() {
    local new_port="$1"
    local env_file="${2:-.env}"
    port_utils_log_debug "port_utils_update_env_port called with new_port=$new_port, env_file=$env_file"
    
    if [ -z "$new_port" ] || ! [[ "$new_port" =~ ^[0-9]+$ ]]; then
        port_utils_log_error "Invalid port number: $new_port"
        return 1
    fi
    
    if [ ! -f "$env_file" ]; then
        port_utils_log_error "Environment file $env_file not found"
        return 1
    fi
    
    port_utils_log_debug "Updating SERVER_PORT to $new_port in $env_file"
    
    if [ "$(uname)" = "Darwin" ]; then
        sed -i '' "s/^SERVER_PORT=.*/SERVER_PORT=$new_port/" "$env_file"
    else
        sed -i "s/^SERVER_PORT=.*/SERVER_PORT=$new_port/" "$env_file"
    fi
    
    local result=$?
    if [ $result -eq 0 ]; then
        port_utils_log_debug "Successfully updated $env_file"
    else
        port_utils_log_error "Failed to update $env_file (exit code: $result)"
    fi
    
    return $result
}

# 完整的端口验证和处理流程
# 参数: port [auto_kill]
port_utils_ensure_port_available() {
    local port="$1"
    local auto_kill="${2:-false}"
    port_utils_log_info "port_utils_ensure_port_available called with port=$port, auto_kill=$auto_kill"
    
    if [ -z "$port" ]; then
        port_utils_log_error "Port parameter is empty"
        return 1
    fi
    
    port_utils_log_info "Checking port $port..."
    
    if ! port_utils_is_port_in_use "$port"; then
        port_utils_log_success "Port $port is available"
        return 0
    fi
    
    if [ "$auto_kill" = "true" ]; then
        port_utils_log_info "Auto-kill mode enabled, attempting to release port $port"
        port_utils_kill_process_by_port "$port"
        return $?
    fi
    
    port_utils_log_debug "Calling interactive conflict handler for port $port"
    port_utils_handle_port_conflict_interactive "$port"
    return $?
}

# 查找可用的端口
# 参数: start_port [end_port]
port_utils_find_available_port() {
    local start_port="${1:-8000}"
    local end_port="${2:-9000}"
    port_utils_log_debug "port_utils_find_available_port called with start_port=$start_port, end_port=$end_port"
    
    if [ -z "$start_port" ] || ! [[ "$start_port" =~ ^[0-9]+$ ]]; then
        port_utils_log_error "Invalid start port: $start_port"
        return 1
    fi
    
    if [ -z "$end_port" ] || ! [[ "$end_port" =~ ^[0-9]+$ ]]; then
        port_utils_log_error "Invalid end port: $end_port"
        return 1
    fi
    
    if [ "$start_port" -gt "$end_port" ]; then
        port_utils_log_error "Start port $start_port is greater than end port $end_port"
        return 1
    fi
    
    port_utils_log_debug "Searching for available port in range $start_port-$end_port"
    
    for (( port=start_port; port<=end_port; port++ )); do
        if ! port_utils_is_port_in_use "$port"; then
            port_utils_log_debug "Found available port: $port"
            echo "$port"
            return 0
        fi
        port_utils_log_debug "Port $port is in use, checking next..."
    done
    
    port_utils_log_error "No available port found in range $start_port-$end_port"
    return 1
}

# ==============================================
# 导出函数供其他脚本使用
# ==============================================
export -f port_utils_set_log_level
export -f port_utils_get_log_level
export -f port_utils_is_valid_log_level
export -f port_utils_should_log
export -f port_utils_enable_debug
export -f port_utils_enable_info
export -f port_utils_enable_warn
export -f port_utils_enable_error
export -f port_utils_log_debug
export -f port_utils_log_info
export -f port_utils_log_warn
export -f port_utils_log_error
export -f port_utils_log_success
export -f port_utils_get_port_from_env
export -f port_utils_is_port_in_use
export -f port_utils_get_pid_by_port
export -f port_utils_get_process_info
export -f port_utils_kill_pid
export -f port_utils_kill_process_by_port
export -f port_utils_handle_port_conflict_interactive
export -f port_utils_update_env_port
export -f port_utils_ensure_port_available
export -f port_utils_find_available_port

# ==============================================
# 初始化日志级别
# ==============================================
port_utils_log_debug "Port utils initialized with log level: $PORT_UTILS_LOG_LEVEL"
