# Metaclouds 后端系统复盘与优化报告

> 生成时间：2026-08-30
> 范围：`metaclouds-backend`（Go + Gin）、`metaclouds-frontend`、`prometheus.yml` / `alerts.yml` / `docker-compose.yml`
> 验证环境：Go 1.25（`~/sdk/go1.25.0`）、Windows、离线沙箱
> 验证结果：`go build ./...` ✅ · `go vet ./...` ✅ · `go test -race ./...` ✅（3 个包全绿）

---

## 一、本轮修复的问题（高严重度）

### P0 安全
| # | 问题 | 根因 | 修复 | 文件 |
|---|------|------|------|------|
| 1 | **注册接口任意提权** | `Register` 请求体直接取 `req.Role`，客户端可自填 `admin` | `Role` 一律忽略，强制按 `isAdmin` 派生；仅管理员可指定角色 | `services/auth_service.go` |
| 2 | **作业接口跨租户越权（IDOR）** | `GetJobs`/`GetJob` 无租户过滤，任意租户可读取全部作业 | 新增 `GetJobsVisibleTo` / `GetJobVisibleTo` / `CreateJobForUser` / `UpdateJobForTenant` / `DeleteJobForTenant` / `CancelJobForTenant`，越权返回 404（防资源探测） | `services/job_service.go`、`controllers/job_controller.go` |
| 3 | **生产凭据入库** | `.env.production` 等被 `git add -f` 进版本库（提交 `386d48c`），含数据库口令 | 已从 git 索引取消追踪（`git rm --cached`，工作区文件保留）；`.gitignore` 本已覆盖 | `.gitignore`（已存在）+ git 索引 |
| 4 | **未鉴权 DoS 端点** | `/api/v1/test/slow` 匿名可访问且 `delay` 无上限 | 仅在非生产环境注册，延迟封顶 `maxTestDelayMs=5000` | `api/routes.go` |

### 并发 / 资源泄漏
| # | 问题 | 修复 | 文件 |
|---|------|------|------|
| 5 | 注册接口 TOCTOU 竞态 | 检查与写入在**同一把锁**内完成 | `services/auth_service.go` |
| 6 | 限流器无界内存增长 | `SlidingWindowLimiter` 增加 `cleanupLoop` 周期性淘汰过期 IP，含上限保护 | `pkg/middleware/rate_limit.go` |
| 7 | 全局熔断放大为全站 503 | 熔断器改为**按路由隔离**（`c.FullPath()` 为 key），锁外执行 `c.Next()` 避免串行化 | `pkg/middleware/circuit_breaker.go` |
| 8 | GPU 部分分配成功泄漏 | `SubmitJob` 与 `allocateGPUs` 均改为「先收集候选→数量不足直接返回错误→确认充足再统一扣减」 | `services/k8s_service.go` |
| 9 | 索引腐坏 | `CancelJob`/`SubmitJob` 经 `UpdateJobStatus` 改状态，保持 `JobsByStatus` 索引一致 | `services/job_service.go` |
| 10 | `GetJob` 返回内部指针致数据竞争 | 返回作业**副本**而非内部指针 | `services/job_service.go` |

### 可观测性 / 配置
| # | 问题 | 修复 | 文件 |
|---|------|------|------|
| 11 | `/health` 恒返回 healthy | 注册依赖探针（`RegisterHealthCheck`），DB 不可用时返回 503 | `services/metrics.go`、`main.go` |
| 12 | `job_failed_count` 指标恒为 0 | `GetMetrics` 漏统计 `failed` 状态，补计数并在 jobs map 导出 `failed` 键 | `services/monitoring_service.go` |
| 13 | `alerts.yml` 引用不存在指标 | `gpu_memory_available` → `gpu_memory_available_bytes`（阈值 10GiB） | `alerts.yml` |
| 14 | 请求体大小限制形同虚设 | `MaxBytesReader` 置于安全过滤**之前** | `main.go` |
| 15 | 启动日志打印默认管理员口令 | 移除口令打印 | `main.go` |

### 测试
| # | 问题 | 修复 |
|---|------|------|
| 16 | `priority_concurrency_test.go` 用 `NewMemoryStore()`（双返回值）赋单变量，测试包无法编译 | 改 `MustNewMemoryStore()` |
| 17 | 测试脚手架数据竞争（`notificationCount` 跨协程无同步） | 改为 `atomic.Int64`（`priority_concurrency_test.go`、`scheduler_priority_test.go`） |

---

## 二、仍需你侧处理的高优先级事项（非代码可独立完成）

1. **轮换凭据 + 清理 git 历史**（P0）
   - 范围：`.env`、`.env.production`、`docker/.env.production` 中的数据库 / Redis / JWT 等口令。
   - 原因：已 `git rm --cached` 取消追踪，但内容仍在本地提交历史 `386d48c` 中。无远程仓库，清理仅影响本地。
   - 建议：`git filter-repo --path .env.production --invert-paths`（或 BFG）清理历史；随后对所有生产口令**强制轮换**。
   - 部署侧：凭据改由环境变量 / Secret Manager 注入，勿再提交。

2. **监控指标真实性**（中）
   - `cpu_usage_percent` / `memory_usage_percent` / `network` / `storage` 当前为**硬编码模拟值**（设计上该后端为内存仿真）。
   - 影响：`HighCPUUsage` / `HighMemoryUsage` 告警不反映真实负载。
   - 建议：生产环境接入 `node_exporter` + `prometheus`，后端仅暴露进程级真实指标（Go runtime memory 已可低成本接入）。

3. **磁盘上的临时令牌文件**（中）
   - `D:\YCYD\token.tmp` / `backend_token.tmp` 未被任何 git 仓库追踪，但含令牌，建议删除或移入受限目录。

---

## 三、验证清单
- [x] `go build ./...` 通过
- [x] `go vet ./...` 通过（含测试编译）
- [x] `go test -race -count=1 ./...` 全绿（`pkg/priorityscheduler`、`services`、`tests`）
- [x] 熔断按路由隔离、GPU 分配无泄漏、租户隔离生效（由单测与竞态检测覆盖）
- [x] 生产凭据已取消 git 追踪（工作区保留）

## 四、关键改动文件（本轮增量）
`services/auth_service.go`、`services/job_service.go`、`services/k8s_service.go`、`services/monitoring_service.go`、`services/metrics.go`、`pkg/middleware/circuit_breaker.go`、`pkg/middleware/rate_limit.go`、`api/routes.go`、`main.go`、`config/config.go`、`alerts.yml`、`tests/*_test.go`、`services/*_test.go`
