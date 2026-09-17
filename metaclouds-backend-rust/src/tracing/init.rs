//! Tracing subscriber 与 OpenTelemetry OTLP exporter 初始化。
//!
//! 对齐 Go 版 `pkg/logger` 的结构化日志字段（timestamp / level / target / message /
//! trace_id / span_id / request_id）。日志默认 JSON 输出，`LOG_FORMAT=pretty` 时切换
//! 为本地开发的美化格式。
//!
//! OTel 设计为**默认关闭 + 优雅降级**：本机无 collector 时不连接、不 panic；
//! `OTEL_ENABLED=true` 且 endpoint 不可达时，batch exporter 在后台异步重连，
//! 连接失败仅以 internal 日志告警，主流程继续。

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, Tracer, TracerProvider};
use opentelemetry_sdk::Resource;
use opentelemetry_semantic_conventions::attribute::{SERVICE_NAME, SERVICE_VERSION};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};

use crate::config::Config;
use crate::error::AppResult;

/// 归一化后的 OpenTelemetry 配置。
#[derive(Debug, Clone)]
pub struct OtelConfig {
    /// 是否启用 OTLP 导出（默认 false，本机无 collector）。
    pub enabled: bool,
    /// OTLP gRPC endpoint，对齐 `OTEL_EXPORTER_OTLP_ENDPOINT`。
    pub endpoint: String,
    /// trace 采样率（0.0~1.0），parent-based + TraceIdRatioBased。
    pub sample_rate: f64,
    /// `service.name` 资源属性。
    pub service_name: String,
}

impl From<&Config> for OtelConfig {
    fn from(c: &Config) -> Self {
        Self {
            enabled: c.otel_enabled,
            endpoint: c.otel_endpoint.clone(),
            sample_rate: c.otel_sample_rate.clamp(0.0, 1.0),
            // 对齐任务要求：service.name = metaclouds-backend-rust（= Cargo package name）。
            service_name: env!("CARGO_PKG_NAME").to_string(),
        }
    }
}

/// 安装全局 tracing subscriber。
///
/// - 日志级别：`RUST_LOG` 环境变量优先，否则回退到 `Config.log_level`。
/// - 日志格式：`LOG_FORMAT=pretty` 时美化输出，否则 JSON（对齐 Go 结构化日志）。
/// - OTel：`otel_enabled` 为 true 时额外挂载 OTLP gRPC exporter +
///   [`tracing_opentelemetry`] layer；构建失败时降级为不导出。
///
/// 幂等：若全局 subscriber 已被设置（如测试中先装了捕获用 subscriber），
/// 仅告警并保留已有 subscriber，不报错。
pub fn init_tracing(config: &Config) -> AppResult<()> {
    // 日志级别：RUST_LOG 优先，否则用配置的 log_level。
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.log_level.as_str()));

    let otel = OtelConfig::from(config);
    let tracer = if otel.enabled {
        init_otel(&otel)
    } else {
        tracing::debug!(endpoint = %otel.endpoint, "OpenTelemetry OTLP exporter disabled");
        None
    };

    let fmt_layer = tracing_subscriber::fmt::layer();
    let registry = Registry::default().with(filter);

    if config.log_format.eq_ignore_ascii_case("pretty") {
        let subscriber = registry.with(fmt_layer.pretty().with_target(true));
        match tracer {
            Some(tracer) => subscriber
                .with(tracing_opentelemetry::OpenTelemetryLayer::new(tracer))
                .try_init(),
            None => subscriber.try_init(),
        }
    } else {
        let subscriber = registry.with(fmt_layer.json().with_current_span(true).with_target(true));
        match tracer {
            Some(tracer) => subscriber
                .with(tracing_opentelemetry::OpenTelemetryLayer::new(tracer))
                .try_init(),
            None => subscriber.try_init(),
        }
    }
    .map_err(|e| {
        // 全局 subscriber 只能设置一次；测试场景下已被捕获 subscriber 占用，
        // 这里视为可忽略，保留已有 subscriber。
        tracing::warn!("global tracing subscriber already installed: {e}");
    })
    .ok();

    tracing::info!(
        service.name = %otel.service_name,
        service.version = env!("CARGO_PKG_VERSION"),
        log_format = %config.log_format,
        otel_enabled = otel.enabled,
        "tracing initialized"
    );
    Ok(())
}

/// 构建 OTLP gRPC exporter 与 TracerProvider，返回供 [`tracing_opentelemetry`] 使用的 tracer。
///
/// 优雅降级：exporter 构建失败（如非法 endpoint）时打 warn 并返回 `None`，
/// 服务继续以纯日志模式运行。endpoint 不可达属于运行时连接问题，batch exporter
/// 会在后台异步重试，不会在此处阻塞或 panic。
fn init_otel(cfg: &OtelConfig) -> Option<Tracer> {
    let exporter = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.endpoint.as_str())
        .build()
    {
        Ok(exporter) => exporter,
        Err(e) => {
            tracing::warn!(
                error = %e,
                endpoint = %cfg.endpoint,
                "failed to build OTLP span exporter; continuing without trace export"
            );
            return None;
        }
    };

    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, cfg.service_name.clone()),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
    ]);

    // parent_based(sampler=TraceIdRatioBased)：有父 span 时沿用父采样决策，
    // 根 span 按 sample_rate 采样。
    let sampler = Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(cfg.sample_rate)));

    let provider = TracerProvider::builder()
        .with_sampler(sampler)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource)
        .with_batch_exporter(exporter, Tokio)
        .build();

    // 先从 provider 取具体类型的 tracer，再把 provider 交给全局（全局持有时仍可 flush）。
    let tracer = provider.tracer(cfg.service_name.clone());
    global::set_tracer_provider(provider);

    tracing::info!(
        endpoint = %cfg.endpoint,
        sample_rate = cfg.sample_rate,
        "OpenTelemetry OTLP exporter initialized"
    );
    Some(tracer)
}

/// 优雅关闭：shutdown 全局 tracer provider，flush 尚未导出的 span。
pub fn shutdown_tracing() {
    tracing::info!("shutting down tracing, flushing traces");
    global::shutdown_tracer_provider();
}
