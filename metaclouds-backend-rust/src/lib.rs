//! Library crate: exposes every internal module plus a small test-friendly
//! entrypoint so `tests/` integration tests can build a router with an
//! in-memory SQLite database.

pub mod auth;
pub mod authz;
pub mod config;
pub mod db;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod orm;
pub mod response;
pub mod routes;
pub mod services;

use std::sync::Arc;
use std::time::Duration;

pub use crate::auth::middleware::AppState;
pub use crate::auth::password::{hash_password, verify_password};
pub use crate::db::seed_admin_if_empty;

/// Test-friendly config mirror. Production code should keep using
/// `config::Config::from_env()`.
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_expires_secs: u64,
    pub server_port: u16,
    pub server_host: String,
    pub log_level: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test-secret-at-least-32-characters-long-xxxx".to_string(),
            jwt_expires_secs: 3600,
            server_port: 8001,
            server_host: "127.0.0.1".to_string(),
            log_level: "warn".to_string(),
        }
    }
}

impl From<TestConfig> for crate::config::Config {
    fn from(t: TestConfig) -> Self {
        // 以 Config::default() 为底座，只覆盖 Phase 0 测试关心的六个字段；
        // 其余字段沿用配置层与 Go 版对齐的默认值。
        Self {
            server_host: t.server_host,
            server_port: t.server_port,
            database_url: t.database_url,
            jwt_secret: t.jwt_secret,
            jwt_expires: Duration::from_secs(t.jwt_expires_secs),
            log_level: t.log_level,
            ..Self::default()
        }
    }
}

/// Build the axum router against an already-connected pool.
///
/// Shared by `main.rs` (production) and integration tests (in-memory sqlite).
pub fn build_app(pool: sqlx::SqlitePool, config: impl Into<crate::config::Config>) -> axum::Router {
    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };
    routes::build_router(state)
}
