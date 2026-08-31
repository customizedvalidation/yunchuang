# Metaclouds Backend

Metaclouds 后端服务 - 提供 GPU 资源分配、任务调度和监控管理的云原生平台。

## 功能特性

- **GPU 资源分配** - 支持多集群 GPU 资源的动态分配和释放
- **任务调度** - 支持作业提交、取消和状态管理
- **监控告警** - 集成 Prometheus 指标和 Grafana 可视化
- **高可用性** - 支持分布式部署和故障转移

## 技术栈

- **Go 1.22** - 后端服务语言
- **PostgreSQL** - 生产环境数据库
- **Redis** - 缓存和消息队列
- **Prometheus** - 指标监控
- **Grafana** - 可视化面板
- **Jaeger** - 分布式追踪

## 快速开始

### 环境要求

- Go 1.22+
- Docker & Docker Compose (可选)
- PostgreSQL 15+ (生产环境)

### 开发环境运行

```bash
# 克隆项目
git clone <repository-url>
cd metaclouds-backend

# 安装依赖
go mod download

# 设置环境变量
cp .env.example .env

# 运行服务
go run main.go
```

### 使用 Docker Compose

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f metaclouds-backend
```

## 配置说明

### 环境配置文件

项目支持多环境配置，通过 `.env.{env}` 文件管理：

| 文件 | 用途 | 日志级别 |
|------|------|----------|
| `.env.development` | 开发环境 | DEBUG |
| `.env.staging` | 预发布环境 | INFO |
| `.env.production` | 生产环境 | WARN |

### 日志级别配置

#### 支持的日志级别

| 级别 | 说明 | 适用场景 |
|------|------|----------|
| `DEBUG` | 最详细的调试信息 | 开发调试 |
| `INFO` | 一般业务信息 | 测试环境 |
| `WARN` | 警告信息 | 生产环境 |
| `ERROR` | 仅错误信息 | 生产环境（最小日志） |

#### 配置方式

**方式1：环境变量**
```bash
export LOG_LEVEL=warn
```

**方式2：命令行参数**
```bash
./deploy.sh -e production -l WARN
```

**方式3：配置文件**
```bash
# .env.production
LOG_LEVEL=warn
LOG_FORMAT=json
LOG_OUTPUT=file
```

### 生产环境配置模板

```bash
# ==============================================
# Metaclouds Production Configuration
# ==============================================

# Server Configuration
SERVER_PORT=8000
SERVER_ENV=production
SERVER_HOST=0.0.0.0

# Database Configuration
USE_SQLITE=false
DATABASE_HOST=prod-db.metaclouds.com
DATABASE_PORT=5432
DATABASE_USER=metaclouds_prod
DATABASE_PASSWORD=<your-secure-password>
DATABASE_NAME=metaclouds_prod
DATABASE_SSL_MODE=require

# Redis Configuration
REDIS_ENABLED=true
REDIS_HOST=prod-redis.metaclouds.com
REDIS_PORT=6379
REDIS_PASSWORD=<your-secure-password>

# JWT Configuration
JWT_SECRET=<at-least-32-characters>
JWT_EXPIRATION_HOURS=12

# Monitoring Configuration
PROMETHEUS_ENABLED=true
PROMETHEUS_PORT=9090

# Logging Configuration (最佳实践)
LOG_LEVEL=warn
LOG_FORMAT=json
LOG_OUTPUT=file
LOG_PATH=/var/log/metaclouds/backend.log
LOG_MAX_SIZE=100MB
LOG_MAX_BACKUPS=7
LOG_MAX_AGE=30

# Security Configuration
MAX_REQUEST_BODY_SIZE=10485760
READ_TIMEOUT_SECONDS=30
WRITE_TIMEOUT_SECONDS=60
```

## 部署指南

### 使用部署脚本

```bash
# 部署到开发环境
./deploy.sh -e development

# 部署到预发布环境
./deploy.sh -e staging

# 部署到生产环境
./deploy.sh -e production

# 覆盖日志级别
./deploy.sh -e production -l DEBUG

# 查看帮助
./deploy.sh -h
```

### CI/CD 集成

项目使用 GitHub Actions 进行持续集成和部署：

1. **触发方式**：
   - 自动：推送到 `main` 或 `develop` 分支
   - 手动：在 GitHub Actions 中选择环境和日志级别

2. **环境映射**：
   - `develop` 分支 → Development (DEBUG)
   - `main` 分支 → Staging (INFO)
   - 手动触发 → Production (WARN)

## API 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/health` | GET | 健康检查 |
| `/metrics` | GET | Prometheus 指标 |
| `/api/v1/jobs` | GET | 获取作业列表 |
| `/api/v1/jobs` | POST | 创建作业 |
| `/api/v1/jobs/:id/submit` | POST | 提交作业 |
| `/api/v1/jobs/:id/cancel` | POST | 取消作业 |

## 监控指标

### GPU 资源指标

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `gpu_allocated_total` | Counter | GPU 分配总数 |
| `gpu_released_total` | Counter | GPU 释放总数 |
| `gpu_allocation_failed_total` | Counter | GPU 分配失败次数 |
| `gpu_usage_percent` | Gauge | GPU 使用率 |

### 服务指标

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `http_requests_total` | Counter | HTTP 请求总数 |
| `http_request_duration_seconds` | Histogram | 请求耗时 |
| `job_submissions_total` | Counter | 作业提交总数 |

## 最佳实践

### 日志配置最佳实践

1. **开发环境**：使用 `DEBUG` 级别，便于调试
2. **测试环境**：使用 `INFO` 级别，了解业务流程
3. **预发布环境**：使用 `INFO` 级别，接近生产环境
4. **生产环境**：使用 `WARN` 级别，减少日志量

### 安全最佳实践

1. 使用强密码并定期轮换
2. 生产环境启用 SSL/TLS
3. 配置合理的请求超时时间
4. 启用速率限制和熔断机制

### 性能最佳实践

1. 生产环境使用 PostgreSQL 而非 SQLite
2. 启用 Redis 缓存
3. 配置合理的连接池大小
4. 定期清理日志和旧数据

## 目录结构

```
metaclouds-backend/
├── cmd/                    # 命令行入口
├── config/                # 配置文件
│   └── prometheus.yml     # Prometheus 配置
├── deploy/                # 部署脚本
│   ├── utils/             # 工具函数库
│   ├── deploy.sh          # Linux/macOS 部署脚本
│   └── deploy.bat         # Windows 部署脚本
├── docs/                  # 文档
│   └── LOG_LEVEL_CONFIGURATION.md
├── pkg/                   # 公共包
│   ├── logger/            # 日志库
│   └── ...
├── services/              # 业务服务
│   ├── k8s_service.go     # K8S 服务
│   ├── job_service.go     # 作业服务
│   └── ...
├── .env.production        # 生产环境配置
├── .env.staging           # 预发布环境配置
├── .env.development       # 开发环境配置
├── docker-compose.yml     # Docker Compose 配置
└── main.go               # 主入口
```

## 贡献指南

1. Fork 项目
2. 创建特性分支
3. 提交代码
4. 创建 Pull Request

## 许可证

MIT License

## 联系方式

如有问题或建议，请通过以下方式联系：
- 邮箱：dev@metaclouds.com
- 项目地址：<repository-url>