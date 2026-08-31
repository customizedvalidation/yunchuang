
# Metaclouds 系统架构文档

## 1. 系统概述

Metaclouds 是一个企业级云资源管理平台，提供GPU加速计算资源管理、Kubernetes集群管理、作业调度和监控告警等核心功能。

## 2. 技术栈

| 分类 | 技术 | 版本 |
|------|------|------|
| 语言 | Go | 1.21 |
| 框架 | Gin | 1.9.x |
| ORM | GORM | 1.25.x |
| 数据库 | PostgreSQL | 16 |
| 缓存 | Redis | 7 |
| 配置中心 | etcd | 3.5.x |
| 监控 | Prometheus | 2.48.x |
| 可视化 | Grafana | 10.2.x |
| 追踪 | Jaeger | 1.53.x |
| 容器 | Docker | 24.x |
| 编排 | Docker Compose | 3.8 |

## 3. 架构设计

### 3.1 架构风格

采用集成式单体应用（Integrated Monolith）架构，为后续微服务拆分做好准备。

### 3.2 模块划分

```
metaclouds-backend/
├── api/           # REST API 路由定义
├── config/        # 配置管理
├── controllers/   # REST API 控制器
├── middlewares/   # HTTP 中间件
├── models/        # 数据模型
├── services/      # 业务逻辑服务
└── pkg/           # 通用工具包
    ├── errors/    # 统一错误处理
    ├── logger/    # 结构化日志
    ├── middleware/# 中间件（限流、熔断等）
    └── response/  # 统一响应格式
```

### 3.3 核心组件

| 组件 | 职责 | 状态 |
|------|------|------|
| AuthService | 用户认证与授权 | 已实现 |
| ClusterService | 集群管理 | 已实现 |
| ResourceService | 资源管理 | 已实现 |
| JobService | 作业管理 | 已实现 |
| K8SService | Kubernetes集成 | 已实现 |
| MonitoringService | 监控告警 | 已实现 |
| ConfigCenter | 配置中心 | 已实现 |
| TracingService | 分布式追踪 | 已实现 |
| Scheduler | 定时任务调度 | 已实现 |

### 3.4 关键流程图

#### 3.4.1 请求处理流程

```
客户端请求 → RateLimit → CircuitBreaker → Auth → Controller → Service → Repository → DB
                                              ↓
                                       Logger/Tracing
```

#### 3.4.2 认证流程

```
POST /auth/login → AuthController → AuthService.ValidateUser()
                                          ↓
                                   验证用户名密码
                                          ↓
                                   生成JWT Token
                                          ↓
                                   Token存入Redis
                                          ↓
                                   返回Token给客户端
```

## 4. API 设计

### 4.1 统一响应格式

```json
{
    "success": true,
    "data": {},
    "message": "操作成功",
    "code": "SUCCESS",
    "timestamp": 1704067200
}
```

### 4.2 错误响应格式

```json
{
    "success": false,
    "message": "用户名或密码错误",
    "code": "UNAUTHORIZED",
    "timestamp": 1704067200
}
```

### 4.3 错误码定义

| 错误码 | HTTP状态码 | 含义 |
|--------|-----------|------|
| INTERNAL_ERROR | 500 | 内部服务器错误 |
| BAD_REQUEST | 400 | 请求参数错误 |
| UNAUTHORIZED | 401 | 未授权 |
| FORBIDDEN | 403 | 禁止访问 |
| NOT_FOUND | 404 | 资源未找到 |
| CONFLICT | 409 | 资源冲突 |
| VALIDATION_ERROR | 400 | 验证错误 |
| RATE_LIMITED | 429 | 请求限流 |
| SERVICE_UNAVAILABLE | 503 | 服务不可用 |

## 5. 数据库设计

### 5.1 核心表结构

#### 5.1.1 users 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PRIMARY KEY | 用户ID |
| username | VARCHAR(50) | UNIQUE NOT NULL | 用户名 |
| password_hash | VARCHAR(255) | NOT NULL | 密码哈希 |
| email | VARCHAR(100) | UNIQUE | 邮箱 |
| role | VARCHAR(20) | NOT NULL | 角色 |
| tenant_id | BIGINT | FOREIGN KEY | 租户ID |
| created_at | TIMESTAMP | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMP | DEFAULT NOW() | 更新时间 |

#### 5.1.2 clusters 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PRIMARY KEY | 集群ID |
| name | VARCHAR(100) | NOT NULL | 集群名称 |
| endpoint | VARCHAR(255) | NOT NULL | API端点 |
| status | VARCHAR(20) | NOT NULL | 状态 |
| tenant_id | BIGINT | FOREIGN KEY | 租户ID |
| created_at | TIMESTAMP | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMP | DEFAULT NOW() | 更新时间 |

#### 5.1.3 jobs 表

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PRIMARY KEY | 作业ID |
| name | VARCHAR(100) | NOT NULL | 作业名称 |
| type | VARCHAR(20) | NOT NULL | 作业类型 |
| status | VARCHAR(20) | NOT NULL | 状态 |
| priority | INT | DEFAULT 0 | 优先级 |
| cluster_id | BIGINT | FOREIGN KEY | 集群ID |
| tenant_id | BIGINT | FOREIGN KEY | 租户ID |
| created_at | TIMESTAMP | DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMP | DEFAULT NOW() | 更新时间 |

## 6. 安全设计

### 6.1 认证机制

- JWT Token认证
- Token过期时间：24小时
- Token黑名单机制（Redis）

### 6.2 权限控制

- 基于角色的访问控制（RBAC）
- 租户隔离

### 6.3 安全防护

| 防护措施 | 实现方式 |
|----------|----------|
| 限流 | 滑动窗口算法 |
| 熔断 | 熔断器模式 |
| 输入验证 | go-playground/validator |
| CORS | Gin CORS中间件 |
| 日志审计 | Zerolog结构化日志 |

## 7. 可观测性

### 7.1 日志

- 结构化日志（JSON格式）
- 生产/开发环境差异化配置
- 自动注入Trace ID

### 7.2 指标

- Prometheus指标暴露
- 自定义业务指标
- 定时指标收集

### 7.3 追踪

- OpenTelemetry集成
- Jaeger分布式追踪
- 自动追踪HTTP请求

## 8. 部署与运维

### 8.1 环境配置

| 环境 | 配置文件 | 说明 |
|------|----------|------|
| 开发 | .env | 本地开发配置 |
| 测试 | .env.test | 测试环境配置 |
| 生产 | Docker环境变量 | 生产环境配置 |

### 8.2 Docker Compose 服务

| 服务 | 端口 | 说明 |
|------|------|------|
| postgres | 5432 | 数据库 |
| redis | 6379 | 缓存 |
| prometheus | 9090 | 监控 |
| grafana | 3001 | 可视化 |
| etcd | 2379 | 配置中心 |
| jaeger | 16686 | 追踪UI |
| backend | 8000 | 后端API |
| frontend | 3000 | 前端应用 |

### 8.3 健康检查

```dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1
```

## 9. 扩展规划

### 9.1 微服务拆分

| 服务 | 职责 | 状态 |
|------|------|------|
| auth-service | 认证授权 | 待拆分 |
| cluster-service | 集群管理 | 待拆分 |
| job-service | 作业调度 | 待拆分 |
| monitoring-service | 监控告警 | 待拆分 |

### 9.2 功能扩展

- [ ] 多租户增强
- [ ] 资源配额管理
- [ ] 成本核算
- [ ] 自动化运维
- [ ] AI模型管理

## 10. 版本历史

| 版本 | 日期 | 变更内容 |
|------|------|----------|
| v1.0 | 2026-05 | 初始版本 |
| v1.1 | 2026-05 | 添加配置中心、全链路追踪 |
| v1.2 | 2026-05 | 添加服务治理（限流、熔断） |

