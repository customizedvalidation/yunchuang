.PHONY: help build up down restart logs status clean test metrics health

DOCKER_DIR := $(shell pwd)
COMPOSE := docker-compose

help:
	@echo "Metaclouds Docker 部署管理"
	@echo ""
	@echo "可用命令:"
	@echo "  make build    - 构建Docker镜像"
	@echo "  make up       - 启动所有服务"
	@echo "  make down     - 停止所有服务"
	@echo "  make restart  - 重启所有服务"
	@echo "  make logs     - 查看日志"
	@echo "  make status   - 查看服务状态"
	@echo "  make clean    - 清理所有资源"
	@echo "  make test     - 运行集成测试"
	@echo "  make metrics  - 查看监控指标"
	@echo "  make health   - 检查服务健康"
	@echo ""

build:
	@echo "构建Docker镜像..."
	cd $(DOCKER_DIR) && $(COMPOSE) build --no-cache

up:
	@echo "启动服务..."
	cd $(DOCKER_DIR) && $(COMPOSE) up -d
	@echo ""
	@echo "等待服务启动..."
	@sleep 15
	@$(MAKE) status

down:
	@echo "停止服务..."
	cd $(DOCKER_DIR) && $(COMPOSE) down

restart:
	@echo "重启服务..."
	cd $(DOCKER_DIR) && $(COMPOSE) restart
	@echo "等待服务就绪..."
	@sleep 5

logs:
	cd $(DOCKER_DIR) && $(COMPOSE) logs -f

logs-app:
	cd $(DOCKER_DIR) && $(COMPOSE) logs -f app-1 app-2 app-3

logs-nginx:
	cd $(DOCKER_DIR) && $(COMPOSE) logs -f nginx

logs-redis:
	cd $(DOCKER_DIR) && $(COMPOSE) logs -f redis

status:
	@echo ""
	@echo "服务状态:"
	@cd $(DOCKER_DIR) && $(COMPOSE) ps
	@echo ""
	@echo "健康检查:"
	@curl -s http://localhost:8000/health || echo "❌ 网关未就绪"

clean:
	@echo "清理Docker资源..."
	cd $(DOCKER_DIR) && $(COMPOSE) down -v
	@echo "清理未使用的镜像..."
	@docker system prune -f
	@echo "✅ 清理完成"

test:
	@echo "运行集成测试..."
	@chmod +x $(DOCKER_DIR)/integration-test.sh
	@$(DOCKER_DIR)/integration-test.sh

metrics:
	@echo "监控指标 (前30行):"
	@curl -s http://localhost:8000/metrics | head -n 30

metrics-full:
	@echo "完整监控指标:"
	@curl -s http://localhost:8000/metrics | less

health:
	@echo "检查服务健康状态..."
	@echo ""
	@echo "网关 (Nginx):"
	@curl -s -o /dev/null -w "HTTP Status: %{http_code}\n" http://localhost:8000/health || echo "❌ 未就绪"
	@echo ""
	@echo "应用实例:"
	@curl -s -o /dev/null -w "App-1: HTTP %{http_code}\n" http://app-1:8000/health || echo "❌ App-1 未就绪"
	@curl -s -o /dev/null -w "App-2: HTTP %{http_code}\n" http://app-2:8000/health || echo "❌ App-2 未就绪"
	@curl -s -o /dev/null -w "App-3: HTTP %{http_code}\n" http://app-3:8000/health || echo "❌ App-3 未就绪"
	@echo ""
	@echo "Redis:"
	@docker exec metaclouds-redis redis-cli ping || echo "❌ Redis 未就绪"

scale:
	@echo "扩展应用实例..."
	@cd $(DOCKER_DIR) && $(COMPOSE) up -d --scale app-1=5 --scale app-2=5 --scale app-3=5

login-grafana:
	@echo "Grafana: http://localhost:3000 (admin/admin)"
	@echo ""

login-prometheus:
	@echo "Prometheus: http://localhost:9090"
	@echo ""

shell-app1:
	docker exec -it metaclouds-app-1 sh

shell-app2:
	docker exec -it metaclouds-app-2 sh

shell-app3:
	docker exec -it metaclouds-app-3 sh

shell-redis:
	docker exec -it metaclouds-redis redis-cli

shell-nginx:
	docker exec -it metaclouds-nginx sh

stats:
	docker stats --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}"

benchmark:
	@echo "运行基准测试..."
	@echo ""
	@echo "100个并发请求:"
	@for i in {1..100}; do curl -s http://localhost:8000/health > /dev/null & done; wait
	@echo "基准测试完成"

stress:
	@echo "压力测试: 1000个并发任务创建"
	@TOKEN=$$(curl -s -X POST http://localhost:8000/api/v1/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username":"admin","password":"admin123"}' | \
		jq -r '.data.token'); \
	for i in {1..1000}; do \
		curl -s -X POST http://localhost:8000/api/v1/jobs \
			-H "Content-Type: application/json" \
			-H "Authorization: Bearer $$TOKEN" \
			-d "{\"name\":\"Stress-Test-$$i\",\"priority\":$$($$i % 4),\"type\":\"training\"}" > /dev/null & \
		if [ $$((i % 50)) -eq 0 ]; then wait; echo "进度: $$i/1000"; fi; \
	done; wait
	@echo "压力测试完成"
