//! 优先级调度器核心（对齐 Go `pkg/priorityscheduler` 的行为契约）。
//!
//! # 设计
//!
//! - 优先级队列：[`std::collections::BinaryHeap`]（最大堆），自定义 [`Ord`]：
//!   优先级高者先出队；同优先级按提交序号（`seq`，越小越早）FIFO 先出队。
//! - 调度器：单 worker 任务（`tokio::spawn`）+ `mpsc` 命令通道 + `tokio::select!` 事件循环。
//! - 并发限制：[`tokio::sync::Semaphore`] 控制同时运行的 job 数（默认 10，可配置）。
//! - 取消语义：pending job 惰性删除（标记 cancelled，出队时跳过）；running job 直接 abort。
//!
//! # 与 Go 版语义差异（行为契约对齐）
//!
//! | 维度 | Go 版 | Rust 版 | 裁决 |
//! |------|-------|---------|------|
//! | 优先级方向 | 0=Low … 3=Critical，数字越大优先级越高 | 同 Go（u8，0=Low…3=Critical） | 对齐 Go（任务描述"0=最高"与 Go 测试冲突，以 Go 行为契约为准） |
//! | 队列结构 | `[4][]uint` 四档切片 + `sync.RWMutex` | `BinaryHeap<QueuedJob>`（优先级+序号排序）+ `tokio::sync::Mutex` | Rust idiomatic，语义等价 |
//! | worker | goroutine + `select{chan}` | `tokio::spawn` + `tokio::select!` | 异步等价 |
//! | 并发控制 | 无显式信号量（由调用方 `GetNextTask` 驱动） | `Semaphore` 限制同时运行数 | Rust 版显式并发上限 |
//! | 取消 pending | 从切片移除 | 惰性标记 cancelled，出队时跳过 | 等价语义（BinaryHeap 不支持高效随机删除） |
//! | 执行体 | 调用方自行执行 | 内部 mock sleep（K8s 真实提交留待后续接入） | 保持 mock，记录 |

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Mutex, Notify, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// 优先级常量（对齐 Go types.go）
// ---------------------------------------------------------------------------

pub const PRIORITY_LOW: u8 = 0;
pub const PRIORITY_MEDIUM: u8 = 1;
pub const PRIORITY_HIGH: u8 = 2;
pub const PRIORITY_CRITICAL: u8 = 3;
pub const MAX_PRIORITY: u8 = PRIORITY_CRITICAL;

/// 判断优先级是否合法（0..=3）。
pub fn is_valid_priority(p: u8) -> bool {
    p <= MAX_PRIORITY
}

/// 优先级级别名称（对齐 Go `GetPriorityLevel`）。
pub fn priority_level(p: u8) -> &'static str {
    match p {
        PRIORITY_LOW => "low",
        PRIORITY_MEDIUM => "medium",
        PRIORITY_HIGH => "high",
        PRIORITY_CRITICAL => "critical",
        _ => "invalid",
    }
}

// ---------------------------------------------------------------------------
// Job 状态与记录
// ---------------------------------------------------------------------------

/// Job 状态（对齐 Go TaskStatus）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 内部 job 记录。
#[derive(Debug, Clone)]
struct JobRecord {
    id: u64,
    priority: u8,
    name: String,
    status: JobStatus,
}

/// 出队元素：仅携带排序所需字段（BinaryHeap 最大堆）。
#[derive(Debug, Clone, Eq, PartialEq)]
struct QueuedJob {
    id: u64,
    priority: u8,
    seq: u64,
}

/// 排序规则：优先级高者先出队；同优先级序号小（提交早）者先出队。
impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// 对外信息结构
// ---------------------------------------------------------------------------

/// 提交 job 入参。
#[derive(Debug, Clone)]
pub struct SubmitRequest {
    pub name: String,
    pub priority: u8,
}

/// job 状态快照（`get_status` 返回）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobInfo {
    pub id: u64,
    pub priority: u8,
    pub name: String,
    pub status: JobStatus,
}

/// 队列信息（`get_queue_info` 返回）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueueInfo {
    pub pending: usize,
    pub running: usize,
    pub completed: usize,
    pub cancelled: usize,
    /// 各优先级（0..=3）的 pending 数量。
    pub by_priority: [usize; 4],
}

/// 节点信息（`get_node_info` 返回，mock）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInfo {
    pub total_nodes: u32,
    pub total_cpus: u32,
    pub total_memory_gb: u32,
    pub total_gpus: u32,
    pub free_gpus: u32,
}

impl Default for NodeInfo {
    fn default() -> Self {
        Self {
            total_nodes: 8,
            total_cpus: 128,
            total_memory_gb: 512,
            total_gpus: 32,
            free_gpus: 24,
        }
    }
}

/// 健康状态（`health_check` 返回）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    pub healthy: bool,
    pub worker_alive: bool,
    pub queued: usize,
    pub running: usize,
}

/// 调度器配置。
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// 同时运行的最大 job 数（信号量许可数）。
    pub max_concurrent: usize,
    /// mock 执行耗时（毫秒）。
    pub job_execution_ms: u64,
    /// 命令通道缓冲。
    pub channel_buffer: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            job_execution_ms: 20,
            channel_buffer: 64,
        }
    }
}

// ---------------------------------------------------------------------------
// 命令通道
// ---------------------------------------------------------------------------

enum SchedulerCommand {
    Submit {
        req: SubmitRequest,
        reply: oneshot::Sender<u64>,
    },
    Cancel {
        id: u64,
        reply: oneshot::Sender<Result<(), CancelError>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelError {
    NotFound,
    InvalidState,
}

// ---------------------------------------------------------------------------
// 共享状态
// ---------------------------------------------------------------------------

struct Shared {
    queue: BinaryHeap<QueuedJob>,
    jobs: HashMap<u64, JobRecord>,
    running: HashMap<u64, JoinHandle<()>>,
    next_job_id: u64,
    next_seq: u64,
    node_info: NodeInfo,
    shutting_down: bool,
}

impl Shared {
    fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            jobs: HashMap::new(),
            running: HashMap::new(),
            next_job_id: 1,
            next_seq: 0,
            node_info: NodeInfo::default(),
            shutting_down: false,
        }
    }
}

// ---------------------------------------------------------------------------
// PriorityScheduler
// ---------------------------------------------------------------------------

/// 优先级调度器核心。
pub struct PriorityScheduler {
    tx: mpsc::Sender<SchedulerCommand>,
    shared: Arc<Mutex<Shared>>,
    worker: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl PriorityScheduler {
    /// 构造并立即启动 worker 任务。
    pub fn new(config: SchedulerConfig) -> Self {
        let (tx, rx) = mpsc::channel(config.channel_buffer.max(1));
        let shared = Arc::new(Mutex::new(Shared::new()));
        let sem = Arc::new(Semaphore::new(config.max_concurrent.max(1)));
        let notify = Arc::new(Notify::new());

        let worker_shared = shared.clone();
        let worker_sem = sem.clone();
        let worker_notify = notify.clone();
        let worker = tokio::spawn(worker_loop(
            rx,
            worker_shared,
            worker_sem,
            worker_notify,
            config.job_execution_ms.max(1),
        ));

        Self {
            tx,
            shared,
            worker: tokio::sync::Mutex::new(Some(worker)),
        }
    }

    /// 提交一个 job，返回分配的 job_id。
    pub async fn submit_job(&self, req: SubmitRequest) -> AppResult<u64> {
        if !is_valid_priority(req.priority) {
            return Err(AppError::bad_request(format!(
                "priority must be 0..={MAX_PRIORITY}"
            )));
        }
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(SchedulerCommand::Submit { req, reply })
            .await
            .map_err(|_| AppError::internal("scheduler worker channel closed"))?;
        let id = rx
            .await
            .map_err(|_| AppError::internal("scheduler dropped submit reply"))?;
        Ok(id)
    }

    /// 取消一个 job（pending 惰性删除；running 直接 abort）。
    pub async fn cancel_job(&self, id: u64) -> AppResult<()> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(SchedulerCommand::Cancel { id, reply })
            .await
            .map_err(|_| AppError::internal("scheduler worker channel closed"))?;
        match rx
            .await
            .map_err(|_| AppError::internal("scheduler dropped cancel reply"))?
        {
            Ok(()) => Ok(()),
            Err(CancelError::NotFound) => Err(AppError::not_found(format!("job {id} not found"))),
            Err(CancelError::InvalidState) => Err(AppError::conflict(format!(
                "job {id} is in a non-cancellable terminal state"
            ))),
        }
    }

    /// 查询 job 状态。
    pub async fn get_status(&self, id: u64) -> AppResult<JobInfo> {
        let s = self.shared.lock().await;
        let rec = s
            .jobs
            .get(&id)
            .ok_or_else(|| AppError::not_found(format!("job {id} not found")))?;
        Ok(JobInfo {
            id: rec.id,
            priority: rec.priority,
            name: rec.name.clone(),
            status: rec.status,
        })
    }

    /// 队列快照。
    pub async fn get_queue_info(&self) -> AppResult<QueueInfo> {
        let s = self.shared.lock().await;
        let mut info = QueueInfo::default();
        for rec in s.jobs.values() {
            match rec.status {
                JobStatus::Pending => info.pending += 1,
                JobStatus::Running => info.running += 1,
                JobStatus::Completed => info.completed += 1,
                JobStatus::Cancelled => info.cancelled += 1,
                JobStatus::Failed => {}
            }
            if rec.status == JobStatus::Pending && rec.priority <= MAX_PRIORITY {
                info.by_priority[rec.priority as usize] += 1;
            }
        }
        Ok(info)
    }

    /// 节点信息（mock 快照）。
    pub async fn get_node_info(&self) -> AppResult<NodeInfo> {
        let s = self.shared.lock().await;
        Ok(s.node_info.clone())
    }

    /// 同步 job 状态（mock：从内部状态返回全部 job 快照）。
    pub async fn sync_jobs(&self) -> AppResult<Vec<JobInfo>> {
        let s = self.shared.lock().await;
        let mut v: Vec<JobInfo> = s
            .jobs
            .values()
            .map(|r| JobInfo {
                id: r.id,
                priority: r.priority,
                name: r.name.clone(),
                status: r.status,
            })
            .collect();
        v.sort_by_key(|j| j.id);
        Ok(v)
    }

    /// 健康检查：worker 是否存活、队列/运行计数。
    pub async fn health_check(&self) -> AppResult<HealthStatus> {
        let worker_alive = {
            let w = self.worker.lock().await;
            w.as_ref().map(|h| !h.is_finished()).unwrap_or(false)
        };
        let s = self.shared.lock().await;
        let queued = s.queue.len();
        let running = s.running.len();
        Ok(HealthStatus {
            healthy: worker_alive && !s.shutting_down,
            worker_alive,
            queued,
            running,
        })
    }

    /// 优雅关闭：通知 worker，等待运行中 job 结束（带超时），abort 残留任务。
    pub async fn shutdown(&self) -> AppResult<()> {
        let (reply, rx) = oneshot::channel();
        if self
            .tx
            .send(SchedulerCommand::Shutdown { reply })
            .await
            .is_ok()
        {
            let _ = rx.await;
        }
        // 等待 worker 结束（最多 2 秒）。
        let mut w = self.worker.lock().await;
        if let Some(handle) = w.take() {
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Worker 事件循环
// ---------------------------------------------------------------------------

async fn worker_loop(
    mut rx: mpsc::Receiver<SchedulerCommand>,
    shared: Arc<Mutex<Shared>>,
    sem: Arc<Semaphore>,
    notify: Arc<Notify>,
    job_execution_ms: u64,
) {
    loop {
        // 先把能跑的都跑起来。
        drain_runnable(&shared, &sem, &notify, job_execution_ms).await;

        // 关闭信号且通道已空 → 退出。
        let shutdown = {
            let s = shared.lock().await;
            s.shutting_down && rx.is_empty()
        };
        if shutdown {
            break;
        }

        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    Some(c) => handle_command(c, &shared).await,
                    None => break,
                }
            }
            _ = notify.notified() => {}
        }
    }

    // 关闭：abort 所有仍在运行的任务（permit 随 task drop 释放）。
    let mut s = shared.lock().await;
    for (_, h) in s.running.drain() {
        h.abort();
    }
}

/// 从队列弹出可运行 job 并 spawn，直到无许可或队列为空。
async fn drain_runnable(
    shared: &Arc<Mutex<Shared>>,
    sem: &Arc<Semaphore>,
    notify: &Arc<Notify>,
    job_execution_ms: u64,
) {
    loop {
        // 非阻塞获取许可；无许可即停。
        let permit: OwnedSemaphorePermit = match sem.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => break,
        };

        // 弹出队首。
        let queued = {
            let mut s = shared.lock().await;
            if s.shutting_down {
                break;
            }
            s.queue.pop()
        };
        let Some(qj) = queued else { break };

        // 惰性删除：若该 job 已被取消/终态，跳过。
        let still_pending = {
            let s = shared.lock().await;
            s.jobs
                .get(&qj.id)
                .map(|r| r.status == JobStatus::Pending)
                .unwrap_or(false)
        };
        if !still_pending {
            drop(permit);
            continue;
        }

        // 标记 running。
        let s2 = shared.clone();
        let notify2 = notify.clone();
        let id = qj.id;
        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(job_execution_ms)).await;
            let mut st = s2.lock().await;
            if let Some(rec) = st.jobs.get_mut(&id) {
                if rec.status == JobStatus::Running {
                    rec.status = JobStatus::Completed;
                }
            }
            st.running.remove(&id);
            drop(permit);
            notify2.notify_one();
        });

        let mut st = shared.lock().await;
        st.running.insert(id, handle);
        if let Some(rec) = st.jobs.get_mut(&id) {
            rec.status = JobStatus::Running;
        }
    }
}

async fn handle_command(cmd: SchedulerCommand, shared: &Arc<Mutex<Shared>>) {
    match cmd {
        SchedulerCommand::Submit { req, reply } => {
            let mut s = shared.lock().await;
            let id = s.next_job_id;
            s.next_job_id += 1;
            let seq = s.next_seq;
            s.next_seq += 1;
            s.jobs.insert(
                id,
                JobRecord {
                    id,
                    priority: req.priority,
                    name: req.name,
                    status: JobStatus::Pending,
                },
            );
            s.queue.push(QueuedJob {
                id,
                priority: req.priority,
                seq,
            });
            let _ = reply.send(id);
        }
        SchedulerCommand::Cancel { id, reply } => {
            let mut s = shared.lock().await;
            let res = cancel_inner(&mut s, id);
            let _ = reply.send(res);
        }
        SchedulerCommand::Shutdown { reply } => {
            let mut s = shared.lock().await;
            s.shutting_down = true;
            let _ = reply.send(());
        }
    }
}

fn cancel_inner(s: &mut Shared, id: u64) -> Result<(), CancelError> {
    let rec = s.jobs.get_mut(&id).ok_or(CancelError::NotFound)?;
    match rec.status {
        JobStatus::Pending => {
            // 惰性删除：队列中的 stale entry 出队时会被跳过。
            rec.status = JobStatus::Cancelled;
            Ok(())
        }
        JobStatus::Running => {
            if let Some(h) = s.running.remove(&id) {
                h.abort();
            }
            rec.status = JobStatus::Cancelled;
            Ok(())
        }
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
            Err(CancelError::InvalidState)
        }
    }
}
