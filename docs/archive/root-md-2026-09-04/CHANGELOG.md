# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased] - 2026-05-27

### ✅ Security Enhancements

| File | Change | Description |
|------|--------|-------------|
| `middlewares/security_headers.go` | CSP Policy Improvement | 区分生产/开发环境的 CSP 策略，生产环境禁用 `unsafe-inline` |
| `middlewares/security_headers.go` | Server Header | 添加 Server 响应头为 "Metaclouds" |

### ✅ Backend Improvements

**Error Handling**
- `main.go`: 重构为 `run()` 函数模式，使用错误返回替代直接 `os.Exit()`
- `main.go`: 使用 `fmt.Errorf("xxx: %w", err)` 包装错误，保留完整错误链

**Logging**
- `config/config.go`: 改用统一 logger 替代标准库 log
- `middlewares/request_logger.go`: 添加慢请求检测（可配置阈值）
- `middlewares/request_logger.go`: 限制请求体读取大小（<1MB）
- `middlewares/request_logger.go`: 限制响应体日志大小（<10KB）
- `middlewares/request_logger.go`: 使用 `io.ReadAll` 替代已弃用的 `ioutil.ReadAll`

**Code Quality**
- `models/db.go`: 添加 bcrypt 密码哈希错误处理

### ✅ Frontend Improvements

**API Layer**
- `src/store/api.ts`: 添加 30 秒超时配置
- `src/store/api.ts`: 添加标准请求头（Content-Type, Accept）
- `src/store/api.ts`: 添加 401 自动重定向登录

**Authentication**
- `src/components/PrivateRoute.tsx`: 添加 Token 过期时间验证
- `src/components/PrivateRoute.tsx`: 添加加载状态显示

### ✅ API Security

**JWT Middleware**
- `middlewares/jwt_auth.go`: 添加详细的认证日志
- `middlewares/jwt_auth.go`: 实现 Token 脱敏显示（保留首尾各8位）
- `middlewares/jwt_auth.go`: 实现请求体密码脱敏

### ✅ Bug Fixes

| Issue | Fix |
|-------|-----|
| Swagger UI 黑屏 | 更新 CSP 允许 cdnjs.cloudflare.com |
| 前端页面无内容 | 添加缺失的 vite.svg 图标文件 |
| 密码哈希错误忽略 | 添加 bcrypt 错误处理 |
| 服务启动错误处理 | 重构 main.go 使用错误返回模式 |

### ✅ Configuration

**New Environment Variables**
- `SLOW_REQUEST_THRESHOLD_MS`: 慢请求阈值（毫秒），默认 2000ms

**Updated Defaults**
- `SERVER_PORT`: 默认端口从 3000 改为 8000

### ✅ Testing & Validation

| Test Case | Result |
|-----------|--------|
| Health Check (`/health`) | ✅ Pass |
| User Login (`/api/v1/auth/login`) | ✅ Pass |
| JWT Authentication (`/api/v1/auth/profile`) | ✅ Pass |
| Frontend Page (`/app`) | ✅ Pass |
| Password Redaction | ✅ Pass |
| Token Masking | ✅ Pass |

---

## [1.0.0] - Initial Release

### Features

- User authentication with JWT
- Cluster management API
- Job scheduling system
- Resource monitoring
- Tenant management
- Swagger API documentation
- Frontend dashboard with React

### Technical Stack

- Backend: Go 1.21 + Gin Framework
- Frontend: React 18 + TypeScript + Redux Toolkit
- Database: SQLite (dev) / PostgreSQL (prod)
- Authentication: JWT