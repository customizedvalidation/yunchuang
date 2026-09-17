# P3-02 cron 表达式迁移对照表（Go robfig/cron → Rust tokio-cron-scheduler）

> 枚举自 Go 版 `metaclouds-backend` 源码（v3.0.1 `github.com/robfig/cron/v3`）。
> 以代码实际注册为准，未臆造任何任务。

## 1. Go 侧事实

### 1.1 调度器构造

`services/scheduler.go:45`：

```go
cron.New(cron.WithLogger(cron.DefaultLogger))
```

- **Parser**：robfig/cron/v3 默认 parser —— **5 字段**（分 时 日 月 周），
  未启用 `cron.WithSeconds`，因此表达式里没有"秒"位。
- **时区**：**未调用** `cron.WithLocation(...)`，使用进程**本地时区**（`time.Local`）。
- **入口**：`main.go:196-203`：

```go
scheduler := services.NewScheduler(jobService, k8sService)
if cfg.Environment != "production" {
    scheduler.AddDefaultSchedules()   // 仅非生产环境注册示例任务
}
go scheduler.Start()
defer scheduler.Stop()
```

### 1.2 实际注册的全部 cron 任务

`services/scheduler.go:315-318` `AddDefaultSchedules()`：

| 任务名            | Go 表达式       | 语义（robfig 5 字段）        | 任务体                          | 注册条件                |
| ----------------- | --------------- | ---------------------------- | ------------------------------- | ----------------------- |
| `sample-training` | `*/30 * * * *`  | 每 30 分钟（每小时 :00 / :30） | `executeJob(jobID=1, name)`     | `Environment != prod`   |
| `sample-inference`| `0 */2 * * *`   | 每 2 小时整点（:00 分）        | `executeJob(jobID=2, name)`     | `Environment != prod`   |

任务体（`executeJobOnce`）：查 job → pending 则改 running → `k8sService.SubmitJob` →
起 goroutine 轮询监控完成状态。Rust 侧按"骨架 + 日志 + TODO"落地，调度层语义先对齐。

> 说明：Go 侧**没有**在 `main.go` / `init_data.go` / `bootstrap_credentials.go`
> 中注册其他 `cron.AddFunc` 任务。限流清理、K8s 同步、指标采集等后台循环用的是
> `time.NewTicker`（`pkg/middleware/rate_limit.go`、`services/metrics.go`、
> `services/k8s_service.go`），不属于 cron 调度器范畴，不在本迁移范围。

## 2. 语法差异

| 维度        | Go robfig/cron/v3              | Rust tokio-cron-scheduler 0.10（`cron` crate） |
| ----------- | ------------------------------ | ----------------------------------------------- |
| 字段数      | 5 字段（分 时 日 月 周）       | 7 字段（**秒** 分 时 日 月 周 **年**）；年份位通常 `*` |
| 秒位        | 无（默认 parser 不含秒）       | 必须显式给出，对齐 Go 的"0 秒"触发              |
| 时区        | 默认 `time.Local`，可用 `WithLocation` 覆盖 | 构造 Job 时用 `Job::new_async_tz(expr, Utc, …)` 显式指定；不指定则 UTC |
| 年份位      | 不支持                         | 7 字段末位，恒为 `*`                             |

字段位对照（从左到右）：

```
Go:   分   时   日   月   周
Rust: 秒   分   时   日   月   周   年
```

## 3. Go → Rust 表达式映射

| 任务名            | Go 表达式（5 字段，本地时区） | Rust 表达式（7 字段，UTC）      | 触发时刻对齐验证                |
| ----------------- | ------------------------------ | ------------------------------- | ------------------------------- |
| `sample-training` | `*/30 * * * *`                 | `0 */30 * * * * *`              | 秒=0，分钟 ∈ {0,30}，每小时；双端同一墙钟时刻（时区见 §4） |
| `sample-inference`| `0 */2 * * *`                  | `0 0 */2 * * * *`             | 秒=0、分=0，小时为偶数；双端同一墙钟时刻（时区见 §4） |

推导过程：

- Go `*/30 * * * *` = 分每 30 → Rust 在秒位补 `0`、年位补 `*`：
  `0`（秒）`*/30`（分）`*` `*` `*` `*`（周）`*`（年）。
- Go `0 */2 * * *` = 分=0、时每 2 → Rust：`0`（秒）`0`（分）`*/2`（时）`*` `*` `*`（周）`*`（年）。

## 4. 时区差异（如实记录）

- **Go 版**：未调用 `WithLocation`，按**进程本地时区**触发（容器时区通常为 UTC，
  但本地开发机为 Asia/Shanghai）。
- **Rust 版**：为保证确定性与可测试性，调度器**统一使用 UTC**（`chrono::Utc`），
  与 tokio-cron-scheduler 默认行为一致。
- 影响：当部署环境时区为 UTC 时，两端触发时刻完全一致；当 Go 运行在非 UTC 时区时，
  同一 cron 表达式的墙钟时刻相差一个时区偏移。测试统一按 UTC 断言，偏差 < 1s。

## 5. 注册条件对齐

- Rust `Scheduler::register_all_jobs()` 复刻 Go 逻辑：仅当
  `config.environment != "production"` 时注册两个示例任务；生产环境调度器启动但为空。
- 未来若 Go 侧通过 `POST /scheduler/schedules` 动态 `AddSchedule`，Rust 侧预留
  `Scheduler::add_job(name, expr, …)` 扩展点（当前未启用，留 TODO）。

## 6. 任务隔离语义

- Go 侧：单个任务 panic 会冒泡到 robfig/cron 的 recover 中间件（robfig/cron/v3
  默认每个 job 在独立 goroutine 运行，panic 不会停调度循环）。
- Rust 侧：每个任务体用 `tokio::spawn` 隔离，外层 `JoinHandle` 探测
  `is_panic()` 并记录日志，**任务 panic 不影响调度循环与其他任务**
  （见 `tests/p3_cron_test.rs::panic_task_does_not_kill_scheduler`）。
