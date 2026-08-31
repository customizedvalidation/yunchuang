#!/bin/bash

set -e

echo "╔════════════════════════════════════════════════════════════════╗"
echo "║     Metaclouds 多实例Docker部署快速启动脚本                  ║"
echo "╚════════════════════════════════════════════════════════════════╝"
echo ""

DOCKER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DOCKER_DIR"

check_docker() {
    if ! command -v docker &> /dev/null; then
        echo "❌ Docker 未安装"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo "❌ Docker Compose 未安装"
        exit 1
    fi
    
    echo "✅ Docker 和 Docker Compose 已安装"
}

build_images() {
    echo ""
    echo "【1/4】构建 Docker 镜像..."
    echo "─────────────────────────────────────"
    
    docker-compose build --no-cache
    echo "✅ 镜像构建完成"
}

start_services() {
    echo ""
    echo "【2/4】启动服务..."
    echo "─────────────────────────────────────"
    
    docker-compose up -d redis
    echo "⏳ 等待 Redis 启动..."
    sleep 5
    
    docker-compose up -d app-1 app-2 app-3
    echo "⏳ 等待应用实例启动..."
    sleep 10
    
    docker-compose up -d nginx
    echo "⏳ 等待 Nginx 启动..."
    sleep 5
    
    docker-compose up -d prometheus grafana
    echo "✅ 所有服务已启动"
}

wait_for_healthy() {
    echo ""
    echo "【3/4】等待服务健康检查..."
    echo "─────────────────────────────────────"
    
    max_attempts=30
    attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        if curl -sf http://localhost:8000/health > /dev/null 2>&1; then
            echo "✅ Nginx 网关已就绪"
            break
        fi
        
        attempt=$((attempt + 1))
        echo "⏳ 等待服务就绪... ($attempt/$max_attempts)"
        sleep 2
    done
    
    if [ $attempt -eq $max_attempts ]; then
        echo "❌ 服务启动超时"
        docker-compose logs
        exit 1
    fi
}

show_status() {
    echo ""
    echo "【4/4】服务状态"
    echo "─────────────────────────────────────"
    
    echo ""
    echo "🌐 服务访问地址："
    echo "   • API网关:      http://localhost:8000"
    echo "   • Swagger UI:   http://localhost:8000/swagger/index.html"
    echo "   • Prometheus:   http://localhost:9090"
    echo "   • Grafana:      http://localhost:3000 (admin/admin)"
    echo ""
    
    echo "📊 Docker 容器状态："
    docker-compose ps
    
    echo ""
    echo "🔍 健康检查："
    curl -s http://localhost:8000/health
    echo ""
    
    echo ""
    echo "📈 监控指标："
    curl -s http://localhost:8000/metrics | head -n 20
    echo "   ... (更多指标)"
}

show_help() {
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "                    可用命令"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    echo "  ./start.sh          启动所有服务"
    echo "  ./start.sh stop     停止所有服务"
    echo "  ./start.sh restart  重启所有服务"
    echo "  ./start.sh logs     查看日志"
    echo "  ./start.sh status   查看服务状态"
    echo "  ./start.sh clean   清理所有资源"
    echo "  ./start.sh test    运行集成测试"
    echo ""
}

case "${1:-start}" in
    start)
        check_docker
        build_images
        start_services
        wait_for_healthy
        show_status
        ;;
    stop)
        echo "停止所有服务..."
        docker-compose down
        echo "✅ 服务已停止"
        ;;
    restart)
        echo "重启所有服务..."
        docker-compose restart
        wait_for_healthy
        show_status
        ;;
    logs)
        docker-compose logs -f
        ;;
    status)
        show_status
        ;;
    clean)
        echo "清理所有Docker资源..."
        docker-compose down -v
        docker system prune -f
        echo "✅ 清理完成"
        ;;
    test)
        chmod +x integration-test.sh
        ./integration-test.sh
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "未知命令: $1"
        show_help
        exit 1
        ;;
esac
