//! 优先级调度器核心测试（对齐 Go `pkg/priorityscheduler` 行为契约）。
//!
//! 覆盖：优先级排序、同优先级 FIFO、submit、cancel(pending/running)、状态查询、
//! 队列信息、并发提交/取消、worker 执行、health、shutdown。

use std::sync::Arc;
use std::time::Duration;

use metaclouds_backend_rust::services::priority_scheduler::{
    is_valid_priority, priority_level, JobStatus, NodeInfo, PriorityScheduler, SchedulerConfig,
    SubmitRequest, MAX_PRIORITY,
};

/// 构造一个慢执行（500ms）、单并发的调度器，便于控制 pending/running 状态。
fn slow_config() -> SchedulerConfig {
    SchedulerConfig {
        max_concurrent: 1,
        job_execution_ms: 500,
        channel_buffer: 64,
    }
}

/// 轮询等待 job 达到目标状态（带超时）。
async fn wait_status(s: &PriorityScheduler, id: u64, target: JobStatus) -> JobStatus {
    let mut last = JobStatus::Pending;
    for _ in 0..100 {
        if let Ok(info) = s.get_status(id).await {
            last = info.status;
            if info.status == target {
                return info.status;
            }
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timeout waiting for job {id} to reach {target:?} (last={last:?})");
}

async fn submit(s: &PriorityScheduler, name: &str, priority: u8) -> u64 {
    s.submit_job(SubmitRequest {
        name: name.to_string(),
        priority,
    })
    .await
    .unwrap()
}

// ---------------------------------------------------------------------------
// 工具/常量
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_priority_constants_and_helpers() {
    assert_eq!(MAX_PRIORITY, 3);
    assert!(is_valid_priority(0));
    assert!(is_valid_priority(3));
    assert!(!is_valid_priority(4));
    assert_eq!(priority_level(0), "low");
    assert_eq!(priority_level(1), "medium");
    assert_eq!(priority_level(2), "high");
    assert_eq!(priority_level(3), "critical");
    assert_eq!(priority_level(9), "invalid");
}

// ---------------------------------------------------------------------------
// Submit
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_submit_returns_valid_id_and_pending() {
    let s = PriorityScheduler::new(slow_config());
    let id = submit(&s, "job-a", 1).await;
    assert!(id >= 1);
    let info = s.get_status(id).await.unwrap();
    assert_eq!(info.name, "job-a");
    assert_eq!(info.priority, 1);
    // worker 可能已立即拾取该 job（单并发慢执行），状态为 pending 或 running。
    assert!(
        info.status == JobStatus::Pending || info.status == JobStatus::Running,
        "unexpected status {:?}",
        info.status
    );
    let q = s.get_queue_info().await.unwrap();
    assert!(q.pending + q.running >= 1);
    s.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_submit_rejects_invalid_priority() {
    let s = PriorityScheduler::new(slow_config());
    let res = s
        .submit_job(SubmitRequest {
            name: "bad".into(),
            priority: 9,
        })
        .await;
    assert!(res.is_err());
    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// 优先级排序
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_priority_ordering_higher_runs_first() {
    // 用一个已在运行的低优先级 job 占住单并发许可，确保后续 job 都进队列。
    let s = PriorityScheduler::new(slow_config());
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    // 队列中提交 高(3) / 中(2) / 低(1)。
    let high = submit(&s, "high", 3).await;
    let mid = submit(&s, "mid", 2).await;
    let low = submit(&s, "low", 1).await;

    // occupy 跑完后，队列应按 3→2→1 出队：high 最先进入 running。
    wait_status(&s, high, JobStatus::Running).await;
    let mid_status = s.get_status(mid).await.unwrap().status;
    let low_status = s.get_status(low).await.unwrap().status;
    assert_eq!(
        mid_status,
        JobStatus::Pending,
        "mid should still be pending"
    );
    assert_eq!(
        low_status,
        JobStatus::Pending,
        "low should still be pending"
    );

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// 同优先级 FIFO
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_same_priority_fifo_order() {
    let s = PriorityScheduler::new(slow_config());
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    let first = submit(&s, "first", 2).await;
    let second = submit(&s, "second", 2).await;

    // 同优先级：先提交的 first 应先运行。
    wait_status(&s, first, JobStatus::Running).await;
    let second_status = s.get_status(second).await.unwrap().status;
    assert_eq!(
        second_status,
        JobStatus::Pending,
        "second should still be pending"
    );

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// Cancel
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_cancel_pending_job() {
    let s = PriorityScheduler::new(slow_config());
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    let pending = submit(&s, "pending", 1).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(
        s.get_status(pending).await.unwrap().status,
        JobStatus::Pending
    );

    s.cancel_job(pending).await.unwrap();
    assert_eq!(
        s.get_status(pending).await.unwrap().status,
        JobStatus::Cancelled
    );

    // 队列信息：pending 不计入，cancelled 计入。
    let q = s.get_queue_info().await.unwrap();
    assert_eq!(q.cancelled, 1);
    assert_eq!(q.pending, 0);

    s.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_cancel_running_job() {
    let s = PriorityScheduler::new(slow_config());
    let running = submit(&s, "running", 2).await;
    wait_status(&s, running, JobStatus::Running).await;

    s.cancel_job(running).await.unwrap();
    let st = wait_status(&s, running, JobStatus::Cancelled).await;
    assert_eq!(st, JobStatus::Cancelled);

    s.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_cancel_not_found_and_terminal() {
    let s = PriorityScheduler::new(slow_config());
    // 不存在的 job。
    assert!(s.cancel_job(9999).await.is_err());

    // 先占住单并发许可，再提交一个 pending job，取消后再次取消应报错（终态）。
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    let id = submit(&s, "victim", 1).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(s.get_status(id).await.unwrap().status, JobStatus::Pending);
    s.cancel_job(id).await.unwrap();
    assert_eq!(s.get_status(id).await.unwrap().status, JobStatus::Cancelled);
    // 终态不可再次取消。
    assert!(s.cancel_job(id).await.is_err());

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// GetStatus 状态流转
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_worker_execution_pending_to_running_to_completed() {
    // 用短执行时间快速完成。
    let cfg = SchedulerConfig {
        max_concurrent: 1,
        job_execution_ms: 30,
        channel_buffer: 16,
    };
    let s = PriorityScheduler::new(cfg);
    let id = submit(&s, "quick", 3).await;

    let running = wait_status(&s, id, JobStatus::Running).await;
    assert_eq!(running, JobStatus::Running);
    let completed = wait_status(&s, id, JobStatus::Completed).await;
    assert_eq!(completed, JobStatus::Completed);

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// GetQueueInfo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_queue_info_counts() {
    let s = PriorityScheduler::new(slow_config());
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    let a = submit(&s, "a", 1).await;
    let b = submit(&s, "b", 2).await;
    let c = submit(&s, "c", 2).await;
    let _ = (a, b, c);

    let q = s.get_queue_info().await.unwrap();
    assert_eq!(q.running, 1);
    assert_eq!(q.pending, 3);
    // by_priority: prio1=1, prio2=2。
    assert_eq!(q.by_priority[0], 0);
    assert_eq!(q.by_priority[1], 1);
    assert_eq!(q.by_priority[2], 2);
    assert_eq!(q.by_priority[3], 0);

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// GetNodeInfo
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_node_info() {
    let s = PriorityScheduler::new(slow_config());
    let n: NodeInfo = s.get_node_info().await.unwrap();
    assert!(n.total_nodes > 0);
    assert!(n.total_gpus > 0);
    assert!(n.free_gpus <= n.total_gpus);
    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// SyncJobs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sync_jobs_returns_all() {
    let s = PriorityScheduler::new(slow_config());
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;
    let _a = submit(&s, "a", 1).await;
    let _b = submit(&s, "b", 2).await;

    let all = s.sync_jobs().await.unwrap();
    assert_eq!(all.len(), 3);
    // 按 id 升序。
    assert_eq!(all[0].id, occupy);
    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// 并发
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_concurrent_submit_unique_ids() {
    let s = Arc::new(PriorityScheduler::new(SchedulerConfig {
        max_concurrent: 4,
        job_execution_ms: 50,
        channel_buffer: 128,
    }));
    let mut handles = Vec::new();
    for i in 0..10u64 {
        let s = s.clone();
        handles.push(tokio::spawn(async move {
            submit(&s, "cj", (i % 4) as u8).await
        }));
    }
    let mut ids = Vec::new();
    for h in handles {
        ids.push(h.await.unwrap());
    }
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10, "job ids must be unique");

    // 等待全部完成（短执行）。
    for _ in 0..50 {
        let q = s.get_queue_info().await.unwrap();
        if q.pending == 0 && q.running == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let q = s.get_queue_info().await.unwrap();
    assert_eq!(q.pending, 0);
    assert!(q.completed + q.cancelled >= 10);

    s.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_concurrent_cancel_no_data_race() {
    let s = Arc::new(PriorityScheduler::new(slow_config()));
    let occupy = submit(&s, "occupy", 0).await;
    wait_status(&s, occupy, JobStatus::Running).await;

    // 提交 10 个 pending job。
    let mut ids = Vec::new();
    for _ in 0..10 {
        ids.push(submit(&s, "x", 1).await);
    }

    // 并发取消前 5 个。
    let cancels: Vec<_> = ids[..5]
        .iter()
        .map(|&id| {
            let s = s.clone();
            tokio::spawn(async move {
                let _ = s.cancel_job(id).await;
            })
        })
        .collect();
    for h in cancels {
        h.await.unwrap();
    }

    // 验证前 5 个已取消，后 5 个仍 pending。
    for &id in &ids[..5] {
        assert_eq!(s.get_status(id).await.unwrap().status, JobStatus::Cancelled);
    }
    for &id in &ids[5..] {
        assert_eq!(s.get_status(id).await.unwrap().status, JobStatus::Pending);
    }

    s.shutdown().await.unwrap();
}

// ---------------------------------------------------------------------------
// Health / Shutdown
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_check_after_start_and_shutdown() {
    let s = PriorityScheduler::new(slow_config());
    let h = s.health_check().await.unwrap();
    assert!(h.healthy);
    assert!(h.worker_alive);

    s.shutdown().await.unwrap();
    let h2 = s.health_check().await.unwrap();
    assert!(!h2.healthy, "after shutdown worker should not be healthy");
}

#[tokio::test]
async fn test_shutdown_ends_worker_and_completes_or_aborts() {
    let cfg = SchedulerConfig {
        max_concurrent: 2,
        job_execution_ms: 100,
        channel_buffer: 32,
    };
    let s = PriorityScheduler::new(cfg);
    // 提交多个 job，shutdown 后 worker 应退出。
    let mut ids = Vec::new();
    for _ in 0..5 {
        ids.push(submit(&s, "s", 1).await);
    }
    s.shutdown().await.unwrap();

    // worker 已退出。
    let h = s.health_check().await.unwrap();
    assert!(!h.worker_alive);
    assert!(!h.healthy);

    // shutdown 后 submit 应报错（通道已关闭/worker 已退出）。
    let res = s
        .submit_job(SubmitRequest {
            name: "late".into(),
            priority: 1,
        })
        .await;
    assert!(res.is_err());
}
