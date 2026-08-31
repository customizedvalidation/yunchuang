#!/bin/bash

# ==============================================
# Metaclouds Production Deployment Script
# ==============================================
# Usage: ./deploy.sh [options]
# Options:
#   -h, --help        Show this help message
#   -e, --env ENV     Environment: production (default), staging, development
#   -b, --build       Only build, don't start
#   -s, --start       Only start, don't build
#   -d, --daemon      Run in background (daemon mode)
#   -k, --kill        Kill existing process using the port before starting
#   -l, --log-level   Log level: DEBUG, INFO (default), WARN, ERROR
# ==============================================

set -e

# 引入端口工具函数库
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT_UTILS_SCRIPT="$SCRIPT_DIR/deploy/utils/port_utils.sh"

if [[ -f "$PORT_UTILS_SCRIPT" ]]; then
    # shellcheck source=deploy/utils/port_utils.sh
    source "$PORT_UTILS_SCRIPT"
else
    echo "[ERROR] Port utility script not found: $PORT_UTILS_SCRIPT"
    exit 1
fi

# Configuration
APP_NAME="metaclouds-backend"
APP_VERSION="1.0.0"
DEFAULT_ENV="production"
BUILD_DIR="."
ENV_FILE=".env"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
ENV=$DEFAULT_ENV
BUILD=true
START=true
DAEMON=false
KILL=false
LOG_LEVEL="INFO"

while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  -h, --help        Show this help message"
            echo "  -e, --env ENV     Environment: production, staging, development"
            echo "  -b, --build       Only build, don't start"
            echo "  -s, --start       Only start, don't build"
            echo "  -d, --daemon      Run in background (daemon mode)"
            echo "  -k, --kill        Kill existing process using the port before starting"
            echo "  -l, --log-level LVL  Log level: DEBUG, INFO, WARN, ERROR (default: INFO)"
            exit 0
            ;;
        -e|--env)
            ENV="$2"
            shift 2
            ;;
        -b|--build)
            START=false
            shift
            ;;
        -s|--start)
            BUILD=false
            shift
            ;;
        -d|--daemon)
            DAEMON=true
            shift
            ;;
        -k|--kill)
            KILL=true
            shift
            ;;
        -l|--log-level)
            LOG_LEVEL="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Set log level for port utils
port_utils_set_log_level "$LOG_LEVEL"

# Functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Main deployment
info "=============================================="
info "Metaclouds Production Deployment Script"
info "Version: $APP_VERSION"
info "Environment: $ENV"
info "=============================================="

# Step 1: Validate environment
info "Validating environment..."
if [[ ! -f ".env.$ENV" ]]; then
    error "Environment file .env.$ENV not found!"
fi

# Step 2: Copy environment configuration
info "Copying environment configuration..."
cp ".env.$ENV" "$ENV_FILE" || error "Failed to copy environment configuration"
success "Environment configuration copied: $ENV_FILE"

# Step 2.1: Read LOG_LEVEL from environment file and apply to port utils
info "Applying log level from environment configuration..."
if [[ -f "$ENV_FILE" ]]; then
    ENV_LOG_LEVEL=$(grep -E "^LOG_LEVEL=" "$ENV_FILE" | cut -d'=' -f2 | tr -d '"' | tr '[:lower:]' '[:upper:]')
    if [[ -n "$ENV_LOG_LEVEL" ]]; then
        info "Found LOG_LEVEL=$ENV_LOG_LEVEL in $ENV_FILE"
        port_utils_set_log_level "$ENV_LOG_LEVEL"
    else
        info "LOG_LEVEL not found in $ENV_FILE, using default: $LOG_LEVEL"
        port_utils_set_log_level "$LOG_LEVEL"
    fi
fi

# Step 3: Build application
if $BUILD; then
    info "Building application..."
    
    # Check if go is available
    if ! command -v go &> /dev/null; then
        error "Go is not installed. Please install Go first."
    fi
    
    # Build
    go build -o "$APP_NAME" . || error "Build failed"
    success "Application built successfully: $APP_NAME"
fi

# Step 4: Start application
if $START; then
    info "Starting application..."
    
    # Check if application exists
    if [[ ! -f "$APP_NAME" ]]; then
        error "Application binary not found: $APP_NAME"
    fi
    
    # Get port from env file
    SERVER_PORT=$(port_utils_get_port_from_env "$ENV_FILE")
    info "Server port configured: $SERVER_PORT"
    
    # 使用工具库确保端口可用
    if port_utils_ensure_port_available "$SERVER_PORT" "$KILL"; then
        # 端口处理成功（可能用户选择了新端口）
        # 重新读取端口配置
        SERVER_PORT=$(port_utils_get_port_from_env "$ENV_FILE")
    else
        # 处理失败或用户取消
        exit $?
    fi
    
    # Make executable
    chmod +x "$APP_NAME"
    
    # Clean up old PID file if exists
    if [[ -f "metaclouds.pid" ]]; then
        rm -f "metaclouds.pid"
    fi
    
    if $DAEMON; then
        # Run in background
        nohup ./"$APP_NAME" > /var/log/metaclouds.log 2>&1 &
        echo $! > metaclouds.pid
        success "Application started in background (PID: $(cat metaclouds.pid))"
        info "Logs: /var/log/metaclouds.log"
        info "Access at: http://localhost:$SERVER_PORT"
    else
        # Run in foreground
        info "Starting application in foreground (Ctrl+C to stop)..."
        info "Access at: http://localhost:$SERVER_PORT"
        ./"$APP_NAME"
    fi
fi

info "=============================================="
info "Deployment completed"
info "=============================================="
