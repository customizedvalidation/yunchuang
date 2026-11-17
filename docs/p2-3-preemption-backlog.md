# P2-3 抢占分支回归清单（部分落地，旧分支仍未合并）

- 日期：2026-11-09（第三轮台账化）/ 2026-11-16（契约落地更新）
- 分支：`park/p2-3-preemption` @ `d3ea543`，约 832 行改动
- 状态：旧分支 **未回归、未合并**（直接 merge 会引入冲突与语义回退）；但其两个 major 缺陷已在本轮以**独立契约实现**修复，不复用旧分支代码。

## 2026-11-16 契约落地（默认关闭）

本轮没有合并旧分支，而是在 `main` 上按「与真实执行器共用同一套租约/回收语义」重新实现了抢占的**存储契约 + SQL 候选集 + 幂等回收路径**：

| 项 | 落地位置 | 状态 |
|---|---|---|
| 开关 | `DISPATCH_PREEMPTION_ENABLED`（`Config::dispatch_preemption_enabled`），默认 `false` | ✅ 默认关闭；生产未启用 |
| 存储 | `migrations/017_preemption.sql`（双驱动）：`jobs.preempt_exempt_until`、`jobs.preempted_by`、复合索引 `idx_jobs_preemption_victim` | ✅ 已落地 |
| M1 修复 | `services::dispatch::find_preemption_victims`：候选集在 **SQL WHERE** 按 `cluster_id` / `partition_id` / `tenant_id` / `priority` / 豁免窗口收窄 | ✅ 正反两向离线用例通过 |
| M2 修复 | `services::dispatch::preempt_victim`：复用 `revert_to_pending` + `release_resources_for_job` 单一回收入口，CAS 失败即跳过，**不再引入第二套 run_token** | ✅ 幂等用例通过 |
| 接入点 | `dispatch_tick`：仅在开关启用时，为派发失败的高优先级作业抢占并重排；开关关闭时行为与之前**完全一致** | ✅ 已落地 |
| 指标 | `metaclouds_scheduler_preempted_total`（仅在成功抢占时递增） | ✅ 已落地 |

**诚实边界**：以上只证明「SQL 候选集 + 排队/回收语义」在离线 SQLite 内存库上成立。它**不**证明真实 K8s 作业能被安全 grace-stop、GPU 能真正让出、跨租户抢占符合策略。在 `SimulatedExecutor` 上开启抢占仍属于演示语义；生产启用前必须在真实执行器上完成下方第七节测试。

---

## 一、已知缺陷（恢复时必须先修）

| # | 级别 | 问题 | 修复方向 |
|---|---|---|---|
| M1 | major | victim 选择未按 `cluster` / `partition` 过滤，可能抢占别的分区/集群的作业 | ✅ **本轮已修**：`find_preemption_victims` 在 SQL WHERE 收窄，并额外按 `tenant_id` 过滤；「跨集群/跨分区/跨租户/非更低优先级/豁免窗口内均不得成为 victim」由 `test_preemption_victim_scope_and_exemption` 正反两向断言覆盖 |
| M2 | major | `run_token` 契约与 `complete_running_job` 不一致 | ✅ **本轮已修**：新实现彻底不引入 run_token；抢占复用 `revert_to_pending` + `release_resources_for_job`，与完成/失败/reconcile 共用幂等回收口；`test_preemption_requeues_and_is_idempotent` 覆盖 |

---

## 二、回归前必须处理的基线漂移

第二轮（2026-11-09）改动与分支高度重叠，rebase 前逐项核对：

- `src/services/dispatch.rs`：`acquire_reconcile_lease`（迁移 014）、`try_dispatch_one` 并发上限改原子 UPDATE、`DispatchLoop::shutdown` 排水 30s、`complete_running_job` 幂等、`sample_job_usage` 写 `usage_source='synthetic'`、CoLocate 份额槽位计数。
- `src/main.rs`：关闭路径由 `abort()` 改为 `d.shutdown(30s).await`。
- `src/config.rs`：`Config::validate()` 生产 fail-closed、`ALLOW_SIMULATED_EXECUTION`、`K8S_SIMULATION_MODE` 真实生效。
- 迁移 `014_dispatch_lease.sql` / `015_usage_source.sql`（双驱动各一份）。
- `legacy-scheduler` feature 门控；`tests/priority_scheduler_test.rs` 加 `#![cfg(feature = "legacy-scheduler")]`。
- `Cargo.toml` version → `2.0.1`。
- `docs/real-executor-contract.md` 新增的 `exec_tasks` / lease 设计 —— 若抢占要长期保留，建议直接在同一套 lease 语义上实现，避免第三套并发控制。

---

## 三、执行顺序

```bash
git checkout park/p2-3-preemption
git rebase main                       # 预期在 dispatch.rs 冲突，逐段人工合并
cd metaclouds-backend-rust
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet                    # 含第二轮 5 个配置门禁用例 + 72 个 lib 用例
cargo test --features legacy-scheduler --quiet
```

冲突合并原则：**保留第二轮的多副本语义**（原子并发上限、租约、幂等、排水），在其之上叠加抢占逻辑；不得用分支的旧 `dispatch.rs` 整文件覆盖。

---

## 四、必须补齐的测试

1. M1：同分区低优先级作业可被抢占；**跨 `partition_id` / `cluster_id` 的作业不得进入 victim 候选**（正反两向断言）。
2. M2：`complete_running_job` 在抢占重排队后被调用，不重复计数、不重复释放资源；`run_token` 失效路径下完成回调被丢弃。
3. 豁免窗口：`preempt_exempt_until` 内不被选为 victim，窗口过期后可被选中。
4. 与 Gang 组合：`gang_replicas>1` 的作业被抢占时按 `gang_fulfilled` 等量释放配额/GPU（沿用 P2-1 语义）。
5. 与拓扑组合：被抢占作业的 GPU 卡回到 `available`，设备显存扣减正确（回归 `fb4a169` 修过的绑定错位）。
6. 多副本：3 个 dispatch loop 同时跑抢占，不出现两个副本抢同一 victim（复用 014 租约）。
7. 与 backend 组合：`K8S_SIMULATION_MODE=false` 时抢占相关读端点不被 mock 数据污染。

---

## 五、验收标准

| 项 | 标准 |
|---|---|
| 编译 | fmt / clippy `-D warnings` / `cargo test` / `--all-features` 全绿 |
| 行为 | 上述 7 组用例全绿；现有 72+ lib 用例零回归 |
| 文档 | `docs/metaclouds-optimization-roadmap.md` 的 P2-3 条目改为「已合并」并写明合并 commit |
| 开关 | ✅ 抢占默认关闭（`DISPATCH_PREEMPTION_ENABLED=false`），需显式启用；生产启用前须在真实集群验证。本轮已完成离线契约与 M1/M2 用例，其余 5 组仍待真实执行器 |

---

## 六、阻塞原因

系统定位（演示平台 vs 生产算力平台）尚未拍板；且真实执行器 / `exec_tasks` lease 契约（`docs/real-executor-contract.md`）尚未完整实施。在内存态模拟执行体上做抢占回归，验证的是「模拟作业互抢」，对生产价值有限。

2026-11-16 更新：M1 / M2 两个 major 已以独立契约实现修复，默认关闭；但**生产启用仍被阻塞**——原因不再是「数据结构缺失」，而是「缺少真实执行器来验证 grace-stop / GPU 让出 / 跨租户策略」。因此维持与原判断一致的结论：**抢占的真实启用必须与真实执行器一并立项**，避免第二次返工。本轮交付的是可复用的契约与离线回归，而不是"已可在生产使用"的抢占。
