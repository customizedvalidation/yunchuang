# Metaclouds 后端 — Go 退役检查清单（Go Retirement Checklist）

> 工作包：Phase 4 / P4-06
> 编制日期：2026-09-18
> 上游：`docs/rust-migration-execution-plan-2026-09-16.md`（§7 WP-P4-06）
> 配套文档：`docs/cutover-plan.md`（切流方案）、P4-05 Runbook v2（回滚手册）
>
> **本清单性质**：纯检查清单。退役实际操作在**目标生产环境**执行，本机仅交付清单。
> 标注「🔧 待目标环境执行」的项需在生产/预发环境由运维按 Runbook 操作。
> 铁律：**Go 后端代码仓库保留不删**，作为历史参考与回滚源。

---

## A. 退役前确认（Retirement Pre-check）

> 全部勾选后方可进入退役操作。任一项存疑，推迟退役并挂 issue。

- [ ] **A1. 稳定运行**：Rust 版 100% 切流已稳定运行 **≥ 1 周**（D2 ~ D9 无异常）。🔧 待目标环境执行
- [ ] **A2. Golden 回归**：P4-01 报告 P0/P1 diff = 0（状态码/信封/字段/类型无 must-fix 差异）。
- [ ] **A3. 影子双轨**：P4-02 连续 **5 个工作日**观察无 must-fix diff。
- [ ] **A4. 性能基准**：P4-03 报告达标（P99 ≤ Go 版 120%，吞吐达预期）。
- [ ] **A5. 遗留项已评估**：以下 Phase 2 / Phase 3 遗留项均已确认不影响退役，或已记录替代方案：
  - FluidCache HTTP handler 未注册（Go 有 `/datasets/{id}/caches/*`、`/fluid-caches/*`）——确认业务未依赖该面。🔧 待业务确认
  - Postgres 双驱动测试未在真实 PG 跑过——确认 CI `test-postgres` job 已绿。🔧 待 CI 确认
  - Partition 部分 Go 路由无 Rust handler（`PUT /partitions/:id/priority`、`PUT /partitions/:id/max-runtime`、`GET /partitions/:id/permissions` 列表）——确认业务未调用。🔧 待业务确认
  - K8s controller 路由映射差异（`clusters/:id/status`、`resources/gpu`、`jobs/:id/submit`、`jobs/:id/status`）——确认前端/调用方已走 Rust 对等端点（`/k8s/clusters/{id}/...`）。🔧 待业务确认
  - Docker 镜像大小（静态估算 31-41MB，临界 40MB）——CI 实测确认。🔧 待 CI 确认
  - Rust `/health` 端点已实现（根级，无 JWT）——三探针已从 `/metrics` 切换到 `/health`（`k8s/05-deployment.yaml`）。✅ 已确认
  - Alert 权限常量（`alert:read`/`alert:write` 用字符串字面量）——确认 RBAC 矩阵无影响。
- [ ] **A6. 前端全量切指**：前端已全部指向 Rust 版，无残留 Go 版 API 调用（`baseURL` 为相对 `/api/v1`，由 Nginx/Ingress 统一转发到 Rust Service；全局搜索无硬编码 Go 版地址）。🔧 待目标环境执行
- [ ] **A7. 监控切换**：Prometheus/Grafana 已抓取 Rust 版 `/metrics`（ServiceMonitor 15s）；16 告警规则指向 Rust 指标；Go 版面板已停用或标注历史。🔧 待目标环境执行
- [ ] **A8. 日志收集**：日志收集已包含 Rust 版 `trace_id` / `span_id` / `request_id`，可按 `X-Trace-Id` 串联。🔧 待目标环境执行
- [ ] **A9. 数据库备份**：已完成备份——Go 版数据库 **和** Rust 版数据库各自的备份均已验证可恢复。🔧 待目标环境执行
- [ ] **A10. 回滚预案最终确认**：即使退役后仍可回滚（Go 代码仓库保留、Go 数据库保留、镜像/二进制归档）。与 P4-05 Runbook v2 一致。

---

## B. 退役操作步骤（Retirement Steps）

> 在 A 全部勾选后执行。建议在低峰期操作，操作人 + 时间记录归档。

- [ ] **B1. 停止 Go 版服务** 🔧 待目标环境执行
  - K8s：`kubectl scale deployment/<go-deployment> --replicas=0 -n metaclouds`（或对应 Go 版 Deployment 名）。
  - 裸机：停止 Go 进程（systemd stop / 进程 kill）。
  - 确认 Go 版 Pod 全部 Terminated / 进程退出，Ingress/网关已无流量路由。
- [ ] **B2. 保留 Go 版代码仓库**：**不删除**，作为历史参考和回滚源；打 tag 归档（如 `frozen-pre-rust`）。🔧 待目标环境执行
- [ ] **B3. 保留 Go 版数据库**：至少保留 **30 天**，确认无数据回查需求后再归档。🔧 待目标环境执行
- [ ] **B4. 更新文档**：API 参考、Runbook、架构图移除 Go 版相关内容，统一指向 Rust 版（utoipa 导出 `openapi.json`）。🔧 待目标环境执行
- [ ] **B5. 更新 CI/CD**：移除 Go 版构建/测试流水线（Go 版 CI job、镜像构建），保留 Rust 版流水线。🔧 待目标环境执行
- [ ] **B6. 清理监控**：移除 Go 版专属监控指标抓取与面板（如保留则标注为历史，不再告警）。🔧 待目标环境执行
- [ ] **B7. 通知团队**：Go 版已退役，所有新功能在 Rust 版开发；公告含退役时间、操作人、回滚入口。🔧 待目标环境执行
- [ ] **B8. 记录归档**：记录退役时间、操作人、执行清单、回滚预案链接。🔧 待目标环境执行

---

## C. 退役后观察（Post-retirement）

- [ ] **C1. 退役后 24 小时内无异常**：Rust 版错误率/延迟/告警平稳。🔧 待目标环境执行
- [ ] **C2. 退役后 7 天内无回滚需求**：未触发任何回滚条件。🔧 待目标环境执行
- [ ] **C3. 30 天后归档 Go 版数据库**：确认无数据回查需求后，对 Go 版数据库做归档（dump 到冷存储），下线实例。🔧 待目标环境执行
- [ ] **C4. 90 天后可考虑归档 Go 版代码**：打 tag `frozen-pre-rust` 后归档仓库（只读/只读镜像），不删除历史。🔧 待目标环境执行

---

## D. 回滚触发条件（即使退役后）

> 退役不等于不可回滚。以下任一情况发生，按 P4-05 Runbook v2 回滚（启动保留的 Go 版进程 / 恢复 Deployment）。

- **D1. 严重故障无法快速修复**：Rust 版出现严重故障且**无法在 30 分钟内修复**。
- **D2. 数据不一致**：需要从 Go 版数据库恢复数据（在 30 天保留期内可直接恢复；超期需从冷存储 dump 还原）。
- **D3. 业务方紧急要求**：业务方要求紧急回滚到 Go 版。

> 回滚前提：B2（代码仓库保留）、B3（数据库保留 ≥30 天）已执行。超过 30 天且数据库已归档冷存储时，回滚需先还原冷备份，时间相应拉长。

---

## E. 退役验收

- [ ] A 组（退役前确认）10 项全部勾选
- [ ] B 组（退役操作）8 项全部执行
- [ ] C 组（退役后观察）按时间点完成
- [ ] D 组（回滚触发条件）已告知值班与业务方
- [ ] 项目收官评审：Rust 版连续稳定运行，Go 版 0 流量，归档记录完整
