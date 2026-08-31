# CI/CD 流水线运行日志报告

---

## 📋 报告信息

| 项目 | 内容 |
|------|------|
| **报告生成时间** | 2026-05-27 15:09:34 |
| **项目名称** | Metaclouds Backend |
| **报告版本** | v1.0 |
| **环境** | test |
| **Commit SHA** | N/A |
| **Branch** | N/A |
| **Workflow** | N/A |

---

## 🚀 流水线配置

### 触发条件
- **push**: main, develop 分支
- **pull_request**: main, develop 分支
- **workflow_dispatch**: 手动触发

### 测试阶段

| 阶段 | 命令 | 超时 |
|------|------|------|
| 单元测试 | `go test ./services/... ./pkg/... ./models/...` | 60s |
| 监控服务测试 | `go test -run TestMonitoring -covermode=atomic` | 60s |
| 集成测试 | `go test ./tests/...` | 120s |
| 中间件测试 | `go test ./middlewares/...` | 60s |
| E2E测试 | `go test -run TestE2E` | 120s |

---

## 🧪 测试执行结果

### 监控服务测试

**运行命令**: `go test -v -coverprofile=monitoring_coverage.out -covermode atomic ./services/ -run TestMonitoring -timeout 60s`

