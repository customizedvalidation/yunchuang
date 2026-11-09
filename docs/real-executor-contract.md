# 真实执行器接入契约（设计稿 · 尚未实现）

- 日期：2026-11-09（第三轮，把「未做」细化为「契约已定、待环境实施」）
- 状态：**设计文档，不是已落地代码**。当前生产执行体仍是 `tokio::time::sleep` 后置 `completed`，实耗是 `申请 × 0.75` 合成值并标记 `usage_source='simulated'`。
- 关联：`docs/system-fixes-2026-11-09.md` 第五节、`src/services/dispatch.rs`、`migrations/014_dispatch_lease.sql`

---

## 一、为什么先写契约

第二轮把「模拟」显式化并加了 fail-closed，解决的是「不能把假的当真的用」。要真正交付算力，必须补三件互相耦合的东西，拆开做会留下半套状态机：

1. **真实执行体**：把作业提交到 K8s（Job / CRD / Volcano / Kueue），采集真实退出码与实耗（DCGM）。
2. **执行任务 DB 持久化**：现在的 `tokio::spawn` 是进程内存态，SIGKILL 后在途作业只能等 300s 宽限期被 reconcile 回退，资源可能短时双占。
3. **worker lease / 心跳 + reconcile owner 幂等**：第二轮只做了「启动期租约选主」，运行期没有 owner 租约，多副本下仍可能重复认领。

这三项必须共用同一套 `exec_task` 生命周期，因此以契约形式先钉死边界。

---

## 二、接口契约

```rust
// 建议置于 src/services/executor/mod.rs（当前不存在）
#[async_trait]
pub trait Executor: Send + Sync + 'static {
    /// 幂等：同一 job_id + attempt 重复调用不得创建第二个工作负载
    async fn submit(&self, req: SubmitRequest) -> Result<ExecHandle, ExecError>;
    /// 拉取终态；非终态返回 Running（附最近一次实耗采样）
    async fn poll(&self, handle: &ExecHandle) -> Result<ExecStatus, ExecError>;
    /// 抢占/取消/超时回收，必须可重复调用
    async fn abort(&self, handle: &ExecHandle, reason: AbortReason) -> Result<(), ExecError>;
}

pub struct SubmitRequest {
    pub job_id: String,
    pub attempt: u32,
    pub tenant_id: String,
    pub partition_id: Option<String>,
    pub gang_replicas: u32,
    pub topology_affinity: Option<String>,
    pub gpu_cards: u32,
    pub gpu_fraction: f64,
    pub cpu: i32,
    pub memory_gb: i32,
}

pub enum ExecStatus {
    Running { sampled: Option<UsageSample> },
    Succeeded { finished_at: String, usage: UsageSample },
    Failed { finished_at: String, exit_code: i32, message: String },
    Lost, // 工作负载消失，交由 reconcile 判定
}
```

实现体：
- `SimulatedExecutor`：**保留**，仅在 `ALLOW_SIMULATED_EXECUTION=true` 且非 production 时注册，写 `usage_source='simulated'`。
- `KubeExecutor`：kube-rs 创建 Job / 自定义 CRD；namespace、ServiceAccount、RBAC、配额与 Gang 由集群侧承担；本服务不再自己分配 GPU 卡，改为校验集群回写的分配结果。
- 两者通过 `EXECUTOR_BACKEND=simulated|kube` 选择，`Config::validate()` 中 production + simulated 继续 fail-closed（现有逻辑扩展一条即可）。

---

## 三、状态机（必须与现有 `jobs.status` 对齐）

```
pending --[原子认领 + 写 exec_task(owner,lease_until)]--> claimed
claimed --[executor.submit 成功 + 回写 handle]----------> running
running --[poll: Succeeded / Failed]--------------------> completed / failed
running --[lease 过期且无心跳]--------------------------> orphaned
orphaned --[reconcile 持有 owner 租约者处理]------------> claimed(attempt+1) | failed
任意态 --[abort]----------------------------------------> cancelled
```

约束：
- **认领与写 `exec_task` 必须在同一事务**；`UPDATE jobs SET status='running' WHERE id=? AND status='claimed'` 用 `rows_affected==1` 判定归属（沿用第二轮 `complete_running_job` 的幂等模式）。
- `attempt` 单调递增，K8s Job 名带 `job_id-attempt`，天然幂等。
- 心跳每 `EXEC_LEASE_TTL/3` 续约；`lease_until` 过期前其他副本不得接管。
- 完成路径先写终态 + `usage_source='dcgm'`，再释放 GPU/配额，最后 `UPDATE jobs`；任一步失败由下一次 reconcile 补齐，**不得重复释放**（现有 `complete_running_job` 已有 `rows_affected` 保护，需推广到释放链路）。

---

## 四、存储草案

```sql
-- 双驱动各一份（SQLite / Postgres），纯增量、可空、无外键
CREATE TABLE IF NOT EXISTS exec_tasks (
  job_id          TEXT NOT NULL,
  attempt         INTEGER NOT NULL DEFAULT 1,
  backend         TEXT NOT NULL,             -- simulated | kube
  handle          TEXT,                     -- namespace/name 或外部 ID
  owner_instance  TEXT,                     -- 进程实例 ID（启动期生成）
  lease_until     TEXT,                     -- RFC3339，字典序即时间序（与 014 一致）
  submitted_at    TEXT,
  finished_at     TEXT,
  last_error      TEXT,
  PRIMARY KEY (job_id, attempt)
);
CREATE INDEX IF NOT EXISTS idx_exec_tasks_lease ON exec_tasks(lease_until);
```

`jobs` 追加 `exec_attempt INTEGER DEFAULT 0`、`executor_backend TEXT NULL`；`usage_source` 已有（迁移 015），真实采集写 `'dcgm'`，**计费/showback 必须过滤 `usage_source <> 'simulated'`**。

---

## 五、配置与灰度

| 变量 | 默认 | 说明 |
|---|---|---|
| `EXECUTOR_BACKEND` | `simulated` | 生产必须显式设 `kube`；`validate()` 拦截 production + simulated |
| `EXEC_LEASE_TTL_SECONDS` | `90` | 心跳周期为 TTL/3；须小于 `DISPATCH_RECONCILE_GRACE_SECONDS`(300) |
| `EXEC_POLL_INTERVAL_MS` | `5000` | `poll` 频率，避免打爆 API Server |
| `EXEC_GANG_BACKEND` | `kube` | Gang all-or-nothing 交给 Volcano/Kueue 或由本服务预检 |

灰度顺序：① 先上 `exec_tasks` 表与 SimulatedExecutor 走完整 lease 链路（不改对外行为）→ ② 单副本开 `kube` 跑只读影子作业 → ③ 小规模真实作业 → ④ 全量。每步都要求「模拟执行开关关闭后仍能完成」的回归用例。

---

## 六、验收用例（接入时必须全部通过）

1. 进程 SIGKILL 后，另一副本在 `EXEC_LEASE_TTL` 内接管并继续推进，不重复提交工作负载。
2. `executor.submit` 超时但集群已创建 Job：重启后按 `job_id-attempt` 复用，不产生第二个 Pod。
3. 完成回调重复投递 3 次：`jobs` 只终态一次，GPU/配额只释放一次，`completed_total` 不重复计数。
4. `usage_source='dcgm'` 的实耗进入账本；`simulated` 行被计费查询排除。
5. `ALLOW_SIMULATED_EXECUTION` 未设置且 `SERVER_ENV=production` 时拒绝启动（现有 `Config::validate()` 用例扩展到 `EXECUTOR_BACKEND`）。
6. 三副本并发派发：全局 running 数仍不超过 `DISPATCH_MAX_CONCURRENT`（第二轮原子 UPDATE 用例扩展）。
7. 抢占/取消：`abort` 幂等，资源在 lease 内回收，不留下 `allocated` 状态的 GPU 卡。

---

## 七、解除阻塞的前提

| 前提 | 现状 |
|---|---|
| 真实 K8s 集群 + GPU + 驱动/Device Plugin | 无（当前 `MockK8sClient`） |
| DCGM / dcgm-exporter 实耗采集 | 无 |
| kubeconfig / RBAC / 命名空间配额 | 无 |
| **系统定位拍板**：演示平台 vs 生产算力平台 | 未定 —— 这决定是保留 SimulatedExecutor 长期共存，还是彻底移除模拟路径 |

在这些条件具备前，本文件只作为设计与验收基线，不改变运行时行为。
