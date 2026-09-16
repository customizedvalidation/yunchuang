//! Auth middleware: `jwt_auth` extracts + verifies the bearer/cookie token,
//! and `require_permission` enforces RBAC on top of the authenticated claims.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use tower_cookies::Cookies;

use crate::auth::jwt::{verify_token, Claims};
use crate::config::Config;
use crate::error::{AppError, AppResult};

/// Shared application state passed to every handler/middleware.
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub config: Arc<Config>,
}

/// All permission strings. Kept in sync with the Go v1 contract.
pub mod permissions {
    pub const CLUSTER_READ: &str = "cluster:read";
    pub const CLUSTER_WRITE: &str = "cluster:write";
    pub const RESOURCE_READ: &str = "resource:read";
    pub const RESOURCE_WRITE: &str = "resource:write";
    pub const JOB_READ: &str = "job:read";
    pub const JOB_WRITE: &str = "job:write";
    pub const JOB_SUBMIT: &str = "job:submit";
    pub const TENANT_READ: &str = "tenant:read";
    pub const TENANT_WRITE: &str = "tenant:write";
    pub const MONITORING_READ: &str = "monitoring:read";
    pub const MONITORING_WRITE: &str = "monitoring:write";
    pub const ACCELERATION_READ: &str = "acceleration:read";
    pub const ACCELERATION_WRITE: &str = "acceleration:write";
    pub const SECURITY_READ: &str = "security:read";
    pub const SECURITY_WRITE: &str = "security:write";
    pub const ADMIN: &str = "admin";
    pub const GPU_READ: &str = "gpu:read";
    pub const GPU_WRITE: &str = "gpu:write";
    pub const PARTITION_READ: &str = "partition:read";
    pub const PARTITION_WRITE: &str = "partition:write";
    pub const QUOTA_READ: &str = "quota:read";
    pub const QUOTA_WRITE: &str = "quota:write";
    pub const SCHEDULER_READ: &str = "scheduler:read";
    pub const SCHEDULER_WRITE: &str = "scheduler:write";
    pub const TOPOLOGY_READ: &str = "topology:read";
    pub const TOPOLOGY_WRITE: &str = "topology:write";
    pub const DATASET_READ: &str = "dataset:read";
    pub const DATASET_WRITE: &str = "dataset:write";
    pub const CHECKPOINT_READ: &str = "checkpoint:read";
    pub const CHECKPOINT_WRITE: &str = "checkpoint:write";
}

/// Does the given role hold the requested permission? `admin` short-circuits.
pub fn role_has_permission(role: &str, permission: &str) -> bool {
    if role == "admin" {
        return true;
    }
    match role {
        // Manager: read access across the platform plus job submit.
        "manager" => matches!(
            permission,
            permissions::CLUSTER_READ
                | permissions::RESOURCE_READ
                | permissions::JOB_READ
                | permissions::JOB_SUBMIT
                | permissions::TENANT_READ
                | permissions::MONITORING_READ
                | permissions::MONITORING_WRITE
                | permissions::ACCELERATION_READ
                | permissions::SECURITY_READ
                | permissions::GPU_READ
                | permissions::PARTITION_READ
                | permissions::QUOTA_READ
                | permissions::SCHEDULER_READ
                | permissions::TOPOLOGY_READ
                | permissions::DATASET_READ
                | permissions::CHECKPOINT_READ
        ),
        // Plain user: minimal read access to their own monitoring view.
        "user" => matches!(
            permission,
            permissions::JOB_READ
                | permissions::MONITORING_READ
                | permissions::DATASET_READ
                | permissions::CHECKPOINT_READ
        ),
        _ => false,
    }
}

/// 从请求中提取令牌：优先 `Authorization: Bearer {token}`，其次 `access_token` Cookie。
///
/// 错误消息对齐 Go `middlewares/jwt_auth.go`：
/// - 带了 Authorization 头但格式不是 `Bearer {token}` → 400；
/// - 既没有 Authorization 头也没有 access_token Cookie → 401。
fn extract_token(request: &Request, cookies: &Cookies) -> AppResult<String> {
    if let Some(value) = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if !value.is_empty() {
            let parts: Vec<&str> = value.splitn(2, ' ').collect();
            if parts.len() != 2 || !parts[0].eq_ignore_ascii_case("Bearer") {
                return Err(AppError::bad_request(
                    "Authorization header format must be Bearer {token}",
                ));
            }
            let token = parts[1].trim();
            if token.is_empty() {
                return Err(AppError::bad_request(
                    "Authorization header format must be Bearer {token}",
                ));
            }
            return Ok(token.to_string());
        }
    }

    // 无 Authorization 头：退回 httpOnly Cookie（浏览器 SPA 通道）。
    if let Some(cookie) = cookies.get("access_token") {
        if !cookie.value().is_empty() {
            return Ok(cookie.value().to_string());
        }
    }

    Err(AppError::unauthorized(
        "Authorization header or access_token cookie is required",
    ))
}

/// Middleware: extract + verify JWT from `Authorization: Bearer ...` or the
/// `access_token` cookie, then stash the claims in request extensions.
pub async fn jwt_auth(
    State(state): State<AppState>,
    cookies: Cookies,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let token = extract_token(&request, &cookies)?;

    let claims = verify_token(state.config.jwt_secret.as_bytes(), &token)?;
    tracing::debug!(user_id = claims.user_id, username = %claims.username, "authenticated");
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

/// Middleware factory: require the authenticated caller's role to hold the
/// given permission. Must be layered *after* `jwt_auth`.
pub async fn require_permission(
    State(permission): State<String>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| AppError::unauthorized("missing authentication context"))?;

    if !role_has_permission(&claims.role, &permission) {
        return Err(AppError::forbidden(format!(
            "role '{}' lacks permission '{}'",
            claims.role, permission
        )));
    }
    Ok(next.run(request).await)
}

/// Extractor: pull the verified `Claims` out of request extensions.
/// Requires `jwt_auth` middleware to have run first.
impl<S> axum::extract::FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("missing authentication context"))
    }
}
