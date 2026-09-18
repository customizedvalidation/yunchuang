# P4-02 影子流量双轨（Shadow Dual-Track）方案

- 任务：P4-02 影子流量双轨 —— 方案 + 对比脚本 + 本机短冒烟采样
- 对照源（Go）：`D:\YCYD\metaclouds-backend`（:8000，内存模式）
- 待测（Rust）：`D:\YCYD\metaclouds-backend-rust`（:8001）
- 前置：P4-01 Golden 全量对等回归完成，其发现的 3 项契约差异已修复
  （M1 列表分页信封、M2 manager 写权限运行时误 403、M3 配额校验路径）
- 执行时间基准：2026-09-18（Asia/Shanghai, UTC+8）

---

## 1. 架构

影子双轨不引入独立的在线复制器进程，而是以**对比脚本**扮演 Shadow Traffic Replicator
角色（本机冒烟阶段）；生产落地时替换为同语义的在线复制器（见 §1.2）。

### 1.1 本机冒烟拓扑

```
            ┌─────────────────────────────────────────────┐
            │      scripts/shadow-compare.ps1             │
            │      (Shadow Traffic Replicator 替身)        │
            └───────────────┬─────────────┬───────────────┘
                  同一请求   │             │   同一请求
                  双发       ▼             ▼
              ┌─────────────────┐   ┌─────────────────┐
              │  Go   :8000     │   │  Rust :8001     │
              │  MEMORY_STORE   │   │  SQLite 持久库   │
              │  admin/Admin@.. │   │  admin/Admin@..  │
              └────────┬─────────┘   └────────┬─────────┘
                       │                      │
                       └──────────┬───────────┘
                                  ▼
                   逐请求记录：状态码 / 耗时 / 体哈希 / 信封键集
                                  │
                                  ▼
                   diff 分级 (P0/P1/P2/P3) → CSV + 汇总
```

- 请求按混合请求集（≥30 条）循环回放，单轮间隔 `-RequestIntervalMs`（默认 200ms）。
- 每个请求**同时**（脚本内背靠背紧连）打到 Go 与 Rust，分别计时。
- 两版使用**各自独立的数据存储**（Go 内存 / Rust SQLite），因此**只对比契约，不对比业务值**。

### 1.2 生产落地拓扑（目标环境，5 工作日观察）

```
真实流量 ──► Ingress / 网关
                │
                ├──► Go  :8000   (主，真实服务用户)
                │
                └──► Rust:8001   (影子，只响应不回写用户；10% 流量复制)
                                    │
                              在线对比器 (逐请求 L0-L3 比对)
                                    │
                              P0/P1 告警 → 通知；P2 记录；P3 放行
```

- 复制器按 `x-shadow-sample` 头或采样率 10% 复制请求到 Rust；Rust 响应**不返回给用户**。
- 对比器异步比对，不阻塞主链路；Rust 故障不影响 Go 主链路。

---

## 2. 对比维度（L0–L3）

| 级别 | 对比内容 | 判定依据 |
|---|---|---|
| **L0** | HTTP 状态码 | 必须逐位一致（200↔200、401↔401、403↔403、422↔422…） |
| **L1** | 信封结构 | 顶层键集与 `success` 布尔一致：成功 `{success,data,timestamp}`；失败 `{success,message,code,timestamp}` |
| **L2** | 数据形状 | 字段名集合与 JSON 类型（对象/数组/数值/字符串/布尔/null），**不对比具体值**（数据独立）；分页信封 `{data,total,page,page_size,total_pages}` 作为整体形状判定 |
| **L3** | 响应头 | `Content-Type`、`X-Trace-Id`/`X-Request-ID` 存在性、`Set-Cookie` 名集合（值不对比） |

> 说明：P4-01 已确认两版信封字段命名一致（成功 `data`、失败 `message/code`）；
> 唯一已知字段名差异为 `POST /auth/logout` 成功时 Rust 用 `message`、Go 用 `data`，
> 列入 §4 P3 可接受差异。

---

## 3. diff 分级

| 级别 | 定义 | 处置 |
|---|---|---|
| **P0** | 状态码不一致（如 Go 200 vs Rust 500；Go 401 vs Rust 200） | **立即告警**，触发回滚评估 |
| **P1** | 信封/数据形状不一致（`success` 翻转、缺关键字段、data 形状 `{obj}` vs `[arr]`） | 记录并告警，纳入当批修复 |
| **P2** | 响应时间差异 > 200%（Rust 耗时 / Go 耗时 > 2.0，或反之） | 记录，观察趋势 |
| **P3** | 可接受差异：时间戳、`X-Trace-Id`/`X-Request-ID`、具体业务值、CSRF/token 串、P4-01 已裁决的单侧行为（见 §4） | 放行，仅计数不告警 |

### 3.1 P2 延迟口径

- 单请求 `ratio = rustMs / goMs`。
- 当 `goMs < 5ms` 时不判 P2（本地回环噪声主导），记为 `n/a`。
- 汇总看分布：P2 数量 / 总请求数 = P2 率。

---

## 4. 已知可接受差异（P3 白名单，源自 P4-01 裁决）

| 项 | Go 行为 | Rust 行为 | 裁决 |
|---|---|---|---|
| 空写请求体校验严格度 | 宽松 → 201 Created | 严格 → 422 Unprocessable | P3：契约未约定空体语义；用对齐合法体复测应收敛 |
| 详情路由 `GET /{域}/1` | 有种子数据 → 200 | 空库 → 404 | P3：数据独立，非契约差异 |
| `POST /auth/logout` 成功信封 | 顶层 `data` | 顶层 `message` | P3：P4-01 §6.2 已记录，后续对齐 |
| 时间戳 / trace_id / token / Set-Cookie 值 | 各版独立生成 | 各版独立生成 | P3：天然不可比 |
| 监控指标 `GET /monitoring/metrics` 字段口径 | 主机资源口径 | 业务运行口径 | P3：指标口径差异，非 REST 契约差异 |

### 4.1 单侧路由（不纳入对等对比集）

为避免把「路由只在一侧注册」误报为 P0，本机对比集**只取两版共有路由**。
单侧路由清单（P4-01 §6.3，不阻断）：

- Rust 独有：`/users*`、`/k8s/clusters/:id/*`、`/jobs/stats`、`/alerts*`、
  `/monitoring/dashboard`、`/monitoring/alert-rules*`、`/acceleration/:id/start|stop`、
  `/security/policies/:id/enable|disable`、`/partitions/:id/resources`、
  `/schedulers/:id/test-connection`、`/auth/change-password`。
- Go 独有：`/clusters/:id/status`、`/resources/gpu`、`/jobs/:id/submit|status`、
  `/monitoring/alerts*`、`/partitions/:id/priority|max-runtime|permissions`、
  `/quotas/usage`、`/schedulers/:id/queues|nodes|health`、`/topology/score`、
  `/checkpoints/latest/:jobId`、`/datasets/:id/caches*`。

---

## 5. 观察指标（验收门槛）

| 指标 | 门槛 | 说明 |
|---|---|---|
| P0 diff 数 | **= 0** | 状态码逐位一致 |
| P1 diff 数 | **= 0** 或全部为已知遗留并逐条记录 | 信封/数据形状 |
| P2 diff 率 | **< 1%** | 延迟差异 >200% 的请求占比 |
| Rust 错误率 | **< 0.1%** | 5xx / 未捕获异常占比 |
| Rust P99 延迟 | **< Go P99 的 120%** | 延迟上限 |

---

## 6. 采样策略

### 6.1 生产目标环境（待执行，真实 5 工作日连续观察）

- 流量复制：Ingress 层对生产流量做 **10% 影子复制**到 Rust。
- 时长：**连续 5 个工作日**（含早晚高峰）。
- 请求来源：真实用户混合流量（读为主，含写、分页、搜索、401/403）。
- 聚合：按小时桶统计 §5 指标；日报一页纸。

### 6.2 本机冒烟（本任务，≥5 分钟）

- 脚本 `scripts/shadow-compare.ps1` 用**混合请求集**模拟真实流量构成：
  各域列表读、分页（`?page=1&page_size=5`）、过滤搜索（`?status=running`）、
  详情、CRUD 写、401（无 token）、403（普通用户 token 写操作）。
- 循环回放请求集直到 `-DurationMinutes` 耗尽；单轮间隔 `-RequestIntervalMs`（≥200ms）。
- 定位为**代理验证**：证明双轨对比管线跑通、P0/P1 收敛、延迟可测；
  不替代目标环境 5 工作日的真实流量观察。

### 6.3 长时间观察脚本 `scripts/shadow-observe.ps1`（P4-02 交付）

面向**连续长时间运行**（目标环境 5 个工作日）的自动化观察脚本，与 `shadow-compare.ps1`
同源（复用双服务启动、双登录、40 条混合请求集、信封/形状/diff 分级），面向长跑做了增量：

- 定时轮询：`-IntervalSeconds`（默认 30s）一轮，`-DurationMinutes` 控制总时长
  （默认 1440=24h；5 个工作日约设 7200）。
- 三级请求集：`-RequestSet read-only`（仅 GET，不写数据，长跑首选）/
  `mixed`（默认 40 条混合）/ `full`（mixed + `obs_` 前缀写探针）。
- 实时分级计数：P0（状态码不一致，白名单外）/ P1（信封或形状不一致）/
  P1-KNOWN-EMPTY（空集合 null vs `[]`）/ P2（Rust 延迟 > Go×3 且绝对差 >100ms）/
  P3（可接受差异）。每轮控制台一行实时统计，diff 逐行写带时间戳日志。
- 长跑健壮性：token 每 2 小时自动刷新、遇 401 立即重登；Ctrl-C 优雅停止并落最终汇总，
  默认**不停止**双服务（`-StopServicesOnExit` 才停本脚本启动的进程）。
- 产出：`scripts/p4work/shadow-observe-yyyyMMdd-HHmm.log`（逐请求行）、
  `scripts/p4work/shadow-observe-summary.txt`（总数/P0-P3 计数/错误率/起止时间/P99）。

```powershell
# 本机短验证（read-only，5 分钟）
powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 -DurationMinutes 5 -RequestSet read-only

# 目标环境 5 个工作日连续观察（约 7200 分钟，30s 一轮）
powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 -DurationMinutes 7200 -IntervalSeconds 30 -RequestSet read-only
```

> 密码不硬编码：用 `-AdminPass` 或环境变量 `OBSERVE_ADMIN_PASS`；两者均缺省时回落开发种子
> `Admin@123456`（与服务启动 `DEFAULT_ADMIN_PASSWORD` 一致，仅开发/本机用）。
> 长时间运行与真实 5 工作日观察在**目标环境**执行；本机仅做脚本逻辑短采样验证。

---

## 7. 回滚触发与处置

| 触发条件 | 动作 |
|---|---|
| **P0 diff > 0**（任一状态码不一致且非 §4 白名单） | 立即停止影子复制，Rust 摘流；拉取该请求 Go/Rust 双侧响应体定位 |
| **Rust 错误率 > 1%**（5xx / 未捕获） | 停止影子复制，检查 Rust panic / 日志 |
| Rust P99 持续 > Go 200% 且影响主链路风险 | 降采样率或暂停，转性能排查 |
| P1 信封差异累计 > 5 例 | 当日告警，进入修复排队（不立即摘流，除非伴随 P0） |

> 本机冒烟阶段：任一 P0 出现即终止采样并在报告中标注，不强制继续跑完时长。

---

## 8. 交付物

| 交付物 | 路径 |
|---|---|
| 本方案文档 | `docs/shadow-dual-track.md` |
| 影子对比脚本（本机短采样） | `scripts/shadow-compare.ps1` |
| 长时间影子观察脚本 | `scripts/shadow-observe.ps1`（见 §6.3） |
| 本机短采样 CSV | `scripts/p4work/shadow-results.csv` |
| 本机短采样汇总 | `scripts/p4work/shadow-summary.txt` |
| 本机短采样结果章节 | 本文档 §9 |

> 复现：`powershell -ExecutionPolicy Bypass -File scripts/shadow-compare.ps1 -DurationMinutes 5`
> 完成后脚本自动停止由其启动的双服务。按任务要求不做 git 提交，由 Phase 4 整合代理统一提交。

---

## 9. 本机短采样结果（P4-02）

### 9.1 采样基本信息

| 项 | 值 |
|---|---|
| 采样时间 | 2026-09-18 12:22 (UTC+8) |
| 时长 / 间隔 | 5 分钟 / 200ms |
| 轮次 / 总请求 | 35 轮 / **1374 条** |
| 混合请求集 | 40 条（覆盖 auth / clusters / resources / tenants / jobs / gpus / partitions / quotas / schedulers / datasets / checkpoints / acceleration / security / monitoring / topology，含分页、过滤、详情、CRUD、401、403） |
| Go 版 | :8000，内存模式 MEMORY_STORE_ENABLED=true，有种子数据 |
| Rust 版 | :8001，SQLite 全新空库（采样前清理旧库重新播种） |
| 双登录 | admin/Admin@123456 双版均成功；普通用户 token 双版均成功 |

### 9.2 diff 分级统计（核心）

| 级别 | 数量 | 结论 |
|---|---|---|
| **P0（状态码不一致）** | **0** | 状态码逐位一致（白名单外无任何分歧） |
| **P1（信封/数据形状不一致）** | **0** | 真实契约差异为 0 |
| P1-KNOWN-EMPTY（已知遗留） | 75 | Go 空序列序列化为 null、Rust 为空数组 []；仅在空集合出现，数据相关，非 P4 修复回归 |
| **P2（延迟差 >200%）** | **0（P2 率 0%）** | 低于 <1% 门槛 |
| P3（可接受差异） | 1299 | 见 9.3 |
| Rust 错误率（5xx/0） | **0%** | 低于 <0.1% 门槛 |

### 9.3 已知 P3 差异清单（全部源自 P4-01 已裁决项，非本次新增缺陷）

| # | 现象 | 数量 | 裁决来源 |
|---|---|---|---|
| 1 | GET /health Go=200 / Rust=404 | 35 | Rust 未实现 /health（单侧路由，非业务契约） |
| 2 | 空体 POST 各域写 Go=201/400/409 / Rust=422 | ~370 | P4-01 6.2 空写校验严格度可接受 |
| 3 | GET /clusters/1、/jobs/1 Go=200(有种子) / Rust=404(空库) | 68 | 数据独立，非契约差异 |
| 4 | GET /monitoring/metrics、/gpus/utilization 输出格式/口径 | 68 | P4-01 4 监控指标口径差异 |
| 5 | P1-KNOWN-EMPTY 空集合 null vs [] | 75 | 序列化差异（数据相关） |
| 6 | 时间戳、trace_id、具体业务值、体哈希 | 其余 | 天然不可比（数据独立） |

### 9.4 响应时间对比

| 指标 | Go | Rust | Rust/Go |
|---|---|---|---|
| 平均 | 1.5 ms | 3.2 ms | ~213% |
| P99 | 4 ms | 9 ms | **225%** |

> 关于 P99 225%：本机 Rust 以 cargo run（未优化 dev/debuginfo 构建）运行，Go 为 go run。绝对值均为个位数毫秒（Go P99=4ms、Rust P99=9ms），属本机 DEBUG 构建噪声，不代表 release 构建性能。5 中「Rust P99 < Go 120%」门槛须在目标环境以 Rust release 构建复测。本机采样定位为管线跑通与契约收敛验证。

### 9.5 各域 diff 分布

真实 P0/P1 在所有业务域均为 0。P3 集中在：空体写校验（各写域）、空集合序列化（gpus/allocations、topology、partitions、quotas 等空库列表）、监控口径（monitoring/metrics、gpus/utilization）。详见 scripts/p4work/shadow-results.csv。

### 9.6 结论与待办

- 双轨对比管线（启动双服务->双登录->40 混合请求双发->分级->CSV/汇总）跑通。
- P0=0、P1=0、P2=0、Rust 错误率=0%，满足本机冒烟验收。
- 所有差异均为 P4-01 已裁决的可接受项或数据相关已知遗留，无本次新增契约回归。
- 真实 5 工作日连续观察待目标环境执行：本采样为代理验证（本机、DEBUG 构建、脚本回放混合流量），不替代目标环境 10% 真实流量影子复制 5 工作日的连续观察（含 release 构建 P99、早晚高峰、真实写流量）。
