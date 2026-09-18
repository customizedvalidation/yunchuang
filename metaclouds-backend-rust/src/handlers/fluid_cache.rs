//! HTTP handlers for FluidCache（对齐 Go `controllers/dataset_controller.go` 的 FluidCache 面）。
//!
//! ⚠️ 路由形状与 Go 逐字对齐：FluidCache 是嵌套在 datasets 下的子资源，**不是**独立顶层 CRUD。
//! Go 侧实际路由（`api/routes.go`）：
//! - `GET  /datasets/:id/caches`          （列出某 dataset 的缓存，JWT）
//! - `POST /datasets/:id/caches`          （创建，dataset:write）
//! - `GET  /datasets/:id/fluid-caches`    （Vue 别名，同上）
//! - `POST /datasets/:id/fluid-caches`    （Vue 别名，同上）
//! - `PUT    /fluid-caches/:cacheId`      （更新，dataset:write）
//! - `DELETE /fluid-caches/:cacheId`      （软删除，dataset:write）
//! - `POST   /fluid-caches/:cacheId/enable|disable|prefetch`（dataset:write）
//!
//! Go 版**没有**顶层 `GET /fluid-caches`（全量列表）、`GET /fluid-caches/:id`（详情）、
//! `POST /fluid-caches`（顶层创建），故 Rust 侧不补这三条（见交付差异清单）。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::fluid_cache::FluidCacheResponse;
use crate::orm::PaginationParams;
use crate::response::ApiResponse;
use crate::services::fluid_cache as service;

/// 列表查询参数（分页）。
#[derive(Debug, Deserialize)]
pub struct FluidCacheListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// `POST /datasets/:id/caches` 创建请求体（dataset_id 取自路径）。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateFluidCacheRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub cache_class: Option<String>,
    #[serde(default)]
    pub replicas: Option<i32>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `PUT /fluid-caches/:cacheId` 更新请求体（全部可选）。
#[derive(Debug, Deserialize, Default)]
pub struct UpdateFluidCacheRequest {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub path: Option<String>,
    pub cache_class: Option<String>,
    pub replicas: Option<i32>,
    pub status: Option<String>,
}

/// `GET /datasets/:id/caches` — 列出某 dataset 的缓存（JWT）。
pub async fn list_fluid_caches(
    State(state): State<AppState>,
    Path(dataset_id): Path<i64>,
    Query(q): Query<FluidCacheListQuery>,
) -> AppResult<Json<ApiResponse<Vec<FluidCacheResponse>>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = service::list_fluid_caches(&state.pool, params, Some(dataset_id)).await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `POST /datasets/:id/caches` — 创建（201）。
pub async fn create_fluid_cache(
    State(state): State<AppState>,
    Path(dataset_id): Path<i64>,
    Json(body): Json<CreateFluidCacheRequest>,
) -> AppResult<(StatusCode, Json<ApiResponse<FluidCacheResponse>>)> {
    body.validate()?;
    let input = service::CreateFluidCacheInput {
        name: body.name,
        dataset_id,
        namespace: body.namespace.unwrap_or_else(|| "default".to_string()),
        path: body.path.unwrap_or_else(|| "/var/lib/fluid".to_string()),
        cache_class: body
            .cache_class
            .unwrap_or_else(|| "read-through".to_string()),
        replicas: body.replicas.unwrap_or(1),
        status: body.status,
    };
    let cache = service::create_fluid_cache(&state.pool, input).await?;
    Ok((StatusCode::CREATED, Json(ApiResponse::success(cache))))
}

/// `PUT /fluid-caches/:cacheId` — 更新。
pub async fn update_fluid_cache(
    State(state): State<AppState>,
    Path(cache_id): Path<i64>,
    Json(body): Json<UpdateFluidCacheRequest>,
) -> AppResult<Json<ApiResponse<FluidCacheResponse>>> {
    let input = service::UpdateFluidCacheInput {
        name: body.name,
        namespace: body.namespace,
        path: body.path,
        cache_class: body.cache_class,
        replicas: body.replicas,
        status: body.status,
    };
    let cache = service::update_fluid_cache(&state.pool, cache_id, input).await?;
    Ok(Json(ApiResponse::success(cache)))
}

/// `DELETE /fluid-caches/:cacheId` — 软删除，204。
pub async fn delete_fluid_cache(
    State(state): State<AppState>,
    Path(cache_id): Path<i64>,
) -> AppResult<StatusCode> {
    service::delete_fluid_cache(&state.pool, cache_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /fluid-caches/:cacheId/enable` — 启用（status→active）。
pub async fn enable_fluid_cache(
    State(state): State<AppState>,
    Path(cache_id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let cache = service::set_status(&state.pool, cache_id, "active").await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "fluid cache enabled",
        "cache": cache,
    }))))
}

/// `POST /fluid-caches/:cacheId/disable` — 停用（status→inactive）。
pub async fn disable_fluid_cache(
    State(state): State<AppState>,
    Path(cache_id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let cache = service::set_status(&state.pool, cache_id, "inactive").await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "fluid cache disabled",
        "cache": cache,
    }))))
}

/// `POST /fluid-caches/:cacheId/prefetch` — 触发预取（占位）。
pub async fn prefetch_fluid_cache(
    State(state): State<AppState>,
    Path(cache_id): Path<i64>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    service::trigger_prefetch(&state.pool, cache_id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "message": "prefetch triggered",
    }))))
}
