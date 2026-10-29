//! Monitoring 服务。
//!
//! 聚合 13 个业务指标（对齐 Go metrics_business.go）+ 16 条告警规则定义。
//!
//! 13 指标：total_users / active_users / total_tenants / total_clusters / total_resources /
//! total_jobs / running_jobs / total_gpus / allocated_gpus / total_datasets / total_alerts /
//! active_alerts / system_uptime。
//!
//! 16 告警规则见 [`list_alert_rules`]。

use std::sync::OnceLock;
use std::time::Duration;

use crate::db::DatabasePool;
use serde::Serialize;

use crate::error::AppResult;

/// 服务启动时刻（用于 system_uptime）。
static START_TIME: OnceLock<std::time::Instant> = OnceLock::new();

fn start_time() -> std::time::Instant {
    *START_TIME.get_or_init(std::time::Instant::now)
}

/// 安全 COUNT：查询失败（表不存在等）返回 0，不中断 dashboard。
async fn safe_count(pool: &DatabasePool, sql: &str) -> i64 {
    crate::with_db!(pool, sql, |db_s, db_e| {
        crate::db::query_scalar_db(db_e, db_s)
            .fetch_one(db_e)
            .await
            .unwrap_or(0)
    })
}

/// 聚合业务指标（对齐 Go metrics_business.go）。
///
/// 外层包 10s 进程内 Redis 缓存（key `metaclouds:dashboard:stats`）；
/// 未命中或 Noop 时回源 [`compute_dashboard_stats`]。GPU 分配/释放成功后会主动失效该 key。
pub async fn get_dashboard_stats(pool: &DatabasePool) -> AppResult<serde_json::Value> {
    let cache = crate::cache::global_cache();
    let key = "dashboard:stats";
    let ttl = Duration::from_secs(10);
    let compute: std::pin::Pin<
        Box<dyn std::future::Future<Output = AppResult<Vec<u8>>> + Send + '_>,
    > = Box::pin(async move {
        let v = compute_dashboard_stats(pool).await?;
        serde_json::to_vec(&v).map_err(|e| {
            crate::error::AppError::with_source(
                crate::error::ErrorCode::InternalServerError,
                "dashboard stats json encode",
                e,
            )
        })
    });
    let bytes = cache.get_or_set(key, ttl, compute).await?;
    // 缓存内容损坏时回源重算（不把 500 抛给 dashboard）。
    match serde_json::from_slice(&bytes) {
        Ok(v) => Ok(v),
        Err(_) => compute_dashboard_stats(pool).await,
    }
}

/// 真正聚合指标：12 条 COUNT 并发发出，结果按原字段组装。
async fn compute_dashboard_stats(pool: &DatabasePool) -> AppResult<serde_json::Value> {
    // 12 条 COUNT 并发执行（SQLite WAL 下由连接池排队，Postgres 并行）。
    let futs = vec![
        safe_count(pool, "SELECT COUNT(*) FROM users"),
        safe_count(pool, "SELECT COUNT(*) FROM tenants WHERE deleted_at IS NULL"),
        safe_count(pool, "SELECT COUNT(*) FROM clusters WHERE deleted_at IS NULL"),
        safe_count(pool, "SELECT COUNT(*) FROM resources WHERE deleted_at IS NULL"),
        safe_count(pool, "SELECT COUNT(*) FROM jobs WHERE deleted_at IS NULL"),
        safe_count(
            pool,
            "SELECT COUNT(*) FROM jobs WHERE deleted_at IS NULL AND status = 'running'",
        ),
        safe_count(
            pool,
            "SELECT COUNT(*) FROM jobs WHERE deleted_at IS NULL AND status = 'pending'",
        ),
        safe_count(pool, "SELECT COUNT(*) FROM gpu_devices WHERE deleted_at IS NULL"),
        safe_count(
            pool,
            "SELECT COUNT(*) FROM gpu_allocations WHERE deleted_at IS NULL",
        ),
        safe_count(
            pool,
            "SELECT COUNT(*) FROM datasets WHERE deleted_at IS NULL",
        ),
        safe_count(pool, "SELECT COUNT(*) FROM alerts WHERE deleted_at IS NULL"),
        safe_count(
            pool,
            "SELECT COUNT(*) FROM alerts WHERE deleted_at IS NULL AND status = 'active'",
        ),
    ];
    let r = futures_util::future::join_all(futs).await;
    let total_users = r[0];
    let total_tenants = r[1];
    let total_clusters = r[2];
    let total_resources = r[3];
    let total_jobs = r[4];
    let running_jobs = r[5];
    let pending_jobs = r[6];
    let total_gpus = r[7];
    let allocated_gpus = r[8];
    let total_datasets = r[9];
    let total_alerts = r[10];
    let active_alerts = r[11];
    // active_users：当前 schema 无登录表，退而求其次用 total_users 近似。
    let active_users = total_users;
    let uptime_secs: i64 = start_time().elapsed().as_secs() as i64;

    Ok(serde_json::json!({
        "total_users": total_users,
        "active_users": active_users,
        "total_tenants": total_tenants,
        "total_clusters": total_clusters,
        "total_resources": total_resources,
        "total_jobs": total_jobs,
        "running_jobs": running_jobs,
        "pending_jobs": pending_jobs,
        "total_gpus": total_gpus,
        "allocated_gpus": allocated_gpus,
        "total_datasets": total_datasets,
        "total_alerts": total_alerts,
        "active_alerts": active_alerts,
        "system_uptime": uptime_secs,
    }))
}

/// 按指标名查询单个指标（不存在返回 null）。
pub async fn get_metric(pool: &DatabasePool, name: &str) -> AppResult<serde_json::Value> {
    let all = get_dashboard_stats(pool).await?;
    let v = all.get(name).cloned().unwrap_or(serde_json::Value::Null);
    Ok(serde_json::json!({
        "name": name,
        "value": v,
    }))
}

/// 告警规则定义（对齐任务规格 16 条）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize)]
pub struct AlertRuleDef {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub category: String,
    pub threshold: f64,
    pub operator: String,
    pub severity: String,
    pub enabled: bool,
}

/// 返回 16 条告警规则定义。
pub fn list_alert_rules() -> Vec<AlertRuleDef> {
    vec![
        AlertRuleDef {
            id: 1,
            name: "GPU High Utilization".into(),
            description: "Trigger when GPU utilization exceeds 90%".into(),
            category: "gpu".into(),
            threshold: 90.0,
            operator: ">".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 2,
            name: "GPU Memory High".into(),
            description: "Trigger when GPU memory usage exceeds 90%".into(),
            category: "gpu".into(),
            threshold: 90.0,
            operator: ">".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 3,
            name: "CPU High Utilization".into(),
            description: "Trigger when CPU utilization exceeds 85%".into(),
            category: "cpu".into(),
            threshold: 85.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 4,
            name: "Memory High".into(),
            description: "Trigger when memory usage exceeds 85%".into(),
            category: "memory".into(),
            threshold: 85.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 5,
            name: "Disk High Usage".into(),
            description: "Trigger when disk usage exceeds 90%".into(),
            category: "storage".into(),
            threshold: 90.0,
            operator: ">".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 6,
            name: "Job Failure Rate High".into(),
            description: "Trigger when job failure rate exceeds 10%".into(),
            category: "job".into(),
            threshold: 10.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 7,
            name: "Job Queue Too Long".into(),
            description: "Trigger when a job queues for more than 30 minutes".into(),
            category: "job".into(),
            threshold: 1800.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 8,
            name: "Job Runtime Too Long".into(),
            description: "Trigger when a running job exceeds 24 hours".into(),
            category: "job".into(),
            threshold: 86400.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 9,
            name: "Cluster Offline".into(),
            description: "Trigger when a cluster goes offline".into(),
            category: "cluster".into(),
            threshold: 0.0,
            operator: "==".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 10,
            name: "Node Offline".into(),
            description: "Trigger when a node goes offline".into(),
            category: "cluster".into(),
            threshold: 0.0,
            operator: "==".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 11,
            name: "GPU Device Failure".into(),
            description: "Trigger when a GPU device reports an error".into(),
            category: "gpu".into(),
            threshold: 0.0,
            operator: "==".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 12,
            name: "Resource Quota Near Limit".into(),
            description: "Trigger when resource quota usage exceeds 90%".into(),
            category: "quota".into(),
            threshold: 90.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 13,
            name: "Tenant Quota Exceeded".into(),
            description: "Trigger when a tenant exceeds its quota".into(),
            category: "quota".into(),
            threshold: 100.0,
            operator: ">=".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 14,
            name: "Security Policy Violation".into(),
            description: "Trigger when a security policy violation is detected".into(),
            category: "security".into(),
            threshold: 0.0,
            operator: ">".into(),
            severity: "critical".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 15,
            name: "Login Failure Threshold".into(),
            description: "Trigger when login failures exceed 5 times in 5 minutes".into(),
            category: "security".into(),
            threshold: 5.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
        AlertRuleDef {
            id: 16,
            name: "API Error Rate High".into(),
            description: "Trigger when API error rate exceeds 5%".into(),
            category: "system".into(),
            threshold: 5.0,
            operator: ">".into(),
            severity: "warning".into(),
            enabled: true,
        },
    ]
}

/// 评估告警规则（当前基于 dashboard 指标做静态评估，返回被触发的规则列表）。
pub async fn evaluate_alert_rules(pool: &DatabasePool) -> AppResult<Vec<serde_json::Value>> {
    let stats = get_dashboard_stats(pool).await?;
    let rules = list_alert_rules();
    let mut triggered = Vec::new();

    // 简单评估：基于已用 GPU 比例判断可推导的规则；其余分类等待数据接入。
    let total_gpus = stats
        .get("total_gpus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let allocated_gpus = stats
        .get("allocated_gpus")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let gpu_alloc_pct = if total_gpus > 0 {
        allocated_gpus as f64 / total_gpus as f64 * 100.0
    } else {
        0.0
    };

    for rule in &rules {
        if !rule.enabled {
            continue;
        }
        // 只对可从 dashboard 推导的规则做实际评估；其余规则返回定义但不判定触发。
        let fire = match rule.category.as_str() {
            "gpu" if rule.id == 1 => gpu_alloc_pct > 90.0,
            "gpu" if rule.id == 2 => gpu_alloc_pct > 90.0,
            "gpu" if rule.id == 11 => false, // 无设备故障信号源
            "cluster" => false,
            "job" => false,
            "quota" => false,
            "security" => false,
            "system" => false,
            "cpu" | "memory" | "storage" => false,
            _ => false,
        };
        // 仅当规则自身阈值判定触发才上报；历史实现把"任意 active 告警"误当成全部规则触发。
        if fire {
            triggered.push(serde_json::json!({
                "rule_id": rule.id,
                "rule_name": rule.name,
                "severity": rule.severity,
                "current_value": gpu_alloc_pct,
                "threshold": rule.threshold,
                "operator": rule.operator,
            }));
        }
    }
    let _ = Duration::from_secs(0); // suppress unused import warning if any
    Ok(triggered)
}
