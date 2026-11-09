# P2-3 抢占分支回归清单（待执行）

- 日期：2026-11-09（第三轮台账化）
- 分支：`park/p2-3-preemption` @ `d3ea543`，约 832 行改动
- 状态：**未回归、未合并**。QA 深验曾通过，但第二轮修复大幅改动了 `dispatch.rs`，且已知 2 处 major，直接 merge 会引入冲突与语义回退。

---

## 一、已知缺陷（恢复时必须先修）

| # | 级别 | 问题 | 修复方向 |
|---|---|---|---|
| M1 | major | victim 选择未按 `cluster` / `partition` 过滤，可能抢占别的分区/集群的作业 | 候选集在 SQL 层按 `partition_id` / `cluster_id` 收窄；补「跨分区不得成为 victim」反向用例 |
| M2 | major | `run_token` 契约与 `complete_running_job` 不一致 | 第二轮给 `complete_running_job` 加了 `status=='running'` 前置 + `rows_affected` 双重幂等；分支的 run_token 需与之对齐或移除重复机制，避免两套幂等互相绕过 |

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
| 开关 | 抢占默认关闭（`DISPATCH_PREEMPTION_ENABLED=false`），需显式启用；生产启用前须在真实集群验证 |

---

## 六、阻塞原因

系统定位（演示平台 vs 生产算力平台）尚未拍板；且真实执行器 / `exec_tasks` lease 契约（`docs/real-executor-contract.md`）未实施。在内存态模拟执行体上做抢占回归，验证的是「模拟作业互抢」，对生产价值有限。**建议与真实执行器一并立项**，避免第二次返工。
