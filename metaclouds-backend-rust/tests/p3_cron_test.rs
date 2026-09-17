//! P3-02 tokio-cron-scheduler 对齐测试。
//!
//! 覆盖：
//! - Go 侧默认任务表与 Rust 表达式对照表（DEFAULT_JOBS）一致。
//! - 每个 Go 表达式映射出的 Rust 7 字段表达式都能被 tokio-cron 解析。
//! - 非生产环境注册 2 个任务（对齐 Go AddDefaultSchedules），生产环境为 0。
//! - 触发时刻与手工计算的 UTC 边界对齐（双端偏差 < 2s）。
//! - Scheduler start/shutdown 不 panic。
//! - 任务 panic 不影响调度循环（注入 panic 任务，其他任务照常触发）。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Timelike, Utc};
use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::scheduler::{Scheduler, DEFAULT_JOBS};
use tokio_cron_scheduler::{Job, JobScheduler};

/// 构造一个开发环境配置（Scheduler 不做 validate，可直接用 default）。
fn dev_config() -> Arc<Config> {
    Arc::new(Config::default())
}

async fn memory_pool() -> sqlx::SqlitePool {
    sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("in-memory sqlite pool")
}

/// 手工计算"下一个 30 分钟边界"（秒=0、分∈{0,30}），严格晚于 now。
fn next_half_hour_boundary(now: DateTime<Utc>) -> DateTime<Utc> {
    let base = now
        .with_nanosecond(0)
        .expect("zero nanos")
        .with_second(0)
        .expect("zero second");
    let floored = base.with_minute((base.minute() / 30) * 30).expect("minute");
    if floored <= now {
        floored + chrono::Duration::minutes(30)
    } else {
        floored
    }
}

/// 手工计算"下一个偶数小时整点"（秒=0、分=0、偶小时），严格晚于 now。
fn next_even_hour_boundary(now: DateTime<Utc>) -> DateTime<Utc> {
    let base = now
        .with_nanosecond(0)
        .expect("zero nanos")
        .with_second(0)
        .expect("zero second")
        .with_minute(0)
        .expect("zero minute");
    let floored = base.with_hour((base.hour() / 2) * 2).expect("hour");
    if floored <= now {
        floored + chrono::Duration::hours(2)
    } else {
        floored
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn default_jobs_table_matches_go() {
    // Go AddDefaultSchedules 恰好注册 2 个任务。
    assert_eq!(
        DEFAULT_JOBS.len(),
        2,
        "go side registers exactly 2 default jobs"
    );
    let names: Vec<&str> = DEFAULT_JOBS.iter().map(|(n, _, _)| *n).collect();
    assert!(names.contains(&"sample-training"));
    assert!(names.contains(&"sample-inference"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rust_expressions_parse_ok() {
    for &(name, go_expr, rust_expr) in DEFAULT_JOBS {
        let job = Job::new_async_tz(rust_expr, Utc, |_uuid, _lock| Box::pin(async {}));
        assert!(
            job.is_ok(),
            "rust expression for '{name}' (go: '{go_expr}') = '{rust_expr}' must parse"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dev_registers_two_jobs() {
    let sched = Scheduler::new(memory_pool().await, dev_config())
        .await
        .expect("scheduler");
    sched.register_all_jobs().await.expect("register");
    assert_eq!(sched.registered_count(), 2);
    sched.shutdown().await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn production_registers_nothing() {
    let cfg = Config {
        environment: "production".to_string(),
        ..Config::default()
    };
    let sched = Scheduler::new(memory_pool().await, Arc::new(cfg))
        .await
        .expect("scheduler");
    sched.register_all_jobs().await.expect("register");
    assert_eq!(sched.registered_count(), 0, "prod must skip sample jobs");
    sched.shutdown().await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn next_tick_sample_training_aligned() {
    let expr: &str = DEFAULT_JOBS[0].2;
    let mut sched = JobScheduler::new().await.expect("scheduler");
    let job = Job::new_async_tz(expr, Utc, |_uuid, _lock| Box::pin(async {})).expect("job");
    let id = sched.add(job).await.expect("add");

    let now = Utc::now();
    let next = sched
        .next_tick_for_job(id)
        .await
        .expect("query next tick")
        .expect("next tick present");

    // UTC 触发：秒必须为 0，且落在下一个 :00 / :30 边界上（偏差 < 2s）。
    assert_eq!(next.second(), 0, "sample-training must fire at second 0");
    let expected = next_half_hour_boundary(now);
    let diff = (next - expected).num_seconds().abs();
    assert!(diff <= 2, "next={next} expected={expected} diff={diff}s");
    sched.shutdown().await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn next_tick_sample_inference_aligned() {
    let expr: &str = DEFAULT_JOBS[1].2;
    let mut sched = JobScheduler::new().await.expect("scheduler");
    let job = Job::new_async_tz(expr, Utc, |_uuid, _lock| Box::pin(async {})).expect("job");
    let id = sched.add(job).await.expect("add");

    let now = Utc::now();
    let next = sched
        .next_tick_for_job(id)
        .await
        .expect("query next tick")
        .expect("next tick present");

    // UTC 触发：秒=0、分=0、偶小时。
    assert_eq!(next.second(), 0, "sample-inference must fire at second 0");
    assert_eq!(next.minute(), 0, "sample-inference must fire at minute 0");
    let expected = next_even_hour_boundary(now);
    let diff = (next - expected).num_seconds().abs();
    assert!(diff <= 2, "next={next} expected={expected} diff={diff}s");
    sched.shutdown().await.ok();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_then_shutdown_no_panic() {
    let sched = Scheduler::new(memory_pool().await, dev_config())
        .await
        .expect("scheduler");
    sched.register_all_jobs().await.expect("register");
    sched.start().await.expect("start");
    // 让调度循环跑一会儿，确认无 panic。
    tokio::time::sleep(Duration::from_millis(300)).await;
    sched.shutdown().await.expect("shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn panic_task_does_not_kill_scheduler() {
    let mut sched = JobScheduler::new().await.expect("scheduler");

    let counter = Arc::new(AtomicUsize::new(0));
    let c = counter.clone();
    let counting = Job::new_async_tz("1/1 * * * * * *", Utc, move |_uuid, _lock| {
        let c = c.clone();
        Box::pin(async move {
            c.fetch_add(1, Ordering::SeqCst);
        })
    })
    .expect("counting job");

    // 每秒触发一次但每次都 panic —— tokio-cron 把每个 job 放在独立 tokio::spawn 里跑，
    // panic 只终止该任务，调度循环与 counting 任务都必须继续。
    let panicking = Job::new_async_tz("1/1 * * * * * *", Utc, |_uuid, _lock| {
        Box::pin(async {
            panic!("intentional: panicking job must not crash the scheduler");
        })
    })
    .expect("panic job");

    sched.add(counting).await.expect("add counting");
    sched.add(panicking).await.expect("add panic");
    sched.start().await.expect("start");

    tokio::time::sleep(Duration::from_millis(3500)).await;
    let runs = counter.load(Ordering::SeqCst);
    assert!(
        runs >= 2,
        "healthy task should still run despite panicking sibling, runs={runs}"
    );

    sched.shutdown().await.expect("shutdown");
}
