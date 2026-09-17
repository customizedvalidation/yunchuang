//! HTTP 请求指标，逐字对齐 Go `metrics_business.go` 中的三个 HTTP 指标。
//!
//! - `http_requests_total`（CounterVec，labels: method, path, status）
//! - `http_request_duration_seconds`（HistogramVec，labels: method, path，
//!   buckets: [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]）
//! - `http_requests_in_flight`（Gauge）

use prometheus::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};

/// 持有 HTTP 指标句柄，供中间件埋点使用。
pub struct HttpMetrics {
    /// 按 method / path / status 分类的请求总数 Counter。
    pub requests_total: IntCounterVec,
    /// 请求延迟直方图（秒），按 method / path 分类。
    pub request_duration_seconds: HistogramVec,
    /// 当前正在处理的请求数 Gauge。
    pub requests_in_flight: IntGauge,
}

impl HttpMetrics {
    /// 向 Prometheus 全局注册表注册全部 HTTP 指标。
    pub fn new() -> Self {
        let requests_total = register_int_counter_vec!(
            "http_requests_total",
            "Total number of HTTP requests processed, labeled by method, path and status code.",
            &["method", "path", "status"]
        )
        .expect("register http_requests_total");

        let request_duration_seconds = register_histogram_vec!(
            "http_request_duration_seconds",
            "HTTP request latency in seconds, labeled by method and path.",
            &["method", "path"],
            // 与 Go metrics_business.go 完全一致的 buckets。
            vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0],
        )
        .expect("register http_request_duration_seconds");

        let requests_in_flight = register_int_gauge!(
            "http_requests_in_flight",
            "Current number of in-flight HTTP requests being processed."
        )
        .expect("register http_requests_in_flight");

        Self {
            requests_total,
            request_duration_seconds,
            requests_in_flight,
        }
    }

    /// 记录一次请求完成：递增 Counter + 观测 Histogram。
    pub fn record(&self, method: &str, path: &str, status: u16, duration_secs: f64) {
        self.requests_total
            .with_label_values(&[method, path, &status.to_string()])
            .inc();
        self.request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_secs);
    }

    /// 请求进入：in-flight Gauge +1。
    pub fn inc_in_flight(&self) {
        self.requests_in_flight.inc();
    }

    /// 请求结束：in-flight Gauge -1。
    pub fn dec_in_flight(&self) {
        self.requests_in_flight.dec();
    }
}

impl Default for HttpMetrics {
    fn default() -> Self {
        Self::new()
    }
}
