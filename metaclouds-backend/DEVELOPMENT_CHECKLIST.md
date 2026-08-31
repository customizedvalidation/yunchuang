# ==============================================================================
# Metaclouds 本地开发环境启动检查清单
# ==============================================================================
# 使用说明：
# 1. 在启动开发服务器前，对照此清单检查所有依赖项
# 2. 标记所有已完成的检查项
# 3. 遇到问题时参考故障排查部分
# ==============================================================================

---

## 📋 前置依赖检查

### 基础工具

| 检查项 | 版本要求 | 检查命令 | 状态 |
|--------|---------|---------|------|
| Go 语言 | >= 1.21 | `go version` | ⬜ |
| Git | >= 2.0 | `git --version` | ⬜ |
| Docker | >= 20.10 | `docker --version` | ⬜ |
| Docker Compose | >= 2.0 | `docker compose version` | ⬜ |

### 可选工具

| 检查项 | 版本要求 | 检查命令 | 状态 | 说明 |
|--------|---------|---------|------|------|
| Node.js | >= 18.0 | `node --version` | ⬜ | 前端开发 |
| npm/yarn/pnpm | - | `npm --version` | ⬜ | 前端包管理 |
| kubectl | >= 1.25 | `kubectl version` | ⬜ | K8s 集群管理 |
| helm | >= 3.0 | `helm version` | ⬜ | K8s 包管理 |

---

## 🔧 环境配置检查

### 配置文件

| 检查项 | 文件路径 | 检查命令 | 状态 |
|--------|---------|---------|------|
| 环境变量文件 | [.env](file:///d:/YCYD/metaclouds-backend/.env) | `ls -la .env` | ⬜ |
| 开发环境配置 | [.env.development](file:///d:/YCYD/metaclouds-backend/.env.development) | `ls -la .env.development` | ⬜ |
| 生产环境配置 | [.env.production](file:///d:/YCYD/metaclouds-backend/.env.production) | `ls -la .env.production` | ⬜ |

### 关键环境变量验证

| 变量名 | 要求 | 检查方法 | 状态 |
|--------|------|---------|------|
| `JWT_SECRET` | 至少 32 字符 | `grep "^JWT_SECRET" .env` | ⬜ |
| `SERVER_PORT` | 有效端口 (1-65535) | `grep "^SERVER_PORT" .env` | ⬜ |
| `SERVER_ENV` | development/staging/production | `grep "^SERVER_ENV" .env` | ⬜ |
| `USE_SQLITE` | true/false | `grep "^USE_SQLITE" .env` | ⬜ |

### 配置验证命令

```bash
# 检查配置文件存在
ls -la .env*

# 验证关键配置
grep -E "^(JWT_SECRET|SERVER_PORT|SERVER_ENV|USE_SQLITE)" .env

# 检查是否有示例值未替换
grep -i "CHANGE\|your-\|example" .env
```

---

## 🚀 服务启动检查

### 依赖服务（可选）

| 服务 | 端口 | 检查命令 | 状态 |
|------|------|---------|------|
| PostgreSQL | 5432 | `pg_isready -h localhost -p 5432` | ⬜ |
| Redis | 6379 | `redis-cli ping` | ⬜ |

### 后端服务启动

| 步骤 | 命令 | 状态 | 说明 |
|------|------|------|------|
| 1. 进入项目目录 | `cd d:\YCYD\metaclouds-backend` | ⬜ | |
| 2. 下载依赖 | `go mod download` | ⬜ | |
| 3. 编译项目 | `go build -o metaclouds-backend.exe .` | ⬜ | |
| 4. 启动服务 | `.\metaclouds-backend.exe` | ⬜ | |

### 使用 Docker Compose 启动（推荐）

| 步骤 | 命令 | 状态 | 说明 |
|------|------|------|------|
| 1. 创建日志目录 | `mkdir -p logs backups` | ⬜ | |
| 2. 启动所有服务 | `docker compose up -d` | ⬜ | |
| 3. 查看服务状态 | `docker compose ps` | ⬜ | |
| 4. 查看日志 | `docker compose logs -f backend` | ⬜ | |

---

## ✅ API 验证检查

### 基础端点测试

| 端点 | 方法 | 命令 | 预期结果 | 状态 |
|------|------|------|---------|------|
| `/health` | GET | `curl http://localhost:8000/health` | `{"status":"healthy"}` | ⬜ |
| `/metrics` | GET | `curl http://localhost:8000/metrics` | Prometheus 指标 | ⬜ |
| `/swagger/index.html` | GET | 浏览器访问 | Swagger UI | ⬜ |

### 认证端点测试

| 端点 | 方法 | 命令 | 预期结果 | 状态 |
|------|------|------|---------|------|
| `/api/v1/auth/login` | POST | 见下方 | 返回 JWT token | ⬜ |
| `/api/v1/auth/register` | POST | 见下方 | 创建新用户 | ⬜ |

```bash
# 登录测试
curl -X POST http://localhost:8000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"$DEFAULT_ADMIN_PASSWORD"}'

# 注册测试
curl -X POST http://localhost:8000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","password":"Test@123!","email":"test@example.com"}'
```

### 业务端点测试

| 端点 | 方法 | 命令 | 状态 |
|------|------|------|------|
| `/api/v1/clusters` | GET | `curl http://localhost:8000/api/v1/clusters` | ⬜ |
| `/api/v1/jobs` | GET | `curl http://localhost:8000/api/v1/jobs` | ⬜ |
| `/api/v1/resources` | GET | `curl http://localhost:8000/api/v1/resources` | ⬜ |

---

## 🧪 测试运行检查

### 单元测试

| 命令 | 说明 | 状态 |
|------|------|------|
| `go test ./...` | 运行所有测试 | ⬜ |
| `go test ./controllers/...` | 运行控制器测试 | ⬜ |
| `go test ./services/...` | 运行服务测试 | ⬜ |
| `go test -v ./tests/...` | 详细输出测试 | ⬜ |

### 覆盖率检查

| 命令 | 说明 | 状态 |
|------|------|------|
| `go test -cover ./...` | 检查测试覆盖率 | ⬜ |
| `go test -coverprofile=coverage.out ./...` | 生成覆盖率报告 | ⬜ |
| `go tool cover -html=coverage.out` | 打开覆盖率报告 | ⬜ |

---

## 📝 开发工具配置

### IDE 配置

| 工具 | 配置文件 | 检查命令 | 状态 |
|------|---------|---------|------|
| Visual Studio Code | `.vscode/settings.json` | `ls -la .vscode/` | ⬜ |
| GoLand | `.idea/workspace.xml` | `ls -la .idea/` | ⬜ |
| 代码格式化 | `.golangci.yml` | `ls -la .golangci.yml` | ⬜ |

### 代码质量检查

| 工具 | 命令 | 状态 |
|------|------|------|
| golangci-lint | `golangci-lint run` | ⬜ |
| gofmt | `gofmt -l .` | ⬜ |
| go vet | `go vet ./...` | ⬜ |

---

## 🗂️ 目录结构检查

### 必需目录

| 目录 | 用途 | 检查命令 | 状态 |
|------|------|---------|------|
| `logs/` | 日志输出 | `ls -la logs/` | ⬜ |
| `backups/` | 备份文件 | `ls -la backups/` | ⬜ |
| `migrations/` | 数据库迁移 | `ls -la migrations/` | ⬜ |
| `deploy/` | 部署脚本 | `ls -la deploy/` | ⬜ |

### 代码目录

| 目录 | 用途 | 状态 |
|------|------|------|
| `api/` | API 路由定义 | ⬜ |
| `controllers/` | REST 控制器 | ⬜ |
| `services/` | 业务逻辑层 | ⬜ |
| `models/` | 数据模型 | ⬜ |
| `middlewares/` | 中间件 | ⬜ |
| `config/` | 配置管理 | ⬜ |
| `pkg/` | 公共包 | ⬜ |

---

## 🔍 故障排查

### 常见问题

| 问题 | 原因 | 解决方案 |
|------|------|---------|
| `JWT_SECRET` 太短 | 安全配置问题 | 使用 `openssl rand -base64 64` 生成 |
| PostgreSQL 连接失败 | 数据库未启动 | `docker compose up -d postgresql` |
| Redis 连接失败 | Redis 未启动 | `docker compose up -d redis` |
| 端口被占用 | 8000 端口被其他服务占用 | 修改 `SERVER_PORT` 或停止占用服务 |
| 编译失败 | 依赖缺失 | `go mod tidy` |

### 日志检查

```bash
# 查看服务日志
tail -f logs/backend.log

# Docker 日志
docker compose logs -f backend

# 检查端口占用
netstat -ano | findstr :8000

# 检查进程
tasklist | findstr metaclouds
```

---

## 📊 启动成功验证清单

### 服务状态

```bash
# 1. 检查服务是否启动
curl -s http://localhost:8000/health | jq .

# 2. 检查 API 响应
curl -s http://localhost:8000/api/v1/clusters | jq .

# 3. 检查指标端点
curl -s http://localhost:8000/metrics | head -20

# 4. 检查调度器日志
docker compose logs backend | grep "schedule"

# 5. 验证登录功能
curl -s -X POST http://localhost:8000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"$DEFAULT_ADMIN_PASSWORD"}' | jq .success
```

### 预期输出

```bash
# 健康检查
{"status":"healthy","timestamp":1779871594}

# 登录成功
true

# 服务日志包含
"Metaclouds Backend is ready!"
"Server starting on 0.0.0.0:8000"
```

---

## 📝 启动命令速查

### 开发模式

```bash
# 方式一：直接运行（需要手动启动依赖）
go run main.go

# 方式二：编译后运行
go build -o metaclouds-backend.exe .
.\metaclouds-backend.exe

# 方式三：使用 Docker Compose（推荐）
docker compose up -d
docker compose logs -f backend
```

### 停止服务

```bash
# 停止 Docker Compose 服务
docker compose down

# 停止直接运行的服务
Ctrl+C 或 kill <pid>
```

---

## ✅ 检查完成确认

| 阶段 | 状态 |
|------|------|
| 前置依赖 | ⬜ |
| 环境配置 | ⬜ |
| 服务启动 | ⬜ |
| API 验证 | ⬜ |
| 测试运行 | ⬜ |

---

**最后更新**: 2024-05-27
**版本**: 1.0.0