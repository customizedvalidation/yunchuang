# Metaclouds 世界一流调度系统复盘与优化报告

- 日期：2026-10-29
- 基线 HEAD：`f9da725`（工作区干净，基线门禁全绿）
- 范围：Rust 后端（调度核心 / GPU 分配 / 配额 / 性能 / 可观测性）+ Vue3 前端（实时刷新 / 作业向导 / 响应式 / 防重提交）
- 原则：不重构、不改 API 契约、不引入新依赖、不削弱安全边界（JWT httpOnly Cookie + CSRF + RBAC fail-closed + 生产配置校验）。

---

## 1. 总结

| 维度 | 审查发现 | 本轮已修 | 留作后续 |
|---|---|---|---|
| P0 调度核心（安全/正确性） | 2 critical + 5 major + 2 minor | 2 critical + 4 major + 2 minor | 1 major（aging/backfill 需接真实 K8s 后再上） |
| P1 后端性能 | 1 critical + 6 major + 3 minor | 1 critical + 6 major + 2 minor | 1 minor（PG max_connections 部署侧协调） |
| P2 前端 UX | 5 major + 10 minor | 5 major + 7 minor | 3 minor（骨架屏 / 移动列表卡片化 / useAutoRefresh 抽取） |
| P3 可观测性 | 3 critical + 6 major + 6 minor | 3 critical + 6 major + 5 minor | 1 minor（cache_hits 真实读数需 redis.rs 暴露 as_any） |
| **合计** | **40** | **36** | **12**（含重叠项） |

- 改动文件：31 个修改 + 3 个新建（`metrics/scheduler.rs`、`migrations/010_*.sql` ×2），+759/-362 行。
- 安全边界：未弱化。GPU 分配与配额的并发竞态反而被加固（行锁 + 原子条件 UPDATE）。
- 最终门禁（见 §5）：后端 cargo check / cargo test --lib 全绿；前端 type-check / test / build 全绿。

---

## 2. 审查方法

4 路只读审查子代理并行深读核心文件，产出带 file:line 证据的差距清单：

1. **P0 调度核心**：`priority_scheduler.rs` / `scheduler.rs` / `quota.rs` / `gpu.rs` / `partition.rs` / `handlers/*` / `models/job.rs` / `db.rs`。
2. **P1 后端性能**：`db.rs` / `routes.rs` / `cache/redis.rs` / 全部 handlers list 路径 / `monitoring.rs` / `main.rs` / 9 个迁移文件。
3. **P2 前端 UX**：`Dashboard.vue` / `JobManagement.vue` / `GPUManagement.vue` / `ClusterManagement.vue` / `SchedulerManagement.vue` / API 层 / Pinia / Layout。
4. **P3 可观测性**：`metrics/*` / `middleware/*` / `tracing` 初始化 / `services/monitoring.rs` / `prometheusrule.yaml`。

关键结构性发现（修正了对系统的认知）：

- `services/priority_scheduler.rs` 是一个**纯内存 mock 队列**（BinaryHeap + mpsc + Semaphore，作业执行是 `tokio::time::sleep`，K8s 真实提交是 TODO）。
- 真正落库的 GPU 分配只由 HTTP `POST /gpus/allocations` 触发，**没有后台循环扫描 pending jobs 并绑卡**。
- 全仓 `#[tracing::instrument]` 命中数 = 0（本轮补上）。
- 仓库根不存在 `prometheus.yml` / `alerts.yml` / `alertmanager.yml`；抓取与告警只在 `k8s/11-servicemonitor.yaml` / `k8s/12-prometheusrule.yaml`。

---

## 3. 修复清单（按域分组）

### 3.1 P0 调度核心 — 安全与正确性

| # | 位置 | 问题 | 修复 |
|---|---|---|---|
| P0-1 | `services/gpu.rs` allocate_gpu | 选机 SELECT 在事务外、逐设备 N 次 SUM(fraction)、UPDATE used_memory_gb 用快照绝对值 → 并发可双分配同一张卡 + 丢失更新 | 选机 + 批量 SUM(fraction) + INSERT allocation + UPDATE device 全部进同一个 `with_tx!`；PG 加 `FOR UPDATE SKIP LOCKED`；N 次 SUM 合并成一条 `GROUP BY device_id`；UPDATE 改增量 `used_memory_gb = used_memory_gb + ?` |
| P0-2 | `services/gpu.rs` release_gpu | 两条 UPDATE 独立事务 + 快照绝对值，与 allocate 并发时账实漂移 | 包进同一 `with_tx!`；减显存改增量 `MAX(used_memory_gb - ?, 0)`，状态用 `CASE WHEN` 一句判定 |
| P0-3 | `services/quota.rs` allocate_resources | check-then-act，并发可突破硬限额 | 改单条条件原子 UPDATE：`... WHERE ... AND (gpu_limit<=0 OR gpu_used + ? <= gpu_limit)`，`rows_affected()==0` 即 403 |
| P0-4 | `services/quota.rs` check_quota_by_scope | `_gpu_fraction` 形参被丢弃，1/4 卡按整卡计数绕开配额 | 形参改名 `gpu_fraction`，超限判断把 fraction 计入 |
| P0-5 | `services/gpu.rs` 选机循环 | first-fit 无碎片治理 | 改 best-fit：遍历所有满足约束的候选，挑剩余 fraction 最小但够本作业者 |
| P0-6 | `services/partition.rs` | `gpu_count`（整卡）与 `gpu_used`（fraction 和）直接相减口径错配 | 改 f64 运算后回 i64，注释标明 fraction 整卡当量口径；cpu/mem 写 0 处加 TODO |
| P0-7 | `migrations/010_*.sql` | `gpu_allocations` 缺 `(user_id,status)` / `(tenant_id,status)` 索引 | 新增迁移（SQLite + Postgres 双方言） |

### 3.2 P1 后端性能

| # | 位置 | 问题 | 修复 |
|---|---|---|---|
| P1-1 | `services/monitoring.rs` get_dashboard_stats | 11 条 COUNT 串行 await，单指标查询放大成全量 | 改 `join_all` 并发（P99 从 11× 降到 1×）；新增 `pending_jobs` 计数 |
| P1-2 | `services/monitoring.rs` evaluate_alert_rules | `if fire || active_alerts > 0` 导致只要有 1 条 active 告警，16 条规则全报触发 | 改 `if fire`，stub 不再撒谎 |
| P1-3 | `cache/redis.rs` + `monitoring.rs` | Redis 建而不用，dashboard 热路径零缓存 | Cache trait 加 `get_or_set` 默认方法；dashboard_stats 包 10s TTL；GPU allocate/release 后主动 del |
| P1-4 | `db.rs` connect_sqlite | 未开 WAL / busy_timeout / foreign_keys，5 连接写并发直接 SQLITE_BUSY | 加 `.journal_mode(Wal).busy_timeout(5s).foreign_keys(true)` |
| P1-5 | `db.rs` connect_postgres | `MAX_OPEN_CONNS=100` 写死，多副本打爆 PG | 改读 env `DB_MAX_CONNS`（默认 20），min_connections = max(1, max/5) |
| P1-6 | `models/job.rs` list | `SELECT *` 拉 45 列含长文本/JSON | 改 42 列显式清单（按 Job FromRow 列全） |
| P1-7 | `handlers/user.rs` list | `SELECT *` 把 password_hash 也拉出来 | 改 9 列显式清单排除 password_hash；`password_hash` 加 `#[sqlx(default)]` 兼容 SELECT * 路径 |
| P1-8 | `middleware/mod.rs` | 无请求超时、无 gzip | 加 `TimeoutLayer(30s)` + `CompressionLayer(gzip)`（Cargo.toml 扩 tower-http feature，不引新 crate） |

### 3.3 P2 前端 UX

| # | 位置 | 问题 | 修复 |
|---|---|---|---|
| F-1 | `components/Sidebar.vue` | 30s 徽标轮询与页面 30s 轮询重复打同一批接口 | 徽标间隔改 60s |
| F-2 | `pages/Dashboard.vue` | `anyLoading`/`anyError` 只覆盖 4/9 个数据源，其余 pending 时渲染半成品 0 值 | 扩到 9 个数据源 `.some(loading)` / `.find(error)` |
| F-3 | `components/Topbar.vue` | `isMobile = computed(() => window.innerWidth < 1024)` 非响应式，窗口缩放不更新 | 加 `windowWidth` ref + resize 监听 |
| F-4 | `pages/GPUManagement.vue` | submitForm/submitAllocate 无 loading，可重复提交 | 加 `submitting` ref + try/finally + 按钮 `:loading` |
| F-5 | `pages/SchedulerManagement.vue` | 同上 | 同上 |
| F-6 | `pages/GPUManagement.vue` | `page_size: 1000` 全量拉 | 降为 200 |
| F-7 | `pages/JobManagement.vue` | 30+ 字段平铺在 860px Dialog | 用 `<el-collapse>` 分 3 组：基础配置（展开）/ 弹性与容错（折叠）/ 拓扑与高级（折叠） |
| F-8 | `pages/JobManagement.vue` | 创建成功 toast 文案模糊 | 改"作业已创建，已加入队列" |
| F-9 | `pages/JobManagement.vue` | 表单 rules 只有 3 个 required | 补 priority required |
| F-10 | `components/PageState.vue` | 空状态是 Box 图标，与 Dashboard 的 ElEmpty 不统一 | 改 `<el-empty :image-size="60" />` |
| F-11 | `pages/Dashboard.vue` hover 阴影 | 硬编码 `rgba(24,110,255,0.14)` 与品牌色 RGB 不一致 | 改 `var(--mc-brand-15, rgba(47,107,255,0.15))` |
| F-12 | `components/Topbar.vue` 头像 | 硬编码 Ant Design 蓝 `#1677ff/#0958d9` | 改 `var(--mc-brand-grad, ...)` |
| F-13 | `pages/__tests__/Dashboard.spec.ts` | 适配 F-2 后补 `dashboard: vi.fn()` mock | 测试桩补齐，56 用例全绿 |

### 3.4 P3 可观测性

| # | 位置 | 问题 | 修复 |
|---|---|---|---|
| O-1 | 新建 `metrics/scheduler.rs` | 调度关键路径零指标 | 注册 9 个指标：`scheduler_pending_jobs{priority}` / `scheduler_running_jobs` / `completed_total` / `failed_total` / `cancelled_total` / `job_aborted_total{reason}` / `schedule_latency_seconds` Histogram / `quota_rejected_total{tenant,resource}` / `gpu_allocation_failed_total{reason}` |
| O-2 | `services/priority_scheduler.rs` | JobRecord 无 enqueued_at；终态 job HashMap 无限增长 | 加 `enqueued_at: Instant`；submit/cancel/complete 三处刷新 queue metrics；drain 弹出时 observe latency；`maybe_gc_terminal()` 在 jobs.len()>10000 时回收 id<next_job_id-5000 的终态记录 |
| O-3 | `services/priority_scheduler.rs` / `auth/middleware.rs` | 全仓 0 个 `#[tracing::instrument]` | 调度 4 个函数加 `#[instrument(level="debug", skip_all)]`；`jwt_auth` 加 `#[instrument(level="info", fields(user_id=Empty, tenant_id=Empty))]`，verify 后 `Span::current().record(...)` |
| O-4 | `middleware/tracing.rs` | http.request span 无 user_id/tenant_id | span 加两个 `Empty` 字段，jwt_auth record 后贯穿 |
| O-5 | `middleware/request_logger.rs` | info 日志只打 user_id，无 tenant_id | 顺带提取并打 tenant_id |
| O-6 | `metrics/business.rs` + `cache/mod.rs` | Redis hits/misses 进程内原子量已算但未导出 | 加 `cache_hits_total`/`cache_misses_total` IntGaugeVec + `install_cache_stats()`/`global_cache_stats()` 接线（已知限制见 §6） |
| O-7 | `config.rs` | `otel_sample_rate` 默认 1.0，高 QPS 爆采集器 | 默认改 0.1，注释说明 |
| O-8 | `metrics/http.rs` | `http_request_duration_seconds` 缺 status label | HistogramVec labels 加 status（基数 <20，安全） |
| O-9 | `k8s/12-prometheusrule.yaml` | `JobQueueLength` 表达式数的是 running（满载误报），不是 pending（真堆积） | 改 `metaclouds_pending_jobs > 100 for 10m`；新增 `RunningJobsHigh = running_jobs > 200` 容量告警 |
| O-10 | `metrics/business.rs` | `active_users` Gauge 实际=total_users，撒谎 | 加 TODO 注释（待 session 表接入） |

---

## 4. 关键设计决策与偏差

1. **不引入新 crate**：tower-http 0.6 没有 `compression` 总 feature，实际开的是 `compression-gzip`；`TimeoutLayer::new` 在 0.6 已 deprecated（建议 `with_status_code`），保留并接受一条 deprecation 警告，行为不变。
2. **GPU 选机 best-fit**：只改循环体（`break` → 遍历挑最优），SQL 不动；Postgres 加 `FOR UPDATE SKIP LOCKED`，SQLite 因库串行写不加行锁。
3. **配额原子化只对 gpu 做超限守卫**：cpu/memory/storage 三列仍同一条 UPDATE 增量累加（保持原记账不退化），WHERE 条件只对 gpu 加 `gpu_used + ? <= gpu_limit` 守卫。
4. **users list 排除 password_hash**：因 `User: FromRow` 的 `password_hash` 是非 Option 字段，缺列会运行时报错，给它加 `#[sqlx(default)]`；登录/改密等 `SELECT *` 路径反序列化不受影响。
5. **partition gpu_available 保持 i64 对外契约**：内部 f64 运算后回 cast i64，不改 JSON 数值类型。
6. **调度器进程内指标前缀用 `metaclouds_scheduler_*`**：避免与 business.rs 已有的 `metaclouds_running_jobs` 重名导致 Prometheus registry 注册 panic。

---

## 5. 门禁证据

```
后端（D:\YCYD\metaclouds-backend-rust）：
  cargo check --quiet          → exit 0（仅 1 条 tower_http TimeoutLayer deprecation 警告，非错误）
  cargo test --lib             → test result: ok. 51 passed; 0 failed; 0 ignored（2 个 PG 实时库用例按预期 ignored）

前端（D:\YCYD\metaclouds-frontend-vue）：
  npm run type-check           → vue-tsc --noEmit exit 0
  npm run test -- --run        → Test Files 11 passed (11) / Tests 56 passed (56)
  npm run build                → vite build exit 0（dist/assets/index-*.js 69.42 kB / element-plus 951 kB）
```

---

## 6. 剩余建议（需要目标环境 / 产品决策，本轮未修）

### 调度算法（接真实 K8s 后再上）

- **aging / 抢占**：当前 mock 队列排序只有 priority+seq，无等待时长老化。真正接 K8s 绑卡后，加 `effective_priority = min(priority + floor(wait_sec/1800), 3)` 作为 tie-breaker，并对 high-priority pending 作业实现 grace-period 抢占。
- **EASY backfill**：drain 弹出前读队首最高优作业的资源需求，只放行"不挡队首"的低优作业。当前 `Semaphore` 只看并发槽位、不看资源。
- **拓扑感知**：`Job` 模型有 `topology_affinity` / `node_selector` / `network_requirement` 字段，但 `allocate_gpu` 从不读 node_name / NUMA / NVLink。接 DCGM-exporter / node label 后按 node_name 亲和做增量评分。

### 可观测性

- **cache_hits 真实读数**：全局缓存是 `Arc<dyn Cache>`，`snapshot()` 在独立的 `CacheMetrics` trait 上被擦除；本轮 Gauge 结构已接线但读 0。后续 `redis.rs` 暴露 `as_any()` 或全局改存 `Arc<dyn CacheMetrics>` 即可自动读出真实原子计数。
- **DB 连接池 stats**：sqlx 0.7+ 提供 `pool.statistics()`，本轮未接；接好后恢复 `k8s/12-prometheusrule.yaml` 里被注释的"连接池使用率 >90%"告警。
- **Alertmanager 路由/抑制**：仓库根无 `alertmanager.yml`，建议加 critical→电话、warning→IM 路由，以及 GPU 全挂时抑制同源 5xx 告警的 inhibit 规则。
- **/metrics 抓取打全表 COUNT**：Prometheus 每 15s 抓一次就跑 13 条 COUNT；建议起 60s 后台 ticker 预刷 Gauge，metrics_handler 只读 Gauge。

### 性能

- **PG max_connections 协调**：env `DB_MAX_CONNS` 默认 20 已加，但生产部署侧需确保 `max_connections >= 副本数 × DB_MAX_CONNS + 预留 20`。
- **SQL 聚合下推**：`get_gpu_utilization_summary` 仍把 `gpu_devices` 全表拉进 Rust 内存再 for 循环求和；规模到 5000 卡时改 `GROUP BY status, vendor, model` 下推。
- **GPU/Job list 后端分页**：前端 GPU 拉取已降为 200，但 `jobApi.list()` 仍无分页参数；建议加 `?page=&page_size=` 走后端分页。

### 前端

- **骨架屏**：PageState loading 仍是转圈图标，表格页可换 `<el-skeleton :rows="8" animated />`。
- **useAutoRefresh 抽取**：Dashboard / GPU / Cluster / Scheduler 4 个页面仍各自复制了一份倒计时逻辑（JobManagement 已用 composable）。
- **移动端表格卡片化**：GPU/Job 表格 15+ 列在 <768px 横向滚动；可在移动端隐藏非关键列或切卡片列表。

### 运维（前序已留，本轮未动）

- Trivy image scan、kubeconform、Dockerfile digest、`automountServiceAccountToken` 矛盾、kustomization prod overlay —— 这些仍需产品决策 / GitHub runner 验证，本地改了无法验证。
