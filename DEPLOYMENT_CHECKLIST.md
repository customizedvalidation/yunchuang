# Metaclouds 生产环境部署配置检查清单

## 📋 概述

本清单用于检查 Metaclouds 后端服务在生产环境部署前的所有必需和推荐配置项。

**部署前请务必完成所有 **✅ 必填** 项的配置。

---

## 🔐 安全配置（✅ 必填

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **JWT 签名密钥** | `JWT_SECRET` | ✅ 必填 | 随机字符串，至少32字符 | 用于 JWT token 签名和验证，务必使用强随机密码 | ☐ |
| **默认管理员密码** | `DEFAULT_ADMIN_PASSWORD` | ✅ 必填 | 强密码（至少8字符，含大小写、数字、特殊字符） | 默认管理员账号密码 | ☐ |
| **默认用户密码** | `DEFAULT_USER_PASSWORD` | ☐ 推荐 | 强密码（至少8字符，含大小写、数字、特殊字符） | 默认普通用户账号密码 | ☐ |

### ⚠️ 重要安全提示

1. **JWT_SECRET 生成建议：
   ```bash
   # 使用以下命令生成安全的随机密钥
   openssl rand -base64 32
   # 或使用 Python 生成
   python -c "import secrets; print(secrets.token_urlsafe(32))"
   ```

2. **密码安全要求：
   - 至少8-16个字符
   - 包含大写字母
   - 包含小写字母
   - 包含数字
   - 包含特殊字符（!@#$%^&*）

---

## 🗄️ 数据库配置（✅ 必填）

### PostgreSQL 配置

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **数据库模式** | `USE_SQLITE` | ✅ 必填 | `false` | 生产环境使用 PostgreSQL | ☐ |
| **数据库主机** | `DATABASE_HOST` | ✅ 必填 | 数据库服务器地址 | PostgreSQL 数据库主机 | ☐ |
| **数据库端口** | `DATABASE_PORT` | ✅ 必填 | `5432` | PostgreSQL 端口 | ☐ |
| **数据库用户名** | `DATABASE_USER` | ✅ 必填 | `metaclouds` | 数据库用户名 | ☐ |
| **数据库密码** | `DATABASE_PASSWORD` | ✅ 必填 | 强密码 | 数据库密码 | ☐ |
| **数据库名称** | `DATABASE_NAME` | ✅ 必填 | `metaclouds` | 数据库名称 | ☐ |
| **SSL 模式** | `DATABASE_SSL_MODE` | ✅ 必填 | `require` | 生产环境启用 SSL | ☐ |
| **内存存储模式** | `MEMORY_STORE_ENABLED` | ☐ 推荐 | `false` | 生产环境建议使用真实数据库 | ☐ |

---

## 🔄 Redis 缓存配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用 Redis** | `REDIS_ENABLED` | ☐ 推荐 | `true` | 生产环境推荐启用 Redis 缓存 | ☐ |
| **Redis 主机** | `REDIS_HOST` | ☐ 推荐 | Redis 服务器地址 | Redis 主机 | ☐ |
| **Redis 端口** | `REDIS_PORT` | ☐ 推荐 | `6379` | Redis 端口 | ☐ |
| **Redis 密码** | `REDIS_PASSWORD` | ☐ 推荐 | 强密码 | Redis 密码（生产环境必填） | ☐ |
| **Redis DB** | `REDIS_DB` | ☐ 推荐 | `0` | Redis 数据库编号 | ☐ |

---

## 🌐 服务器配置（✅ 必填）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **服务端口** | `SERVER_PORT` | ✅ 必填 | `8000` | 服务监听端口 | ☐ |
| **服务主机** | `SERVER_HOST` | ✅ 必填 | `0.0.0.0` | 服务监听地址 | ☐ |
| **环境标识** | `SERVER_ENV` | ✅ 必填 | `production` | 环境标识（production/staging/development） | ☐ |

---

## 📊 Prometheus 监控配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用 Prometheus** | `PROMETHEUS_ENABLED` | ☐ 推荐 | `true` | 启用 Prometheus 指标 | ☐ |
| **Prometheus 端口** | `PROMETHEUS_PORT` | ☐ 推荐 | `9090` | Prometheus 端口 | ☐ |

---

## 📈 监控与告警配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用监控** | `MONITORING_ENABLED` | ☐ 推荐 | `true` | 启用监控功能 | ☐ |
| **启用告警** | `ALERT_ENABLED` | ☐ 推荐 | `true` | 启用告警功能 | ☐ |
| **指标采集间隔** | `METRICS_COLLECTION_INTERVAL_SECONDS` | ☐ 推荐 | `15` | 采集间隔（秒） | ☐ |
| **慢请求阈值** | `SLOW_REQUEST_THRESHOLD_MS` | ☐ 推荐 | `2000` | 慢请求阈值（毫秒） | ☐ |

---

## ☸️ Kubernetes 配置（☐ 根据需求）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用 K8s** | `K8S_ENABLED` | ☐ 根据需求 | `true` | 启用 K8s 集成 | ☐ |
| **K8s 命名空间** | `K8S_NAMESPACE` | ☐ 推荐 | `metaclouds` | K8s 命名空间 | ☐ |
| **K8s 配置路径** | `K8S_CONFIG_PATH` | ☐ 推荐 | `~/.kube/config` | K8s 配置文件路径 | ☐ |
| **K8s 模拟模式** | `K8S_SIMULATION_MODE` | ☐ 根据需求 | `false` | 生产环境禁用模拟模式 | ☐ |

---

## 📅 调度器配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用调度器** | `SCHEDULER_ENABLED` | ☐ 推荐 | `true` | 启用任务调度器 | ☐ |
| **调度间隔** | `SCHEDULER_INTERVAL_SECONDS` | ☐ 推荐 | `10` | 调度检查间隔（秒） | ☐ |

---

## 🚦 限流与熔断配置（✅ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **启用限流** | `RATE_LIMIT_ENABLED` | ✅ 推荐 | `true` | 启用速率限制 | ☐ |
| **限流请求数** | `RATE_LIMIT_REQUESTS` | ☐ 推荐 | `100` | 单位时间内最大请求数 | ☐ |
| **限流时间窗口** | `RATE_LIMIT_DURATION_SECONDS` | ☐ 推荐 | `60` | 限流时间窗口（秒） | ☐ |
| **启用熔断** | `CIRCUIT_BREAKER_ENABLED` | ✅ 推荐 | `true` | 启用熔断机制 | ☐ |
| **熔断阈值** | `CIRCUIT_BREAKER_THRESHOLD` | ☐ 推荐 | `10` | 连续失败次数阈值 | ☐ |
| **熔断超时** | `CIRCUIT_BREAKER_TIMEOUT_SECONDS` | ☐ 推荐 | `30` | 熔断恢复等待时间（秒） | ☐ |

---

## 📝 日志配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **日志级别** | `LOG_LEVEL` | ☐ 推荐 | `info` | 日志级别（debug/info/warn/error） | ☐ |
| **日志格式** | `LOG_FORMAT` | ☐ 推荐 | `json` | 日志格式（json/console） | ☐ |
| **日志输出** | `LOG_OUTPUT` | ☐ 推荐 | `console` | 日志输出（console/file） | ☐ |

---

## ⏱️ 服务器超时配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **最大请求体大小** | `MAX_REQUEST_BODY_SIZE` | ☐ 推荐 | `10485760` | 最大请求体大小（字节，默认10MB） | ☐ |
| **读取超时** | `READ_TIMEOUT_SECONDS` | ☐ 推荐 | `30` | 读取超时（秒） | ☐ |
| **写入超时** | `WRITE_TIMEOUT_SECONDS` | ☐ 推荐 | `30` | 写入超时（秒） | ☐ |
| **空闲超时** | `IDLE_TIMEOUT_SECONDS` | ☐ 推荐 | `60` | 空闲超时（秒） | ☐ |

---

## 📖 分页配置（☐ 推荐）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|
| **默认分页大小** | `DEFAULT_PAGE_SIZE` | ☐ 推荐 | `10` | 默认分页大小 | ☐ |
| **最大分页大小** | `MAX_PAGE_SIZE` | ☐ 推荐 | `100` | 最大分页大小 | ☐ |

---

## 🎛️ 功能开关配置（☐ 根据需求）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|-------|
| **GPU 分配功能** | `FEATURE_GPU_ALLOCATION` | ☐ 根据需求 | `true` | 启用 GPU 资源分配 | ☐ |
| **任务调度功能** | `FEATURE_JOB_SCHEDULER` | ☐ 根据需求 | `true` | 启用任务调度 | ☐ |
| **监控功能** | `FEATURE_MONITORING` | ☐ 根据需求 | `true` | 启用监控功能 | ☐ |
| **安全策略功能** | `FEATURE_SECURITY_POLICIES` | ☐ 根据需求 | `true` | 启用安全策略 | ☐ |

---

## 🔍 Jaeger 链路追踪（☐ 可选）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|
| **启用链路追踪** | `TRACING_ENABLED` | ☐ 可选 | `false` | 启用链路追踪 | ☐ |
| **追踪服务名** | `TRACING_SERVICE_NAME` | ☐ 可选 | `metaclouds-backend` | 追踪服务名称 | ☐ |
| **Jaeger 端点** | `JAEGER_ENDPOINT` | ☐ 可选 | `http://localhost:14268/api/traces` | Jaeger 追踪端点 | ☐ |

---

## 🔧 配置中心（☐ 可选）

| 配置项 | 环境变量 | 是否必填 | 建议值 | 说明 | 状态 |
|---------|----------|---------|-------|-------|
| **启用配置中心** | `CONFIG_CENTER_ENABLED` | ☐ 可选 | `false` | 启用配置中心 | ☐ |
| **配置中心端点** | `CONFIG_CENTER_ENDPOINTS` | ☐ 可选 | `localhost:2379` | 配置中心地址 | ☐ |
| **配置中心前缀** | `CONFIG_CENTER_PREFIX` | ☐ 可选 | `/metaclouds/config/` | 配置中心键前缀 | ☐ |

---

## 📦 生产环境配置模板

### .env.production 示例

```env
# ==================== 安全配置（✅ 必填
JWT_SECRET=your_secure_jwt_secret_key_here_at_least_32_characters
DEFAULT_ADMIN_PASSWORD=YourStrongAdminPass123!
DEFAULT_USER_PASSWORD=YourStrongUserPass123!

# ==================== 数据库配置（✅ 必填）
USE_SQLITE=false
MEMORY_STORE_ENABLED=false
DATABASE_HOST=your-database-host
DATABASE_PORT=5432
DATABASE_USER=metaclouds
DATABASE_PASSWORD=your_secure_database_password
DATABASE_NAME=metaclouds
DATABASE_SSL_MODE=require

# ==================== Redis 缓存配置（☐ 推荐）
REDIS_ENABLED=true
REDIS_HOST=your-redis-host
REDIS_PORT=6379
REDIS_PASSWORD=your_secure_redis_password
REDIS_DB=0

# ==================== 服务器配置（✅ 必填）
SERVER_PORT=8000
SERVER_HOST=0.0.0.0
SERVER_ENV=production

# ==================== Prometheus 监控配置（☐ 推荐）
PROMETHEUS_ENABLED=true
PROMETHEUS_PORT=9090

# ==================== 监控与告警配置（☐ 推荐）
MONITORING_ENABLED=true
ALERT_ENABLED=true
METRICS_COLLECTION_INTERVAL_SECONDS=15
SLOW_REQUEST_THRESHOLD_MS=2000

# ==================== Kubernetes 配置（☐ 根据需求）
K8S_ENABLED=true
K8S_NAMESPACE=metaclouds
K8S_CONFIG_PATH=/path/to/kubeconfig
K8S_SIMULATION_MODE=false

# ==================== 调度器配置（☐ 推荐）
SCHEDULER_ENABLED=true
SCHEDULER_INTERVAL_SECONDS=10

# ==================== 限流与熔断配置（✅ 推荐）
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_DURATION_SECONDS=60
CIRCUIT_BREAKER_ENABLED=true
CIRCUIT_BREAKER_THRESHOLD=10
CIRCUIT_BREAKER_TIMEOUT_SECONDS=30

# ==================== 日志配置（☐ 推荐）
LOG_LEVEL=info
LOG_FORMAT=json
LOG_OUTPUT=console

# ==================== 服务器超时配置（☐ 推荐）
MAX_REQUEST_BODY_SIZE=10485760
READ_TIMEOUT_SECONDS=30
WRITE_TIMEOUT_SECONDS=30
IDLE_TIMEOUT_SECONDS=60

# ==================== 分页配置（☐ 推荐）
DEFAULT_PAGE_SIZE=10
MAX_PAGE_SIZE=100

# ==================== 功能开关配置（☐ 根据需求）
FEATURE_GPU_ALLOCATION=true
FEATURE_JOB_SCHEDULER=true
FEATURE_MONITORING=true
FEATURE_SECURITY_POLICIES=true
```

---

## ✅ 部署前检查清单

### 1. 安全检查
- [ ] 所有敏感信息已从代码中移除
- [ ] JWT_SECRET 已配置为强随机密钥
- [ ] 数据库密码已配置为强密码
- [ ] Redis 密码已配置为强密码（如启用）
- [ ] 默认用户密码已配置为强密码
- [ ] 所有默认配置已记录在安全的密码管理系统中
- [ ] 已准备好密码轮换机制

### 2. 基础设施检查
- [ ] PostgreSQL 数据库已准备就绪
- [ ] 数据库用户已创建并授权
- [ ] Redis 已准备就绪（如使用）
- [ ] 网络连接已测试
- [ ] SSL/TLS 证书已配置（如需要）
- [ ] 备份策略已制定

### 3. 监控检查
- [ ] 所有必填配置项已检查通过
- [ ] 推荐配置项已根据需求配置
- [ ] 日志配置已确认
- [ ] 监控与告警配置已确认
- [ ] 限流与熔断配置已确认

### 4. 部署后检查
- [ ] 服务启动正常
- [ ] 健康检查端点可访问
- [ ] 登录功能正常
- [ ] 日志输出正常
- [ ] 监控指标可采集
- [ ] 告警通道已验证
- [ ] 限流与安全扫描已通过

---

## ⚠️ 重要提示

1. **永远不要将 `.env` 文件提交到版本控制系统中
2. 使用环境变量管理系统（如 Kubernetes Secrets、Vault）管理敏感信息
3. 定期轮换密码和密钥
4. 启用日志审查和监控
5. 建立备份策略和灾难恢复计划
