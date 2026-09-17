//! Tracing & OpenTelemetry 横切能力。
//!
//! - [`init_tracing`]：安装全局 tracing subscriber（JSON/pretty 结构化日志，
//!   级别由 `RUST_LOG` 或 `Config.log_level` 控制），并在启用时挂载
//!   OpenTelemetry OTLP gRPC exporter。
//! - [`shutdown_tracing`]：优雅关闭时 flush / shutdown tracer provider，避免丢失 span。
//! - [`OtelConfig`]：归一化后的 OTel 配置。
//!
//! trace_id 的 HTTP 透传（traceparent 继承 / `X-Trace-Id` 响应头 / 日志字段）
//! 在 [`crate::middleware::tracing`] 中间件中完成。

pub mod init;

pub use init::{init_tracing, shutdown_tracing, OtelConfig};
