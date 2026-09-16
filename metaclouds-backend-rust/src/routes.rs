//! Router assembly: public auth routes + protected user/admin routes.

use axum::routing::{get, post};
use axum::Router;
use tower_cookies::CookieManagerLayer;

use crate::auth::handler::{get_profile, login, logout};
use crate::auth::middleware::{jwt_auth, permissions, require_permission, AppState};
use crate::handlers::user::{create_user, delete_user, get_user, list_users, update_user};
use crate::middleware::panic_recover::catch_panics;
use crate::middleware::request_id::set_request_id;

/// Build the full application router. `state` is shared with all handlers.
pub fn build_router(state: AppState) -> Router {
    // Public routes (no JWT required).
    let public = Router::new()
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout));

    // Protected routes: JWT auth + RBAC on each endpoint.
    let users = Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ADMIN.to_string(),
            require_permission,
        ));

    let protected = Router::new()
        .route("/auth/profile", get(get_profile))
        .nest("/users", users)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    Router::new()
        .nest("/api/v1", public.merge(protected))
        .layer(CookieManagerLayer::new())
        .layer(axum::middleware::from_fn(set_request_id))
        .layer(axum::middleware::from_fn(catch_panics))
        .with_state(state)
}
