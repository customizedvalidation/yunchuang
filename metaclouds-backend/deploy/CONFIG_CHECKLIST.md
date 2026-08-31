# Metaclouds 环境配置完整清单

## 📁 配置文件总览

### 根目录配置文件

| 文件 | 说明 | 状态 |
|------|------|------|
| `.env.production.example` | 生产环境配置示例 | ✅ |
| `.env.development.example` | 开发环境配置示例 | ✅ |
| `.gitignore` | Git 忽略文件配置 | ✅ |

### 部署脚本目录 (`deploy/`)

| 文件 | 说明 | 状态 |
|------|------|------|
| `README.md` | 部署脚本说明文档 | ✅ |
| `DEPENDENCIES.md` | 依赖检查文档 | ✅ |
| `DEPLOYMENT_QUICKSTART.md` | 快速部署指南 | ✅ |
| `install.sh` | 一键安装脚本 | ✅ |
| `verify_deployment.sh` | 部署验证脚本 | ✅ |
| `validate_env.sh` | 环境变量验证脚本 | ✅ |
| `backup_postgresql.sh` | PostgreSQL 备份脚本 | ✅ |
| `verify_backup.sh` | 备份验证脚本 | ✅ |
| `collect_metrics.sh` | Prometheus 指标收集脚本 | ✅ |
| `health_check.sh` | 健康检查脚本 | ✅ |
| `check_disk_space.sh` | 磁盘空间检查脚本 | ✅ |
| `metaclouds.logrotate` | logrotate 日志轮转配置 | ✅ |
| `metaclouds-cron` | Linux cron 定时任务配置 | ✅ |

### Kubernetes 配置目录 (`deploy/kubernetes/`)

| 文件 | 说明 | 状态 |
|------|------|------|
| `metaclouds-secrets.yaml` | Kubernetes Secret 配置 | ✅ |
| `backup-cronjob.yaml` | Kubernetes CronJob 备份配置 | ✅ |

---

## 🚀 快速开始

### 1. 复制配置文件
```bash
# 生产环境
cp .env.production.example .env.production

# 开发环境
cp .env.development.example .env.development
```

### 2. 修改必填配置
```bash
nano .env.production
```

必填配置：
- `SECURE_JWT_SECRET` - JWT 密钥
- `SECURE_DEFAULT_ADMIN_PASSWORD` - 管理员密码
- `SECURE_DATABASE_PASSWORD` - 数据库密码
- `ALLOWED_ORIGINS` - CORS 允许的源

### 3. 验证配置
```bash
chmod +x deploy/validate_env.sh
./deploy/validate_env.sh -f .env.production
```

### 4. 安装和部署
```bash
chmod +x deploy/install.sh
./deploy/install.sh
```

---

## 📊 环境变量分类

### 必需配置（安全相关）

| 变量 | 说明 | 示例 |
|------|------|------|
| `SECURE_JWT_SECRET` | JWT 签名密钥 | `openssl rand -base64 64` |
| `SECURE_DEFAULT_ADMIN_PASSWORD` | 默认管理员密码 | `StrongPassword123!` |
| `SECURE_DATABASE_PASSWORD` | 数据库密码 | `DBPassword123!` |

### 服务器配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `SERVER_PORT` | 8000 | 服务端口 |
| `SERVER_HOST` | 0.0.0.0 | 监听地址 |
| `SERVER_ENV` | production | 运行环境 |
| `ALLOWED_ORIGINS` | - | CORS 允许源 |

### 数据库配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `USE_SQLITE` | false | 使用 SQLite |
| `MEMORY_STORE_ENABLED` | false | 使用内存存储 |
| `DATABASE_HOST` | localhost | 数据库主机 |
| `DATABASE_PORT` | 5432 | 数据库端口 |
| `DATABASE_NAME` | metaclouds | 数据库名 |
| `DATABASE_USER` | metaclouds_user | 数据库用户 |
| `DATABASE_SSL_MODE` | require | SSL 模式 |

### Redis 配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `REDIS_ENABLED` | true | 启用 Redis |
| `REDIS_HOST` | localhost | Redis 主机 |
| `REDIS_PORT` | 6379 | Redis 端口 |
| `REDIS_DB` | 0 | Redis 数据库 |

### 日志配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `LOG_LEVEL` | warn | 日志级别 |
| `LOG_FORMAT` | json | 日志格式 |
| `LOG_OUTPUT` | file | 输出位置 |
| `LOG_PATH` | /var/log/metaclouds/backend.log | 日志路径 |

### 限流配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RATE_LIMIT_ENABLED` | true | 启用限流 |
| `RATE_LIMIT_REQUESTS` | 1000 | 请求阈值 |
| `RATE_LIMIT_DURATION_SECONDS` | 60 | 时间窗口 |

### 熔断器配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `CIRCUIT_BREAKER_ENABLED` | true | 启用熔断器 |
| `CIRCUIT_BREAKER_THRESHOLD` | 50 | 失败阈值 |
| `CIRCUIT_BREAKER_TIMEOUT_SECONDS` | 30 | 熔断超时 |

### 监控配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `PROMETHEUS_ENABLED` | true | 启用 Prometheus |
| `METRICS_COLLECTION_INTERVAL` | 15 | 收集间隔(秒) |

### 超时配置

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `READ_TIMEOUT_SECONDS` | 30 | 读取超时 |
| `WRITE_TIMEOUT_SECONDS` | 60 | 写入超时 |
| `IDLE_TIMEOUT_SECONDS` | 120 | 空闲超时 |
| `MAX_REQUEST_BODY_SIZE` | 10485760 | 最大请求体 |

---

## 🔐 安全建议

### 1. 生产环境必须修改的配置
- [ ] JWT 密钥（至少 32 字符，建议 64+）
- [ ] 所有密码（至少 8 字符，包含大小写和数字）
- [ ] CORS 源（禁止使用 localhost）
- [ ] 数据库 SSL 模式（使用 require 或 verify-full）

### 2. 不应该在生产环境使用的配置
- [ ] `USE_SQLITE=true` - 生产环境必须使用 PostgreSQL
- [ ] `MEMORY_STORE_ENABLED=true` - 生产环境必须使用数据库
- [ ] `LOG_LEVEL=debug` - 生产环境使用 warn 或 error
- [ ] `RATE_LIMIT_ENABLED=false` - 生产环境必须启用限流

### 3. 密码生成命令
```bash
# 生成 JWT 密钥
openssl rand -base64 64

# 生成密码
openssl rand -base64 32

# 生成随机字符串
cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 32 | head -n 1
```

---

## 📝 版本历史

- **v2.0.0** (2024-05)
  - 添加完整的环境变量配置
  - 添加 Kubernetes Secret 配置
  - 添加配置验证脚本
  - 添加 .gitignore 配置
  - 添加快速部署指南

- **v1.0.0** (2024-04)
  - 初始版本
