//! 存活/就绪探针 `GET /health`（根级，无 `/api/v1` 前缀，无认证）。
//!
//! 对齐 Go `api/routes.go:485` 的 `r.GET("/health", ...)`：Prometheus scraper /
//! K8s liveness·readinessProbe 直接抓取，不挂 JWT / CSRF。
//!
//! 响应信封（本任务约定，最小信封）：
//! `{"success":true,"data":{"status":"ok","version":"<cargo version>","uptime":<secs>}}`
//!
//! uptime 以进程启动时刻为基准（[`init_start`] 在 `main` 启动时调用）；
//! 单测路径下若未调用 [`init_start`]，则退化为首个请求时刻（get_or_init）。

use std::sync::OnceLock;
use std::time::Instant;

use serde::Serialize;

use axum::Json;

use crate::response::ApiResponse;

/// 进程启动时刻。`main` 启动时调用 [`init_start`] 记录。
static STARTED: OnceLock<Instant> = OnceLock::new();

/// 在进程启动时调用，记录 uptime 基准时刻。
pub fn init_start() {
    // 重复调用无害：OnceLock 仅首次生效。
    let _ = STARTED.set(Instant::now());
}

/// 自进程启动以来的秒数。
fn uptime_secs() -> u64 {
    STARTED.get_or_init(Instant::now).elapsed().as_secs()
}

/// `/health` 返回的 data 体。
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub status: &'static str,
    pub version: &'static str,
    pub uptime: u64,
}

/// `GET /health` — 存活/就绪探针，无需认证。
pub async fn health() -> Json<ApiResponse<HealthStatus>> {
    Json(ApiResponse::success(HealthStatus {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime: uptime_secs(),
    }))
}
