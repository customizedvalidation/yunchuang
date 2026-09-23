//! HTTP handlers for `/api/v1/jobs`（对齐 Go `controllers/job_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD / 状态机 / 取消 / 统计在 [`crate::services::job`]。
//! 路由上：列表/详情/统计需 JWT + `job:read`，创建/更新/删除/取消需 JWT + `job:write`。
//!
//! 调用方身份（tenant_id / user_id / is_admin）一律取自 JWT claims，不采信请求体
//! （对齐 Go `CreateJobForUser`：防止伪造租户/用户归属）。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::jwt::Claims;
use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::job::JobResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::job::{self, Actor, CreateJobInput, UpdateJobInput};

/// 从 JWT claims 构造服务层 Actor。
fn actor_from_claims(c: &Claims) -> Actor {
    Actor {
        tenant_id: c.tenant_id as i64,
        user_id: c.user_id as i64,
        is_admin: c.role == "admin",
    }
}

/// 分页 + 搜索 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct JobListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
    pub status: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub cluster_id: Option<i64>,
    pub user_id: Option<i64>,
}

/// `POST /api/v1/jobs` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateJobRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub gpus: Option<i64>,
    #[serde(default)]
    pub cpus: Option<i64>,
    #[serde(default)]
    pub memory: Option<i64>,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub cluster_id: Option<i64>,
}

/// `PUT /api/v1/jobs/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateJobRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub progress: Option<i64>,
    pub output_path: Option<String>,
    pub error_msg: Option<String>,
}

/// `GET /api/v1/jobs` — 分页列表（搜索 + status/type/cluster_id/user_id 过滤）。
#[utoipa::path(get,path="/api/v1/jobs",tag="jobs",responses((status=200,description="jobs",body=Vec<crate::models::job::JobResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_jobs(
    State(state): State<AppState>,
    claims: Claims,
    Query(q): Query<JobListQuery>,
) -> AppResult<Json<ApiResponse<Vec<JobResponse>>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = job::list_jobs(
        &state.pool,
        params,
        q.status.as_deref(),
        q.kind.as_deref(),
        q.cluster_id,
        q.user_id,
        q.search.as_deref(),
        actor_from_claims(&claims),
    )
    .await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/jobs/stats` — 按状态统计数量。
#[utoipa::path(get,path="/api/v1/jobs/stats",tag="jobs",responses((status=200,description="job stats",body=std::collections::HashMap<String,i64>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_job_stats(
    State(state): State<AppState>,
    claims: Claims,
) -> AppResult<Json<ApiResponse<std::collections::HashMap<String, i64>>>> {
    let stats = job::get_job_stats(&state.pool, actor_from_claims(&claims)).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// `GET /api/v1/jobs/:id` — 详情。
#[utoipa::path(get,path="/api/v1/jobs/{id}",tag="jobs",responses((status=200,description="job",body=crate::models::job::JobResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_job(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<JobResponse>>> {
    let j = job::get_job(&state.pool, id, actor_from_claims(&claims)).await?;
    Ok(Json(ApiResponse::success(j)))
}

/// `GET /api/v1/jobs/:id/status` — 作业运行态（真实 DB 聚合）。
///
/// 返回作业自身 DB 字段 + 所属集群名 + 已绑定的 GPU 分配列表。
/// `phase` 由作业状态语义派生（Running/Pending/Succeeded/Failed），并非 mock；
/// 真实 Pod 状态待对接 K8s API 后在本结构上扩展 `pods` 字段。
#[utoipa::path(get,path="/api/v1/jobs/{id}/status",tag="jobs",responses((status=200,description="job status",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_job_status(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let j = job::get_job(&state.pool, id, actor_from_claims(&claims)).await?;
    let phase = match j.status.as_str() {
        "running" => "Running",
        "pending" => "Pending",
        "completed" => "Succeeded",
        "failed" | "cancelled" => "Failed",
        _ => "Unknown",
    };

    // 所属集群名（cluster_id=0 表示未绑定，返回 null）。
    let cluster_name: Option<String> = if j.cluster_id > 0 {
        sqlx::query_scalar::<_, String>(
            "SELECT name FROM clusters WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(j.cluster_id)
        .fetch_optional(&state.pool)
        .await?
    } else {
        None
    };

    // 该作业绑定的 GPU 分配记录（含设备厂商/型号/设备状态）。
    // 列序：(device_id, allocation_status, vendor, model, device_status)
    let rows: Vec<(i64, String, String, String, String)> = sqlx::query_as(
        "SELECT a.device_id, a.status, d.vendor, d.model, d.status \
         FROM gpu_allocations a \
         LEFT JOIN gpu_devices d ON d.id = a.device_id \
         WHERE a.job_id = ? AND a.deleted_at IS NULL \
         ORDER BY a.id ASC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let gpu_allocations: Vec<serde_json::Value> = rows
        .into_iter()
        .map(
            |(device_id, allocation_status, vendor, model, device_status)| {
                serde_json::json!({
                    "gpu_device_id": device_id,
                    "vendor": vendor,
                    "model": model,
                    "allocation_status": allocation_status,
                    "device_status": device_status,
                })
            },
        )
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "job_id": j.id,
        "name": j.name,
        "status": j.status,
        "phase": phase,
        "created_at": j.created_at,
        "updated_at": j.updated_at,
        "cluster_id": j.cluster_id,
        "cluster_name": cluster_name,
        "gpu_requested": j.gpus,
        "gpu_allocations": gpu_allocations,
        "message": "real DB-backed job status",
    }))))
}

/// `POST /api/v1/jobs` — 创建（201）。
#[utoipa::path(post,path="/api/v1/jobs",request_body=CreateJobRequest,tag="jobs",responses((status=201,description="created",body=crate::models::job::JobResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_job(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateJobRequest>,
) -> AppResult<WithStatus<JobResponse>> {
    body.validate()?;
    let input = CreateJobInput {
        name: body.name,
        description: body.description,
        kind: body.kind,
        priority: body.priority.unwrap_or(0),
        gpus: body.gpus.unwrap_or(0),
        cpus: body.cpus.unwrap_or(0),
        memory: body.memory.unwrap_or(0),
        duration: body.duration.unwrap_or(0),
        cluster_id: body.cluster_id.unwrap_or(0),
    };
    let j = job::create_job(&state.pool, input, actor_from_claims(&claims)).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(j),
    })
}

/// `PUT /api/v1/jobs/:id` — 更新（状态机校验）。
#[utoipa::path(put,path="/api/v1/jobs/{id}",request_body=UpdateJobRequest,tag="jobs",responses((status=200,description="updated",body=crate::models::job::JobResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_job(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
    Json(body): Json<UpdateJobRequest>,
) -> AppResult<Json<ApiResponse<JobResponse>>> {
    body.validate()?;
    let input = UpdateJobInput {
        name: body.name,
        description: body.description,
        status: body.status,
        priority: body.priority,
        progress: body.progress,
        output_path: body.output_path,
        error_msg: body.error_msg,
    };
    let j = job::update_job(&state.pool, id, input, actor_from_claims(&claims)).await?;
    Ok(Json(ApiResponse::success(j)))
}

/// `DELETE /api/v1/jobs/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/jobs/{id}",tag="jobs",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_job(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    job::delete_job(&state.pool, id, actor_from_claims(&claims)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/jobs/:id/cancel` — 取消（仅 pending/running）。
#[utoipa::path(post,path="/api/v1/jobs/{id}/cancel",tag="jobs",responses((status=200,description="cancelled",body=crate::models::job::JobResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn cancel_job(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<JobResponse>>> {
    let j = job::cancel_job(&state.pool, id, actor_from_claims(&claims)).await?;
    Ok(Json(ApiResponse::success(j)))
}

/// `POST /api/v1/jobs/:id/submit` — 提交作业到 K8S（对齐前端 `submitToK8S`）。
///
/// 当前为 mock：校验作业存在后返回成功，不真正下发到 K8s 集群。
/// 真实下发待 K8s 执行器接入后替换实现。
#[utoipa::path(post,path="/api/v1/jobs/{id}/submit",tag="jobs",responses((status=200,description="submit accepted",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn submit_job_to_k8s(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    // 校验作业存在（兼做归属校验），不存在则 404。
    job::get_job(&state.pool, id, actor_from_claims(&claims)).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "submit accepted",
        "job_id": id,
        "cluster": "mock-k8s",
    }))))
}
