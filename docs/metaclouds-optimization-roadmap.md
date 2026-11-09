# Metaclouds 算力调度系统 · 对标复盘与优化路线图

- 日期：2026-10-30
- 方法：联网对标全球先进算力调度系统（K8s 调度生态 / Slurm / Google Borg·Omega / Mesos·YARN / Run:ai / CoreWeave SUNK），交叉复盘 Metaclouds 现有代码库，产出差距分析与 P0/P1/P2 优化路线图。
- 范围：后端 `metaclouds-backend-rust`（Rust axum + SQLx，SQLite/Postgres 双驱动），前端 `metaclouds-frontend-vue`（Vue3 + Element Plus）。

---

> ⚠️ **状态回标（2026-11-09）**：以下第一至四节是 2026-10-30 立项时的**初始差距分析**，其核心结论已被第六节「实施进度」推翻 —— P0 / P1 / P2-1 / P2-2 / P2-4 均已落地。当前准确状态请以第六节及 `docs/system-review-and-optimization-2026-11-09.md` 为准；本段之后的原文按历史记录保留，未作改写。
>
> ✅ **第二轮修复（2026-11-09 下午）**：全面复盘报告中的 P0-1~3、P1-1 / P1-3 / P1-4、P2-1 / P2-3 / P2-6 已按 `docs/system-fixes-2026-11-09.md` 落地 —— 生产模拟执行 fail-closed、`/system/mode` 模式接口与前端横幅、多副本租约 / 全局并发 / 优雅排水、CI 真实断言与 SHA 钉版本、撤销名单默认 fail-closed、配置矛盾收敛、死调度器 feature 门控、合成实耗 `usage_source` 标记。**部分落地**：P1-2 已钉镜像 SHA tag 但 Dockerfile 基础镜像 digest 未钉；P1-5 已标注 PG 单点并加 CI guard，但 HA 本身未做；P0-2 的执行任务 DB 持久化 / worker lease 未做。**剩余未决项**：真实执行器（需 K8s 集群/GPU/DCGM）、P2-3 抢占分支回归（832 行改动且已知 2 处 major，需系统定位拍板）、PG HA、Docker 基础镜像 digest、CI 首次完整 run（需推送 + 配置仓库变量与 secrets，详见修复报告第四节）。

## 一、核心结论（2026-10-30 初始基线 · 历史留存）

Metaclouds 立项时处于"**有数据模型、有配额/GPU 分配原语，但无真实调度闭环**"的状态：

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

### 🔧 P2 — 高级调度（对标 Volcano/Run:ai/Borg）

- **P2-1 Gang 调度（all-or-nothing 准入）** — ✅ 已完成（2026-10-30，commit `7d23ee6`，QA 独立验证 PASS）
  - `jobs` 新增 `gang_replicas/gang_min_available/gang_fulfilled`（双驱动迁移 012，全默认值零破坏）；`gang_replicas>1` 时按 min_available 副本资源量原子 all-or-nothing 预留（配额含借用 + GPU 逐卡分配整体成功或整体回退）。
  - `gang_unit()` 统一放大配额请求/GPU 张数/job_size（backfill 排序与评分惩罚）；完成时按 `gang_fulfilled.max(1)` 等量释放防配额泄漏（非 Gang 回退 1 副本，行为不变）。
  - 指标 `metaclouds_scheduler_gang_admitted_total/blocked_total`；开关 `DISPATCH_GANG_ENABLED`（默认 true，关闭按 replicas=1）。
  - 验证：63 passed / 0 failed（+2 Gang 单测），clippy -D warnings 零告警；QA 复核回退整体性、非 Gang 释放回归、PG 迁移同构、边界值（min>replicas/0/负数宽松回退）均通过。

- **P2-2 拓扑感知调度（消费 `jobs.topology_affinity`）** — ✅ 已完成（2026-10-30，commit `304a597`；QA 终验 + 轻量确认双 PASS，livelock 修复经突变验证成立）
  - **格式判别规则**：affinity trim 后，`""`/`cluster` → 无约束（既有行为）；`node`/`rack`/`switch`（大小写不敏感）→ 级别共置——该作业（× Gang unit）**全部 GPU 卡落在同一节点**；`node:<name>` 前缀或非关键字裸值 → 钉住指定节点（节点名大小写敏感，保留原值）。
  - **rack/switch 取舍**：`gpu_devices` 无 rack/switch 层级字段，rack/switch 级退化为 node 级共置——严格满足约束（单节点天然属于同一 rack/switch）、放置更紧；不满足即整体回退 pending 背压，绝不违反亲和。真层级感知需设备表加 rack/switch 列 + migration（后续项）。
  - 放置实现（预选节点，防 livelock）：分配任何卡之前先按节点分组统计满足单卡需求（vendor/显存/fraction）的空闲卡数，best-fit 选一个"空闲卡数 ≥ total_cards"的节点，再把全部卡钉在该节点逐卡分配；无满足节点 → blocked 指标 + pending 背压（正确背压，非确定性反复回退）。任一卡失败整体回退（all-or-nothing，与 Gang 语义一致）。
  - Gang 协同：min_available × gpus 张卡全部满足同节点/钉节点约束才整体准入，任一失败整体回退。
  - `AllocateGpuInput` 新增可选 `node_name`（空 = 不限节点，既有调用方零影响），best-fit 选机按节点过滤；API `CreateJobRequest`/`UpdateJobRequest` 新增**可选** `topology_affinity`（serde default 向后兼容），沉睡字段从此可经 API 设置并被调度器消费。
  - 指标 `metaclouds_scheduler_topology_matched_total`（约束满足放置）/ `topology_blocked_total`（约束不可满足回退）；开关 `DISPATCH_TOPOLOGY_ENABLED`（默认 true）。
  - 验证：**68 passed / 0 failed**（+5：级别共置卡数不足→pending→补卡成功、livelock 回归 `test_topology_colocate_preselects_node_with_enough_cards`（低 id 节点卡数不足时预选直接落卡数足够节点；QA 突变验证：去预选/复刻旧算法均使该测试变红）、显式钉扎 node:不存在→pending/裸名→成功、前缀形式大小写不敏感 `test_topology_pinned_prefix_case_insensitive`（"Node:node-1"→钉 node-1）、Gang×拓扑组合共置），clippy -D warnings 零告警；零 schema 变更。
  - 测试基建顺带修复：dispatch 内存库测试池改单连接（`:memory:` 多连接各私有，偶发 no such table）。
- P2-3 抢占体系（REQUEUE + 豁免窗口）——**完整实现已保全于 `park/p2-3-preemption` 分支**（原 d3ea543，含 run token 防护/豁免窗口/4 测试，QA 深验通过）；重新立项时必查：victim 未按 cluster/partition 过滤的 major + `complete_running_job` run_token 契约回归。
- **P2-4 GPU 时间片切分 + 申请/实耗双账本超卖回收** — ✅ 已完成（2026-10-30，commit `6cef438`；收口缺陷修复见 `fix(gpu)` 提交）
  - **时间片共享**：fraction 容量上限由 `DISPATCH_GPU_TIMESLICE_MAX` 控制（默认 `1.0` = 不超卖、与既有行为完全一致；配置 <1.0 一律按 1.0），`allocate_gpu(..., timeslice_max)` 的 best-fit 用 `slice_cap − Σactive_fraction` 判断；fraction<1 的成功分配计入 `metaclouds_gpu_timeslice_allocations_total`，分配/释放后按「active 份额 ≥ 2」刷新 `metaclouds_gpu_shared_devices` gauge。
  - **双账本**：`jobs` 追加 `gpu_fraction_actual / cpu_actual / memory_gb_actual / usage_sampled_at`（双驱动迁移 013，全部可空、零破坏）；调度采样按 `DISPATCH_SIMULATED_UTILIZATION`（默认 `0.75`）写入实耗（模拟口径，接入 DCGM 后替换数据源）。
  - **超卖回收准入**：`quota::allocate_resources_with_borrow` 新增第 3 档——自身限额与集群借用都不够时，以本租户 running **已采样**作业的 `Σmax(0, 申请 − 实耗)` 作为可回收量准入（先回收自己的「申请未实耗」再借集群，更公平）；未采样作业保守视为占满申请量、不计回收，storage 无实耗不参与；GPU 缺口按卡等效需求（申请卡数 × fraction）同口径比较，申请账本仍按卡数累加。准入路径埋点 `quota_reclaimed_admission_total`，可回收量 gauge `quota_reclaimable_gpu`。
  - **收口缺陷（P0 级，自 `4f77937` 起存在）**：`release_gpu` 的设备扣减 SQL 只有 `?1/?2/?3` 三个编号占位符（`?1` 复用两次），却绑定了 4 个参数。SQLite 按位绑定错位后 `updated_at` 收到显存值、`WHERE id` 收到时间戳（RFC3339 与整数永不相等）→ 语句静默 0 行：**设备显存只增不减、状态永久卡在 `allocated`**；Postgres 则直接 Bind 报错。已删去多余绑定，并加注释钉住「绑定必须与占位符一一对应」。
  - 验证：`cargo test --lib` **72 passed / 0 failed**；全量 `cargo test` 全绿；`cargo clippy --all-targets -- -D warnings` 零告警；`cargo fmt --all -- --check` 干净。反向验证（突变测试）：恢复旧绑定后 `b3_gpu_allocate_updates_device_status_then_release` 立即变红（设备停留 `allocated`），证明修复有区分力。
  - 测试稳定性：`shared_gpu_devices` 是进程级 gauge，并行单测各自独立内存库会在释放路径互相把 gauge 刷成 0 → 断言前用自身 pool 主动刷新（`refresh_shared_devices_gauge` 提升为 `pub(crate)`）并带短重试，避免偶发红。
- P2-5 计费/showback + DCGM 利用率采集 + goodput 指标。

### P2 backlog（QA 复核发现的非阻塞观察项）

> **状态回标（2026-11-09 第二轮修复）**：下列 3 项已全部闭环，详见 `docs/system-fixes-2026-11-09.md`。

1. ~~`reconcile` 孤儿 running 回退不释放已预留 CPU/内存配额~~ → ✅ **本轮核验确认已由既有代码覆盖**：`dispatch.rs::reconcile` 回退前调用 `release_quota_for_job`，其中 `QuotaRequest` 同时含 `gpu / cpu / memory_gb`，按 `release_unit`（Gang 副本数）等量释放。此前描述为 10-30 摘除抢占分支时的中间态，已不成立。
2. ~~CoLocate 保守计数（每设备至多计 1 卡）→ fraction<1 需同卡多份额共置的作业可能被误 blocked~~ → ✅ **本轮修复**：`select_topology_node` 在 `fraction<1 && slice_cap>1` 时按设备剩余容量可容纳的份额槽位数计数；默认 `slice_cap=1.0` 时仍为 1，行为零回归。新增回归用例 `test_colocate_counts_fraction_slots_on_single_device`（正反两向断言）。
3. ~~`complete_running_job` 未检查 rows_affected 即计 completed_total~~ → ✅ **本轮修复**：前置 `job.status == "running"` 检查 + 末尾 `rows_affected == 0` 双重幂等，重复完成不再重复计数/释放；配套 reconcile 数据库租约（迁移 014）消除多副本互踩。

### CI 门禁（2026-10-30 核查与修复）

- **事实**：GitHub Actions 最近 8 次 push 全部 `failure`，首个失败 job 均为 `Security Pre-Scan`（Checkov IaC），其余 13 个 job 全部 `skipped`——Rust Lint & Test、前端构建、Docker 构建**从未真正执行**，因此 P2 期间积累的 `release_gpu` 绑定缺陷与 `cargo fmt` 漂移长期未被拦截。
- **Checkov 失败面**：仅 2 个清单。`17-postgresql.yaml` 缺 CKV_K8S_15（拉取策略非 Always）；`18-redis.yaml` 的 `gen-redis-conf` initContainer 缺全部资源与安全上下文（CKV_K8S_10/11/12/13 资源、20 越权提升、22 只读根、28/30/37 能力与 seccomp、15 拉取策略）。
- **修复**：两份清单镜像拉取策略统一 `Always`（与仓库其余清单一致，tag 可被上游重推、Always 每次校验 digest）；initContainer 补 `requests/limits`（50m/64Mi → 200m/128Mi）、`allowPrivilegeEscalation: false`、`readOnlyRootFilesystem: true`、`capabilities.drop: [ALL]`（seccomp 由 Pod 级 `RuntimeDefault` 覆盖）。本地按 Checkov 同名规则做等价静态校验通过。
- **遗留核验点**：门禁修好后需观察首个真正跑通的 Rust job，确认 1.96.0 rustfmt/clippy/测试在 CI 环境同样绿。

### 关键决策与约束
- 既有 `priority_scheduler.rs`（645 行 mock）**保持不动**，新闭环独立成模块，确保零回归、前端 API 契约不受影响。
- 执行体保留模拟（K8s 真实提交留待后续接入），调度**状态机与资源预留已真实闭环**——这是 P0 与"只落库不调度"的本质区别。
- 调度闭环默认开启，可通过 `DISPATCH_ENABLED=false` 关闭（如纯静态展示环境）。

