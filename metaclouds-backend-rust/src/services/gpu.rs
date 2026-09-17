//! GPU 设备与分配服务（对齐 Go `services/gpu_service.go`）。
//!
//! CRUD + 分页过滤 + allocate/release（联动设备显存与状态）+ 利用率汇总。
//! handler 只做参数提取与响应封装，所有 DB 操作在本层。

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;

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
    pool: &SqlitePool,
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
pub async fn get_gpu_device(pool: &SqlitePool, id: i64) -> AppResult<GpuDeviceResponse> {
    let device = gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("GPU device not found"))?;
    Ok(device.into())
}

/// 分页 GPU 设备列表（按 cluster_id/vendor/status 过滤）。
pub async fn list_gpu_devices(
    pool: &SqlitePool,
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
    pool: &SqlitePool,
    id: i64,
    input: UpdateGpuDeviceInput,
) -> AppResult<GpuDeviceResponse> {
    gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("GPU device not found"))?;

    let now = Utc::now();
    sqlx::query(
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
    )
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
    .execute(pool)
    .await?;

    let device = gpu_device::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("gpu device disappeared after update"))?;
    Ok(device.into())
}

/// 软删除 GPU 设备。
pub async fn delete_gpu_device(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = gpu_device::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("GPU device not found"));
    }
    Ok(())
}

/// 分配 GPU：挑选满足 vendor/显存/fraction 容量的可用设备，创建 allocation 并联动设备状态。
pub async fn allocate_gpu(
    pool: &SqlitePool,
    input: AllocateGpuInput,
) -> AppResult<GpuAllocationResponse> {
    // 查找可用设备：status available/allocated，vendor 匹配，显存充足，fraction 未超限。
    let candidates: Vec<GpuDevice> = sqlx::query_as(
        "SELECT * FROM gpu_devices WHERE deleted_at IS NULL AND (status = 'available' OR status = 'allocated')",
    )
    .fetch_all(pool)
    .await?;

    let mut selected: Option<GpuDevice> = None;
    for d in candidates {
        if !input.vendor.is_empty() && d.vendor != input.vendor {
            continue;
        }
        let available_mem = d.allocatable_memory_gb - d.used_memory_gb;
        if available_mem < input.memory_gb {
            continue;
        }
        // 累计已分配 fraction（active）。
        let used_fraction: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(fraction), 0.0) FROM gpu_allocations WHERE device_id = ? AND status = 'active'",
        )
        .bind(d.id)
        .fetch_one(pool)
        .await?;
        if used_fraction + input.fraction > 1.0 + 0.001 {
            continue;
        }
        selected = Some(d);
        break;
    }

    let device = selected.ok_or_else(|| {
        AppError::service_unavailable("no available GPU device matching criteria")
    })?;

    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO gpu_allocations (created_at, updated_at, device_id, job_id, tenant_id, \
         user_id, fraction, memory_gb, status, started_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'active', ?)",
    )
    .bind(now)
    .bind(now)
    .bind(device.id)
    .bind(input.job_id)
    .bind(input.tenant_id)
    .bind(input.user_id)
    .bind(input.fraction)
    .bind(input.memory_gb)
    .bind(now)
    .execute(pool)
    .await?;

    let alloc_id = res.last_insert_rowid();

    // 联动设备已用显存与状态。
    let new_used = device.used_memory_gb + input.memory_gb;
    let new_status = if new_used > 0 {
        dev_status::ALLOCATED
    } else {
        dev_status::AVAILABLE
    };
    sqlx::query(
        "UPDATE gpu_devices SET used_memory_gb = ?, status = ?, updated_at = ? WHERE id = ?",
    )
    .bind(new_used)
    .bind(new_status)
    .bind(now)
    .bind(device.id)
    .execute(pool)
    .await?;

    let alloc = gpu_allocation::get_by_id(pool, alloc_id)
        .await?
        .ok_or_else(|| AppError::internal("allocation disappeared after insert"))?;
    Ok(alloc.into())
}

/// 释放 GPU 分配：标记 released，回写设备显存与状态。
pub async fn release_gpu(pool: &SqlitePool, allocation_id: i64) -> AppResult<()> {
    let alloc = gpu_allocation::get_by_id(pool, allocation_id)
        .await?
        .ok_or_else(|| AppError::not_found("GPU allocation not found"))?;
    if alloc.status != alloc_status::ACTIVE {
        return Err(AppError::bad_request("allocation is not active"));
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE gpu_allocations SET status = 'released', ended_at = ?, updated_at = ? WHERE id = ?",
    )
    .bind(now)
    .bind(now)
    .bind(allocation_id)
    .execute(pool)
    .await?;

    // 回写设备显存。
    let device = gpu_device::get_by_id(pool, alloc.device_id, false).await?;
    if let Some(d) = device {
        let new_used = (d.used_memory_gb - alloc.memory_gb).max(0);
        let new_status = if new_used == 0 {
            dev_status::AVAILABLE.to_string()
        } else {
            d.status.clone()
        };
        sqlx::query(
            "UPDATE gpu_devices SET used_memory_gb = ?, status = ?, updated_at = ? WHERE id = ?",
        )
        .bind(new_used)
        .bind(&new_status)
        .bind(now)
        .bind(d.id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// 分页分配记录列表（按 job_id/user_id/status 过滤）。
pub async fn list_allocations(
    pool: &SqlitePool,
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
    pool: &SqlitePool,
    cluster_id: Option<i64>,
) -> AppResult<HashMap<String, serde_json::Value>> {
    let rows: Vec<GpuDevice> = match cluster_id {
        Some(cid) => {
            sqlx::query_as("SELECT * FROM gpu_devices WHERE deleted_at IS NULL AND cluster_id = ?")
                .bind(cid)
                .fetch_all(pool)
                .await?
        }
        None => {
            sqlx::query_as("SELECT * FROM gpu_devices WHERE deleted_at IS NULL")
                .fetch_all(pool)
                .await?
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
