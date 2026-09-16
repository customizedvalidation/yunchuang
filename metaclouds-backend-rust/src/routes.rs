//! Router assembly: public auth routes + protected user/admin routes.
//!
//! 对齐 Go `api/routes.go`：公开路由仅 login；受保护路由需 JWT，
//! User CRUD 额外需 admin 权限。中间件栈由 `middleware::apply_core_stack`
//! 统一应用（request_id → request_logger → timing → security_headers →
//! error_handler → panic_recover）。

use axum::routing::{get, post};
use axum::Router;
use tower_cookies::CookieManagerLayer;

use crate::auth::csrf::csrf_protect;
use crate::auth::handler::{get_csrf_token, get_profile, login, logout, refresh};
use crate::auth::middleware::{jwt_auth, permissions, require_permission, AppState};
use crate::handlers::user::{create_user, delete_user, get_user, list_users, update_user};
use crate::middleware::apply_core_stack;

/// Build the full application router. `state` is shared with all handlers.
pub fn build_router(state: AppState) -> Router {
    // Public routes (no JWT required).
    let public = Router::new().route("/auth/login", post(login));

    // Protected routes: JWT auth required.
    let auth_protected = Router::new()
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route("/auth/profile", get(get_profile))
        .route("/auth/csrf", get(get_csrf_token));

    // User CRUD: JWT + admin permission.
    let users = Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ADMIN.to_string(),
            require_permission,
        ));

    let protected = Router::new()
        .merge(auth_protected)
        .nest("/users", users)
        // CSRF 双提交校验：仅对携带 access_token Cookie 的写操作生效，Bearer 通道跳过
        .route_layer(axum::middleware::from_fn(csrf_protect))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    let api = Router::new()
        .nest("/api/v1", public.merge(protected))
        .layer(CookieManagerLayer::new());

    // 应用核心中间件栈（request_id/request_logger/timing/security_headers/error_handler/panic_recover）
    apply_core_stack(api).with_state(state)
}
