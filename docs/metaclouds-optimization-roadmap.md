# Metaclouds 算力调度系统 · 对标复盘与优化路线图

- 日期：2026-10-30
- 方法：联网对标全球先进算力调度系统（K8s 调度生态 / Slurm / Google Borg·Omega / Mesos·YARN / Run:ai / CoreWeave SUNK），交叉复盘 Metaclouds 现有代码库，产出差距分析与 P0/P1/P2 优化路线图。
- 范围：后端 `metaclouds-backend-rust`（Rust axum + SQLx，SQLite/Postgres 双驱动），前端 `metaclouds-frontend-vue`（Vue3 + Element Plus）。

---

## 一、核心结论

Metaclouds 当前处于"**有数据模型、有配额/GPU 分配原语，但无真实调度闭环**"的状态：

- `jobs` 表已建 30+ 字段（含 `topology_affinity`/`elastic`/`checkpoint`/`network_requirement` 等高级调度字段），但**全部处于"只落库、不被消费"的沉睡状态**。
- `services/scheduler/priority_scheduler.rs`（645 行）是与真实 `jobs`/`GPU` **完全脱钩的进程内 mock**（自带 mock executor）。
- `job_service.submit` 只是"写库 + 前端模拟成功"，并未真正入队调度。
- K8s 全部走 `MockK8sClient`；集群 HA 仅占位字段。
- 配额 `quota.rs` 有原子条件 UPDATE（仅 GPU），但**未接入作业创建/GPU 分配路径**；cpu/mem/storage 存在超卖窗口。

**调度系统尚未真正"调度"任何作业——这是所有优化的地基断点。**

---

## 二、对标差距分析

| 业界标杆能力 | 来源 | Metaclouds 现状 | 差距 |
|---|---|---|---|
| 真实调度闭环（submit→配额→分配→回写） | 全部系统必备 | jobs 只落库，调度器孤岛 mock | ❌ 地基缺失 |
| 配额原子校验闭环 | Kueue | 原子 UPDATE 未接入作业/GPU 路径；cpu/mem/storage 可超卖 | ⚠️ 半成品未闭环 |
| GPU 细粒度分配 | Run:ai | 事务化 fraction 分配 + best-fit | ✅ 较好，缺配额/拓扑联动 |
| Backfill 回填 | Slurm | 无 | ❌ 利用率提升最直接 |
| 多因子加权优先级 + 半衰期衰减 | Slurm multifactor | 仅 4 级枚举 + FIFO | ⚠️ 单一优先级 |
| 弹性配额借用/归还 | Kueue Cohort | resource_quotas 硬上限 | ❌ 空闲算力不流转 |
| Gang 调度（all-or-nothing） | Volcano/Kueue | 无 | ❌ 分布式训练刚需 |
| 拓扑感知调度 | Volcano/Slurm/Kueue TAS | topology 仅 CRUD+score，分配器不消费 | ❌ 字段沉睡 |
| 抢占体系（REQUEUE/豁免窗口） | Slurm/Kueue | 无 | ❌ |
| 申请/实耗双账本 + 超卖回收 | Borg | 利用率靠手工上报 | ❌ |
| 作业依赖与重试生命周期 | Slurm | 无 afterok DAG/重排队 | ❌ |
| 计费/showback + goodput | slurmdbd/Run:ai | Prometheus 有，无计费 | ⚠️ 无计费 |

---

## 三、业界可借鉴能力（按价值排序，供实现引用）

1. **Backfill 回填**（Slurm）：为排队作业计算预计开始时间+资源预约，短作业插空——利用率提升最直接。
2. **弹性配额借用/归还**（Kueue Cohort）：nominalQuota + borrowingLimit/lendingLimit，空闲算力自动流转。
3. **多因子加权优先级 + 用量半衰期衰减**（Slurm multifactor）：Age/Fairshare/QOS/JobSize 加权。
4. **抢占体系**（Slurm+Kueue）：QOS 间抢占、作业级 REQUEUE、PreemptExemptTime/min-runtime 豁免。
5. **Gang 调度**（Volcano/Kueue）：多任务作业 all-or-nothing 准入，防部分占位死锁。
6. **GPU 动态切分**（Run:ai）：显存比例 + 时间片双维切分，Strict/Fair 两模式。
7. **拓扑感知**（Volcano HyperNode/Slurm topology/Kueue TAS）：NUMA/NVLink/交换机层级放置。
8. **层级配额树 + FairTree/DRF 公平**（Slurm/Yunikorn/YARN）。
9. **申请/实耗双账本 + 超卖回收**（Borg）：按 DCGM 实耗周期回收闲置配额转售。
10. **作业依赖与重试**（Slurm --dependency / requeue）。
11. **可观测性与计费**（slurmdbd + DCGM + goodput 北极星指标）。
12. **事后重均衡**（descheduler）。

---

## 四、优化路线图（P0 / P1 / P2）

### P0 — 打通调度闭环（地基，最高价值，先行）
- **P0-1 真实调度循环**：`priority_scheduler` 从 mock 改造为消费真实 `pending` 作业 → 配额校验（quota.rs 原子）→ GPU 分配（gpu.rs 事务行锁）→ 状态回写 running + started_at → 完成/取消/失败时释放 GPU + 配额。执行层先保留模拟，后续再接真实 K8s。
- **P0-2 配额闭环**：原子校验接入作业创建 + GPU 分配路径；为 cpu/mem/storage 补条件原子校验，消除超卖窗口。

### P1 — 利用率与公平（对标 Slurm/Kueue）
- P1-1 Backfill 回填调度。
- P1-2 多因子加权优先级 + 用量半衰期衰减。
- P1-3 弹性配额借用（nominalQuota + borrow/lend）。

### P2 — 高级调度（对标 Volcano/Run:ai/Borg）
- P2-1 Gang 调度（all-or-nothing 准入）。
- P2-2 拓扑感知（消费 `jobs.topology_affinity`）。
- P2-3 抢占体系（REQUEUE + 豁免窗口）。
- P2-4 GPU 时间片切分 + 申请/实耗双账本超卖回收。
- P2-5 计费/showback + DCGM 利用率采集 + goodput 指标。

---

## 五、实施约束

- 最小变更原则，不破坏前端 API 契约（routes/响应结构）。
- 每个阶段：工程师实现 → 自测（cargo build/test/clippy）→ QA 独立回归验证。
- 提交用约定式提交；schema 变更需新 migration。

---

## 六、实施进度（增量交付 + 逐段验证）

> 原则：每阶段独立实现、独立验证，不破坏现有功能。

### ✅ P0 — 调度闭环（已落地、验证通过并 commit `ca6c32c`）

> 57 lib tests 全绿，零 schema 变更、前端 API 契约不变、保留既有 mock 模块。
- **P0-2 配额闭环（超卖窗口修复）** — `src/services/quota.rs`
  - `allocate_resources` 的原子条件 UPDATE 由"仅 GPU"扩展为 **gpu/cpu/memory/storage 四类资源全部条件校验**，消除 cpu/mem/storage 的并发超卖窗口（SQLite 串行写 + PG 行级条件更新兜底）。
  - 新增 `find_quota_for_scope(pool, tenant_id, partition_id)`：优先精确分区配额、回退租户级配额；无 active 配额视为无限（对齐 `check_quota_by_scope` 语义）。
  - 新增单测 `test_allocate_blocks_cpu_oversell` / `test_allocate_blocks_memory_and_storage_oversell` / `test_allocate_blocks_gpu_oversell` / `test_find_quota_for_scope_prefers_partition`。
- **P0-1 真实调度循环** — 新增 `src/services/dispatch.rs`（全新模块，与现有 `priority_scheduler` 孤岛 mock 解耦，是生产唯一启动的调度闭环）
  - `start_dispatch_loop` 在 `main.rs` 启动（受 `DISPATCH_ENABLED` 控，默认开）；优雅关闭时 `abort`。
  - 单轮 `dispatch_tick`：扫描 `pending` 作业→**原子认领** `UPDATE ... WHERE status='pending'`→配额预留→GPU 事务分配→置 `running`→挂接模拟执行任务（默认 2s）→释放 GPU+配额→置 `completed`。
  - 资源不足（配额/GPU）即回退 `pending` 下一轮重试（背压，不丢作业）；并发上限 `DISPATCH_MAX_CONCURRENT` 控制同时 running 数。
  - 崩溃恢复 `reconcile`：启动期扫描 `running` 孤儿（无 active GPU 分配且超 `DISPATCH_RECONCILE_GRACE_SECONDS`）回退 pending；仍持分配者重挂接完成逻辑，避免重复分配。
  - 配置项：`DISPATCH_ENABLED` / `DISPATCH_INTERVAL_MS`(2000) / `DISPATCH_SIMULATED_RUN_MS`(2000) / `DISPATCH_BATCH_SIZE`(50) / `DISPATCH_MAX_CONCURRENT`(10) / `DISPATCH_RECONCILE_GRACE_SECONDS`(300)。
  - 新增单测 `test_dispatch_pending_job_completes_and_releases`（端到端：派发→completed→GPU/配额释放）与 `test_dispatch_without_quota_or_gpu_still_runs_cpu_only`。

### ✅ P1 — 利用率与公平（已完成，2026-10-30）

- **P1-1 Backfill 回填调度** — `src/services/dispatch.rs` `dispatch_tick`
  - 两阶段派发：优先级主路径（`budget` 个按评分/优先级排序尝试）＋ Backfill 阶段（剩余候选按**规模升序**排序，在不阻塞高优先级作业前提下利用空闲并发槽与资源余量派发小作业）。
  - 指标 `metaclouds_scheduler_backfilled_total`；开关 `DISPATCH_BACKFILL_ENABLED` + `DISPATCH_BACKFILL_WINDOW` 默认开启。
- **P1-2 多因子加权优先级 + 半衰期衰减** — `dispatch_scores`
  - `score = w_priority·priority + age_bonus + fairshare_bonus − w_size·size`：
    - `age_bonus = w_age·(1 − 0.5^(age_hours / half_life))`（等待越久边际加成越小，防无限积压/饿死）；
    - `fairshare_bonus = w_fairshare·(1 − tenant_usage_ratio)`（越空闲的租户越优先）；
    - `size_penalty = w_size·size`（大作业降分，利于小作业回填装箱）。
  - 开关 `DISPATCH_SCORING_ENABLED` + 各权重/半衰期配置项（默认开启）。
- **P1-3 弹性配额借用（nominalQuota + borrow/lend）** — `src/services/quota.rs` `allocate_resources_with_borrow` + `migrations/011_*`
  - `resource_quotas` 新增 `borrow_enabled` + `borrow_limit_gpu/cpu/memory_gb/storage_gb`（双驱动迁移 011）。
  - 当租户自身限额不足且开启借用时，允许从**集群空闲容量**（`Σ(limit−used)`）借用，约束：① 单租户预计借用量 ≤ `borrow_limit_*`；② 请求量 ≤ 集群空闲余量（防集群级超卖）。
  - 事务内完成；Postgres 用 `FOR UPDATE` 锁 cohort 全部 active 配额行使余量权威，SQLite 串行写天然安全；指标 `metaclouds_quota_borrowed_total`。
  - HTTP CRUD 通过 `CreateQuotaRequest`/`UpdateQuotaRequest` 的**可选** borrow 字段暴露（向后兼容）。

- **验证记录（2026-10-30）**：全量 `cargo test --lib` **61 passed / 0 failed**（新增 quota 借用 3 例 + backfill 1 例），`cargo clippy -- -D warnings` 零告警。
  - 调试发现：测试若传 `priority: 5` 会被 `create_job` 夹紧归 0（对齐 Go 的 [0,3] 越界归 0 行为），导致评分排序与预期相反——backfill 测试已改用合法优先级 3。
  - 顺手清理预存告警：`TimeoutLayer::new` → `with_status_code(StatusCode::REQUEST_TIMEOUT, 30s)`（行为不变）。

### ⏳ P2 — 高级调度（待排期）
- P2-1 Gang 调度（all-or-nothing 准入）。
- P2-2 拓扑感知（消费 `jobs.topology_affinity`）。
- P2-3 抢占体系（REQUEUE + 豁免窗口）。
- P2-4 GPU 时间片切分 + 申请/实耗双账本超卖回收。
- P2-5 计费/showback + DCGM 利用率采集 + goodput 指标。

### 关键决策与约束
- 既有 `priority_scheduler.rs`（645 行 mock）**保持不动**，新闭环独立成模块，确保零回归、前端 API 契约不受影响。
- 执行体保留模拟（K8s 真实提交留待后续接入），调度**状态机与资源预留已真实闭环**——这是 P0 与"只落库不调度"的本质区别。
- 调度闭环默认开启，可通过 `DISPATCH_ENABLED=false` 关闭（如纯静态展示环境）。

