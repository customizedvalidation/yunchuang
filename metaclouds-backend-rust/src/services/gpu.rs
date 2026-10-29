//! GPU 设备与分配服务（对齐 Go `services/gpu_service.go`）。
//!
//! CRUD + 分页过滤 + allocate/release（联动设备显存与状态）+ 利用率汇总。
//! handler 只做参数提取与响应封装，所有 DB 操作在本层。

use std::collections::HashMap;

use crate::db::DatabasePool;
use chrono::Utc;

use crate::error::{AppError, AppResult};
use crate::models::gpu_allocation::{
    self, status as alloc_status, GpuAllocationFilter, GpuAllocationResponse,
};
use crate::models::gpu_device::{
    self, status as dev_status, GpuDevice, GpuDeviceFilter, GpuDeviceResponse, NewGpuDevice,
};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建 GPU 设备入参。
#[derive(Debug, Clone)]
pub struct CreateGpuDeviceInput {
    pub cluster_id: i64,
    pub node_name: String,
    pub vendor: String,
    pub model: String,
    pub index: i64,
    pub total_memory_gb: i64,
    pub allocatable_memory_gb: i64,
    pub used_memory_gb: i64,
    pub mig_enabled: bool,
    pub mig_profiles: String,
    pub driver_version: String,
    pub cuda_version: String,
    pub status: Option<String>,
    pub utilization: f64,
    pub temperature: i64,
    pub power_draw: i64,
    pub details: String,
}

/// 更新 GPU 设备入参（全部可选）。
#[derive(Debug, Clone, Default)]
pub struct UpdateGpuDeviceInput {
    pub node_name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub total_memory_gb: Option<i64>,
    pub allocatable_memory_gb: Option<i64>,
    pub used_memory_gb: Option<i64>,
    pub mig_enabled: Option<bool>,
    pub mig_profiles: Option<String>,
    pub driver_version: Option<String>,
    pub cuda_version: Option<String>,
    pub status: Option<String>,
    pub utilization: Option<f64>,
    pub temperature: Option<i64>,
    pub power_draw: Option<i64>,
    pub details: Option<String>,
}

/// 分配 GPU 入参（对应 Go `AllocateGPURequest`）。
#[derive(Debug, Clone)]
pub struct AllocateGpuInput {
    pub job_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub fraction: f64,
    pub memory_gb: i64,
    pub vendor: String,
}

/// 创建 GPU 设备。
pub async fn create_gpu_device(
    pool: &DatabasePool,
    input: CreateGpuDeviceInput,
) -> AppResult<GpuDeviceResponse> {
    let device = gpu_device::create(
        pool,
        NewGpuDevice {
            cluster_id: input.cluster_id,
            node_name: &input.node_name,
            vendor: &input.vendor,
            model: &input.model,
            index: input.index,
            total_memory_gb: input.total_memory_gb,
            allocatable_memory_gb: input.allocatable_memory_gb,
            used_memory_gb: input.used_memory_gb,
            mig_enabled: input.mig_enabled,
            mig_profiles: &input.mig_profiles,
            driver_version: &input.driver_version,
            cuda_version: &input.cuda_version,
            status: input.status.as_deref().unwrap_or(dev_status::AVAILABLE),
            utilization: input.utilization,
            temperature: input.temperature,
            power_draw: input.power_draw,
            details: &input.details,
        },
    )
    .await?;
    Ok(device.into())
}

/// GPU 设备详情。
pub async fn get_gpu_device(pool: &DatabasePool, id: i64) -> AppResult<GpuDeviceResponse> {
    let device = gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("GPU device not found"))?;
    Ok(device.into())
}

/// 分页 GPU 设备列表（按 cluster_id/vendor/status 过滤）。
pub async fn list_gpu_devices(
    pool: &DatabasePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    vendor: Option<&str>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<GpuDeviceResponse>> {
    let filter = GpuDeviceFilter {
        cluster_id,
        vendor,
        status,
    };
    let rows = gpu_device::list(pool, params, filter).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(GpuDeviceResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新 GPU 设备：仅覆盖传入字段。
pub async fn update_gpu_device(
    pool: &DatabasePool,
    id: i64,
    input: UpdateGpuDeviceInput,
) -> AppResult<GpuDeviceResponse> {
    gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("GPU device not found"))?;

    let now = Utc::now();
    crate::with_db!(
        pool,
        "UPDATE gpu_devices SET \
            node_name = COALESCE(?, node_name), \
            vendor = COALESCE(?, vendor), \
            model = COALESCE(?, model), \
            total_memory_gb = COALESCE(?, total_memory_gb), \
            allocatable_memory_gb = COALESCE(?, allocatable_memory_gb), \
            used_memory_gb = COALESCE(?, used_memory_gb), \
            mig_enabled = COALESCE(?, mig_enabled), \
            mig_profiles = COALESCE(?, mig_profiles), \
            driver_version = COALESCE(?, driver_version), \
            cuda_version = COALESCE(?, cuda_version), \
            status = COALESCE(?, status), \
            utilization = COALESCE(?, utilization), \
            temperature = COALESCE(?, temperature), \
            power_draw = COALESCE(?, power_draw), \
            details = COALESCE(?, details), \
            updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(&input.node_name)
                .bind(&input.vendor)
                .bind(&input.model)
                .bind(input.total_memory_gb)
                .bind(input.allocatable_memory_gb)
                .bind(input.used_memory_gb)
                .bind(input.mig_enabled)
                .bind(&input.mig_profiles)
                .bind(&input.driver_version)
                .bind(&input.cuda_version)
                .bind(&input.status)
                .bind(input.utilization)
                .bind(input.temperature)
                .bind(input.power_draw)
                .bind(&input.details)
                .bind(now)
                .bind(id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );

    let device = gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("gpu device disappeared after update"))?;
    Ok(device.into())
}

/// 软删除 GPU 设备。
pub async fn delete_gpu_device(pool: &DatabasePool, id: i64) -> AppResult<()> {
    let hit = gpu_device::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("GPU device not found"));
    }
    Ok(())
}

/// 分配 GPU：挑选满足 vendor/显存/fraction 容量的可用设备，创建 allocation 并联动设备状态。
///
/// 选机 + 批量算 fraction + INSERT allocation + UPDATE device 全部在同一事务内完成；
/// Postgres 用 `FOR UPDATE SKIP LOCKED` 锁住候选行，SQLite 库本身串行写。
/// 选机为 best-fit（剩余 fraction 最小但仍够本作业），设备显存用增量表达式更新。
pub async fn allocate_gpu(
    pool: &DatabasePool,
    input: AllocateGpuInput,
) -> AppResult<GpuAllocationResponse> {
    let now = Utc::now();
    // P1: 写操作事务化——候选 SELECT（PG 行锁）、批量 fraction、INSERT allocation、
    // UPDATE device 全部原子提交，消除 check-then-act 竞态与 N+1 往返。
    let alloc_id: i64 =
        crate::with_tx!(pool, |tx| {
            // 1) 拉候选设备。PG 分支带 vendor 下推 + FOR UPDATE SKIP LOCKED；
            //    SQLite 分支不加行锁（库串行写）。
            let candidates: Vec<GpuDevice> = if pool.is_sqlite() {
                crate::with_db_tx!(pool, &mut *tx,
                    "SELECT * FROM gpu_devices WHERE deleted_at IS NULL AND (status = 'available' OR status = 'allocated')",
                |db_s, db_e| {
                    crate::db::query_as_db(&mut *db_e, db_s).fetch_all(&mut *db_e).await?
                })
            } else {
                crate::with_db_tx!(pool, &mut *tx,
                    "SELECT * FROM gpu_devices WHERE deleted_at IS NULL AND (status = 'available' OR status = 'allocated') AND (vendor = ?1 OR ?1 = '') ORDER BY id FOR UPDATE SKIP LOCKED",
                |db_s, db_e| {
                    crate::db::query_as_db(&mut *db_e, db_s)
                        .bind(&input.vendor)
                        .fetch_all(&mut *db_e).await?
                })
            };

            // 2) Rust 侧再过滤 vendor + 显存（PG 已下推 vendor，此处幂等）。
            let prelim: Vec<&GpuDevice> = candidates.iter()
                .filter(|d| input.vendor.is_empty() || d.vendor == input.vendor)
                .filter(|d| d.allocatable_memory_gb - d.used_memory_gb >= input.memory_gb)
                .collect();
            let ids: Vec<i64> = prelim.iter().map(|d| d.id).collect();

            // 3) 一条批量查询拿到所有候选设备的 active fraction 之和（消除 N 次 SUM）。
            let frac_map: HashMap<i64, f64> = if ids.is_empty() {
                HashMap::new()
            } else {
                let placeholders = (1..=ids.len())
                    .map(|i| format!("?{i}"))
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT device_id, COALESCE(SUM(fraction), 0.0) AS used_fraction \
                     FROM gpu_allocations \
                     WHERE status = 'active' AND deleted_at IS NULL AND device_id IN ({placeholders}) \
                     GROUP BY device_id"
                );
                let rows: Vec<(i64, f64)> = crate::with_db_tx!(pool, &mut *tx, &sql, |db_s, db_e| {
                    let mut q = crate::db::query_as_db(&mut *db_e, db_s);
                    for id in &ids {
                        q = q.bind(id);
                    }
                    q.fetch_all(&mut *db_e).await?
                });
                rows.into_iter().collect()
            };

            // 4) best-fit：满足 fraction 约束的候选里挑"剩余 fraction 最小但仍够本作业"者。
            let mut best: Option<&GpuDevice> = None;
            let mut best_remaining: f64 = f64::MAX;
            for d in &prelim {
                let used_frac = frac_map.get(&d.id).copied().unwrap_or(0.0);
                let remaining = 1.0 - used_frac;
                if remaining + 0.001 < input.fraction {
                    continue;
                }
                if remaining < best_remaining {
                    best_remaining = remaining;
                    best = Some(d);
                }
            }
            let device = best.ok_or_else(|| {
                AppError::service_unavailable("no available GPU device matching criteria")
            })?;

            // 5) INSERT allocation。
            let aid = crate::insert_id_tx!(pool, &mut *tx,
            "INSERT INTO gpu_allocations (created_at, updated_at, device_id, job_id, tenant_id, \
             user_id, fraction, memory_gb, status, started_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', ?)",
        |q| q
            .bind(now)
            .bind(now)
            .bind(device.id)
            .bind(input.job_id)
            .bind(input.tenant_id)
            .bind(input.user_id)
            .bind(input.fraction)
            .bind(input.memory_gb)
            .bind(now)
        );

            // 6) 设备显存用增量表达式（并发安全）；分配后必然非空 → allocated。
            crate::with_db_tx!(pool, &mut *tx,
            "UPDATE gpu_devices SET used_memory_gb = used_memory_gb + ?1, status = 'allocated', updated_at = ?2 WHERE id = ?3",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(input.memory_gb)
                .bind(now)
                .bind(device.id)
                .execute(db_e)
                .await?
                .rows_affected()
        });
            aid
        });

    // 分配成功后主动失效 dashboard 统计缓存。
    let _ = crate::cache::global_cache()
        .delete("dashboard:stats")
        .await;

    let alloc = gpu_allocation::get_by_id(pool, alloc_id)
        .await?
        .ok_or_else(|| AppError::internal("allocation disappeared after insert"))?;
    Ok(alloc.into())
}

/// 释放 GPU 分配：标记 released，回写设备显存与状态。
///
/// 两条写操作（allocation 置 released + device 减显存）包在同一事务；
/// 设备显存用增量表达式 `MAX(used - n, 0)`，状态由 CASE 一句判定。
pub async fn release_gpu(pool: &DatabasePool, allocation_id: i64) -> AppResult<()> {
    let alloc = gpu_allocation::get_by_id(pool, allocation_id)
        .await?
        .ok_or_else(|| AppError::not_found("GPU allocation not found"))?;
    if alloc.status != alloc_status::ACTIVE {
        return Err(AppError::bad_request("allocation is not active"));
    }

    let now = Utc::now();
    let memory_gb = alloc.memory_gb;
    let device_id = alloc.device_id;
    crate::with_tx!(pool, |tx| {
        // 1) allocation 置 released（仅当仍 active，幂等）。
        crate::with_db_tx!(
            pool,
            &mut *tx,
            "UPDATE gpu_allocations SET status = 'released', ended_at = ?1, updated_at = ?2 WHERE id = ?3 AND status = 'active'",
            |db_s, db_e| {
                sqlx::query(db_s)
                    .bind(now)
                    .bind(now)
                    .bind(allocation_id)
                    .execute(db_e)
                    .await?
                    .rows_affected()
            }
        );
        // 2) 设备显存增量扣减，状态按剩余量一句判定。
        crate::with_db_tx!(
            pool,
            &mut *tx,
            "UPDATE gpu_devices SET used_memory_gb = MAX(used_memory_gb - ?1, 0), \
             status = CASE WHEN used_memory_gb - ?1 <= 0.001 THEN 'available' ELSE 'allocated' END, \
             updated_at = ?2 WHERE id = ?3",
            |db_s, db_e| {
                sqlx::query(db_s)
                    .bind(memory_gb)
                    .bind(memory_gb)
                    .bind(now)
                    .bind(device_id)
                    .execute(db_e)
                    .await?
                    .rows_affected()
            }
        );
    });

    // 释放成功后主动失效 dashboard 统计缓存。
    let _ = crate::cache::global_cache()
        .delete("dashboard:stats")
        .await;
    Ok(())
}

/// 分页分配记录列表（按 job_id/user_id/status 过滤）。
pub async fn list_allocations(
    pool: &DatabasePool,
    params: PaginationParams,
    job_id: Option<i64>,
    user_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<GpuAllocationResponse>> {
    let filter = GpuAllocationFilter {
        job_id,
        user_id,
        status,
    };
    let rows = gpu_allocation::list(pool, params, filter).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(GpuAllocationResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// GPU 利用率汇总（按 cluster_id 可选过滤）。
pub async fn get_gpu_utilization_summary(
    pool: &DatabasePool,
    cluster_id: Option<i64>,
) -> AppResult<HashMap<String, serde_json::Value>> {
    let rows: Vec<GpuDevice> = match cluster_id {
        Some(cid) => {
            crate::with_db!(
                pool,
                "SELECT * FROM gpu_devices WHERE deleted_at IS NULL AND cluster_id = ?",
                |db_s, db_e| {
                    crate::db::query_as_db(db_e, db_s)
                        .bind(cid)
                        .fetch_all(db_e)
                        .await?
                }
            )
        }
        None => {
            crate::with_db!(
                pool,
                "SELECT * FROM gpu_devices WHERE deleted_at IS NULL",
                |db_s, db_e| crate::db::query_as_db(db_e, db_s).fetch_all(db_e).await?
            )
        }
    };

    let mut total_devices: i64 = 0;
    let mut available_devices: i64 = 0;
    let mut allocated_devices: i64 = 0;
    let mut maintenance_devices: i64 = 0;
    let mut fault_devices: i64 = 0;
    let mut total_memory_gb: f64 = 0.0;
    let mut used_memory_gb: f64 = 0.0;
    let mut total_utilization: f64 = 0.0;
    let mut vendor_stats: HashMap<String, i64> = HashMap::new();

    for d in &rows {
        total_devices += 1;
        total_memory_gb += d.total_memory_gb as f64;
        used_memory_gb += d.used_memory_gb as f64;
        total_utilization += d.utilization;
        *vendor_stats.entry(d.vendor.clone()).or_insert(0) += 1;
        match d.status.as_str() {
            "available" => available_devices += 1,
            "allocated" => allocated_devices += 1,
            "maintenance" => maintenance_devices += 1,
            "fault" => fault_devices += 1,
            _ => {}
        }
    }

    let avg_utilization = if total_devices > 0 {
        total_utilization / total_devices as f64
    } else {
        0.0
    };

    let mut out: HashMap<String, serde_json::Value> = HashMap::new();
    out.insert("total_devices".into(), serde_json::json!(total_devices));
    out.insert(
        "available_devices".into(),
        serde_json::json!(available_devices),
    );
    out.insert(
        "allocated_devices".into(),
        serde_json::json!(allocated_devices),
    );
    out.insert(
        "maintenance_devices".into(),
        serde_json::json!(maintenance_devices),
    );
    out.insert("fault_devices".into(), serde_json::json!(fault_devices));
    out.insert("total_memory_gb".into(), serde_json::json!(total_memory_gb));
    out.insert("used_memory_gb".into(), serde_json::json!(used_memory_gb));
    out.insert(
        "avg_utilization".into(),
        serde_json::json!(format!("{avg_utilization:.2}")),
    );
    out.insert(
        "vendor_stats".into(),
        serde_json::to_value(&vendor_stats).map_err(|e| {
            AppError::with_source(
                crate::error::ErrorCode::InternalServerError,
                "json encode error",
                e,
            )
        })?,
    );
    Ok(out)
}
