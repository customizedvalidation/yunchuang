//! Binary entrypoint: config -> db -> router -> serve on :8001.

use std::sync::Arc;

use metaclouds_backend_rust::auth::middleware::AppState;
use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::db::{self, DatabasePool};
use metaclouds_backend_rust::error::{AppError, ErrorCode};
use metaclouds_backend_rust::tracing::{init_tracing, shutdown_tracing};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 记录 /health uptime 基准时刻（尽早，贴近进程启动）。
    metaclouds_backend_rust::handlers::health::init_start();

    let config = Config::from_env()?;

    // Tracing: JSON structured logs (LOG_FORMAT=pretty 可切换本地美化输出)，
    // 级别由 RUST_LOG 或 LOG_LEVEL 控制；OTel 默认关闭。
    init_tracing(&config)?;

    tracing::info!(
        port = config.server_port,
        "starting metaclouds-backend-rust"
    );

    // 按配置选择连接驱动：
    // - 默认（USE_SQLITE=true / sqlite url）：与 Phase 0 行为完全一致。
    // - USE_SQLITE=false 或 DATABASE_URL=postgres(s)://：真正连接 Postgres，
    //   跑 migrations/postgres 方言迁移并播种 admin（此前 connect_pool /
    //   DatabasePool::Postgres 是死代码，compose 下发 postgresql:// 时被静默
    //   忽略、错误地按 SQLite 打开了一个同名文件）。
    let pool: DatabasePool = if db::wants_postgres(&config) {
        let db_pool = db::connect_pool(&config).await?;
        db::run_migrations(&db_pool).await.map_err(|e| {
            AppError::with_source(
                ErrorCode::InternalServerError,
                "database migration failed",
                e,
            )
        })?;
        match &db_pool {
            DatabasePool::Postgres(pg) => db::seed_admin_if_empty_postgres(pg).await?,
            DatabasePool::Sqlite(p) => db::seed_admin_if_empty(p).await?,
        }
        // 请求层已移植到双驱动方言（?N ⇄ $N、last_insert_rowid ⇄ RETURNING id），
        // Postgres 与 SQLite 共用同一套 SQL，此处可直接以 DatabasePool 对外服务。
        db_pool
    } else {
        let sqlite_pool = db::connect_and_migrate(&config.database_url).await?;
        db::seed_admin_if_empty(&sqlite_pool).await?;
        DatabasePool::Sqlite(sqlite_pool)
    };

    // ── Redis 缓存/会话：按配置建立，失败自动降级为 NoopCache ──────────
    // 启用后 JWT 登出即刻生效（jti 写入撤销名单，见 cache::session）。
    // 未启用或连不上时不阻塞启动，功能退化为"令牌自然过期"。
    let cache = metaclouds_backend_rust::cache::build_cache(&config).await;
    if cache.is_noop() {
        tracing::warn!(
            redis_enabled = config.redis_enabled,
            "running without redis: token revocation on logout is degraded to token expiry"
        );
    }
    metaclouds_backend_rust::cache::install_cache(cache);

    // ── cron 调度器：按配置注册并启动 ──────────────────────────────────
    // 此前 `scheduler::Scheduler` 只在单测里被构造，生产进程从不启动它，
    // 定时任务（采样训练/推理）实际从未跑过。这里接线到进程生命周期：
    // 注册默认任务 → start() → 优雅关闭时 shutdown()。
    let scheduler = if config.scheduler_enabled {
        match metaclouds_backend_rust::scheduler::Scheduler::new(
            pool.clone(),
            Arc::new(config.clone()),
        )
        .await
        {
            Ok(s) => {
                if let Err(e) = s.register_all_jobs().await {
                    tracing::error!(error = %e, "failed to register cron jobs");
                }
                if let Err(e) = s.start().await {
                    tracing::error!(error = %e, "failed to start cron scheduler");
                } else {
                    tracing::info!("cron scheduler started");
                }
                Some(s)
            }
            Err(e) => {
                tracing::error!(error = %e, "failed to create cron scheduler");
                None
            }
        }
    } else {
        tracing::info!("scheduler disabled via config");
        None
    };

    let state = AppState {
        pool,
        config: Arc::new(config.clone()),
    };
    let app = metaclouds_backend_rust::routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr()).await?;
    tracing::info!(addr = %config.bind_addr(), "listening");

    // 优雅关闭：收到 Ctrl-C / SIGTERM 后停止接新连接，再 flush trace。
    //
    // `into_make_service_with_connect_info::<SocketAddr>()` 把 TCP 对端地址注入
    // 请求扩展，限流与访问日志据此判断 `X-Forwarded-For` 是否可采信
    // （否则任何人伪造该头即可绕过限流、污染审计日志）。
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutdown signal received, stopping server");
        // 先停调度器（不再触发新任务），再停 HTTP。
        if let Some(s) = &scheduler {
            if let Err(e) = s.shutdown().await {
                tracing::warn!(error = %e, "cron scheduler shutdown failed");
            } else {
                tracing::info!("cron scheduler stopped");
            }
        }
    })
    .await?;

    shutdown_tracing();
    tracing::info!("server exited gracefully");
    Ok(())
}
