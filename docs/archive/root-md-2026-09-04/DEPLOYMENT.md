
# Metaclouds 部署指南

## 1. 环境要求

### 1.1 硬件要求

| 组件 | CPU | 内存 | 存储 |
|------|-----|------|------|
| 开发环境 | 2核 | 4GB | 20GB |
| 测试环境 | 4核 | 8GB | 50GB |
| 生产环境 | 8核+ | 16GB+ | 100GB+ |

### 1.2 软件要求

| 软件 | 版本 | 说明 |
|------|------|------|
| Docker | 24.0+ | 容器运行时 |
| Docker Compose | 2.20+ | 容器编排 |
| Go | 1.21+ | 后端开发 |
| Node.js | 20+ | 前端开发 |

## 2. 快速开始

### 2.1 克隆项目

```bash
git clone <repository-url>
cd metaclouds
```

### 2.2 使用 Docker Compose 启动

```bash
# 启动所有服务
docker-compose up -d

# 查看服务状态
docker-compose ps

# 查看日志
docker-compose logs -f
```

### 2.3 停止服务

```bash
docker-compose down

# 停止并删除数据卷
docker-compose down -v
```

## 3. 服务访问

### 3.1 服务地址

| 服务 | 地址 | 用户名/密码 |
|------|------|-------------|
| 前端 | http://localhost:3000 | - |
| 后端API | http://localhost:8000 | - |
| Prometheus | http://localhost:9090 | - |
| Grafana | http://localhost:3001 | admin/MetacloudsGrafana2026! |
| Jaeger | http://localhost:16686 | - |
| PostgreSQL | localhost:5432 | metaclouds/MetacloudsSecure2026! |
| Redis | localhost:6379 | MetacloudsRedisSecure2026! |
| etcd | localhost:2379 | - |

### 3.2 API 测试

```bash
# 健康检查
curl http://localhost:8000/health

# 用户登录
curl -X POST http://localhost:8000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "admin123"}'
```

## 4. 配置说明

### 4.1 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| SERVER_PORT | 8000 | 服务端口 |
| SERVER_ENV | development | 运行环境 |
| USE_SQLITE | false | 是否使用SQLite |
| DATABASE_HOST | postgres | 数据库主机 |
| DATABASE_PORT | 5432 | 数据库端口 |
| DATABASE_USER | metaclouds | 数据库用户名 |
| DATABASE_PASSWORD | MetacloudsSecure2026! | 数据库密码 |
| DATABASE_NAME | metaclouds | 数据库名称 |
| REDIS_HOST | redis | Redis主机 |
| REDIS_PORT | 6379 | Redis端口 |
| REDIS_PASSWORD | MetacloudsRedisSecure2026! | Redis密码 |
| JWT_SECRET | metaclouds-secure-jwt-secret-key-2026-production-version-long-enough | JWT密钥 |
| JWT_EXPIRATION_HOURS | 24 | Token过期时间 |
| TRACING_ENABLED | true | 是否启用追踪 |
| CONFIG_CENTER_ENABLED | true | 是否启用配置中心 |

### 4.2 配置文件

```bash
# 复制示例配置
cp .env.example .env

# 编辑配置
vim .env
```

## 5. 开发模式

### 5.1 后端开发

```bash
cd metaclouds-backend

# 安装依赖
go mod download

# 运行开发服务器
go run main.go

# 构建
go build -o metaclouds-backend .

# 运行测试
go test ./...
```

### 5.2 前端开发

```bash
cd metaclouds-frontend

# 安装依赖
npm install

# 运行开发服务器
npm run dev

# 构建生产版本
npm run build
```

## 6. 生产部署

### 6.1 Docker 镜像构建

```bash
# 构建后端镜像
docker build -t metaclouds-backend:latest ./metaclouds-backend

# 构建前端镜像
docker build -t metaclouds-frontend:latest ./metaclouds-frontend
```

### 6.2 Kubernetes 部署（可选）

```bash
# 创建命名空间
kubectl create namespace metaclouds

# 部署应用
kubectl apply -f k8s/
```

### 6.3 健康检查配置

后端服务已内置健康检查端点：

```
GET /health
```

返回示例：
```json
{
    "status": "ok"
}
```

## 7. 数据库管理

### 7.1 初始化数据库

数据库会在首次启动时自动初始化。如需手动初始化：

```bash
docker-compose exec postgres psql -U metaclouds -d metaclouds -f /docker-entrypoint-initdb.d/init.sql
```

### 7.2 数据备份

```bash
# 备份数据库
docker-compose exec postgres pg_dump -U metaclouds metaclouds > backup.sql

# 恢复数据库
cat backup.sql | docker-compose exec -T postgres psql -U metaclouds -d metaclouds
```

## 8. 监控与日志

### 8.1 查看日志

```bash
# 查看所有服务日志
docker-compose logs -f

# 查看特定服务日志
docker-compose logs -f backend
docker-compose logs -f frontend
```

### 8.2 指标访问

```bash
# 后端指标
curl http://localhost:8000/metrics

# Prometheus UI
open http://localhost:9090
```

### 8.3 分布式追踪

```bash
# Jaeger UI
open http://localhost:16686
```

## 9. 故障排查

### 9.1 常见问题

| 问题 | 原因 | 解决方案 |
|------|------|----------|
| 数据库连接失败 | PostgreSQL未启动或配置错误 | 检查数据库配置和容器状态 |
| Redis连接失败 | Redis未启动或密码错误 | 检查Redis配置和容器状态 |
| 端口被占用 | 端口已被其他服务使用 | 修改端口配置或停止占用服务 |
| 服务启动超时 | 依赖服务未就绪 | 检查依赖服务状态 |

### 9.2 日志级别

日志级别可通过环境变量配置：

```bash
# 设置日志级别
LOG_LEVEL=debug
```

支持的级别：debug, info, warn, error

## 10. 安全注意事项

### 10.1 生产环境配置

- 修改默认密码（数据库、Redis、Grafana）
- 使用HTTPS
- 限制访问IP
- 配置防火墙规则

### 10.2 敏感信息管理

- 不要将敏感信息提交到版本控制
- 使用环境变量管理敏感配置
- 定期轮换密钥和密码

## 11. 性能优化

### 11.1 数据库优化

- 创建索引
- 优化查询语句
- 配置连接池

### 11.2 缓存优化

- 使用Redis缓存热点数据
- 配置合理的缓存过期时间

### 11.3 资源限制

```yaml
# docker-compose.yml 中配置资源限制
services:
  backend:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

