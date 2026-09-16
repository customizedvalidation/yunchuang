//! Binary entrypoint: config -> db -> router -> serve on :8001.

use std::sync::Arc;

use tracing_subscriber::EnvFilter;

use metaclouds_backend_rust::auth::middleware::AppState;
use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    // Tracing: JSON output, filter from RUST_LOG or LOG_LEVEL.
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_target(true)
        .init();

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
    axum::serve(listener, app).await?;
    Ok(())
}
