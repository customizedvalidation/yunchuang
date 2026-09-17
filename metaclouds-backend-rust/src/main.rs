//! Binary entrypoint: config -> db -> router -> serve on :8001.

use std::sync::Arc;

use metaclouds_backend_rust::auth::middleware::AppState;
use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::db;
use metaclouds_backend_rust::tracing::{init_tracing, shutdown_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    // Tracing: JSON structured logs (LOG_FORMAT=pretty 可切换本地美化输出)，
    // 级别由 RUST_LOG 或 LOG_LEVEL 控制；OTel 默认关闭。
    init_tracing(&config)?;

    tracing::info!(
        port = config.server_port,
        "starting metaclouds-backend-rust"
    );

    let pool = db::connect_and_migrate(&config.database_url).await?;
    db::seed_admin_if_empty(&pool).await?;

    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
    };
    let app = metaclouds_backend_rust::routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;
    tracing::info!(addr = %config.bind_addr(), "listening");

    // 优雅关闭：收到 Ctrl-C / SIGTERM 后停止接新连接，再 flush trace。
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutdown signal received, stopping server");
        })
        .await?;

    shutdown_tracing();
    tracing::info!("server exited gracefully");
    Ok(())
}
