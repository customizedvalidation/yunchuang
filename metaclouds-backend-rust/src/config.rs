//! 应用配置：从环境变量加载，并校验启动期必需项。
//!
//! 与 Go 版 `config/config.go` 逐字段对齐：
//! - 通过 `dotenvy` 优先加载 `.env`（缺失不报错，生产环境直接读进程环境变量）。
//! - 所有字段均有与 Go 版一致的默认值。
//! - [`Config::validate`] 复刻 Go 版 `Validate()` 的全部规则。
//!
//! 保留 Phase 0 的 6 个兼容字段（`server_host` / `server_port` / `database_url` /
//! `jwt_secret` / `jwt_expires` / `log_level`），供现有 main / auth / db 模块继续使用；
//! 其余字段为本次按 Go 版补齐的扩展配置。

use std::time::Duration;

use crate::error::{AppError, AppResult};

/// 认证 Cookie 的 SameSite 模式映射结果。
///
/// 与 Go 版 `CookieSameSiteMode()` 对齐：非法值（含空串）一律回退到 [`SameSiteMode::Lax`]，
/// 避免误配导致 Cookie 完全无法携带。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SameSiteMode {
    Lax,
    Strict,
    None,
}

/// 全局应用配置。字段名采用 snake_case，与 Go 版环境变量一一对应。
#[derive(Debug, Clone)]
pub struct Config {
    // ---- 服务 ----
    pub server_host: String,
    pub server_port: u16,
    pub environment: String,
    pub allowed_origins: Vec<String>,

    // ---- 数据库 ----
    pub use_sqlite: bool,
    pub memory_store_enabled: bool,
    pub database_host: String,
    pub database_port: String,
    pub database_user: String,
    pub database_password: String,
    pub database_name: String,
    pub database_ssl_mode: String,
    /// Phase 0 兼容字段：sqlx 连接串（DATABASE_URL）。Go 版由各部件拼装 DSN，
    /// 这里保留独立字段供现有 db 模块直连使用。
    pub database_url: String,

    // ---- Redis ----
    pub redis_enabled: bool,
    pub redis_host: String,
    pub redis_port: String,
    pub redis_password: String,
    pub redis_db: i32,
    /// 直连 Redis 连接串（`redis://[:password@]host:port/db`）。
    /// 非空时优先于 host/port/password/db 拼装；为空则由 [`Self::get_redis_url`]
    /// 从上面四个字段拼装。对齐任务 P3-01 的 `redis_url` 配置项。
    pub redis_url: String,

    // ---- JWT ----
    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,
    pub jwt_refresh_expiration_hours: i64,
    /// Phase 0 兼容字段：访问令牌有效期（秒级 Duration）。
    pub jwt_expires: Duration,

    // ---- 监控 ----
    pub prometheus_enabled: bool,
    pub prometheus_port: String,
    pub monitoring_enabled: bool,
    pub alert_enabled: bool,
    pub metrics_collection_interval: i64,

    // ---- K8S ----
    pub k8s_enabled: bool,
    pub k8s_namespace: String,
    pub k8s_config_path: String,
    pub k8s_simulation_mode: bool,

    // ---- 调度 ----
    pub scheduler_enabled: bool,
    pub scheduler_interval_seconds: i64,

    // ---- 链路追踪 ----
    pub tracing_enabled: bool,
    pub tracing_service_name: String,
    pub jaeger_endpoint: String,
    /// OpenTelemetry OTLP exporter 开关。本机无 collector 时默认关闭，
    /// 启用后通过 gRPC（默认 4317）导出 span，连接失败仅告警不 panic。
    pub otel_enabled: bool,
    /// OTLP gRPC endpoint，对齐 `OTEL_EXPORTER_OTLP_ENDPOINT`。
    pub otel_endpoint: String,
    /// trace 采样率（0.0~1.0），parent-based + TraceIdRatioBased。
    pub otel_sample_rate: f64,
    /// SQL 慢查询阈值（毫秒），超过则打 warn 日志，对齐 Go 版阈值。
    pub slow_query_threshold_ms: u64,

    // ---- 配置中心 ----
    pub config_center_enabled: bool,
    pub config_center_endpoints: String,
    pub config_center_prefix: String,

    // ---- 限流 ----
    pub rate_limit_enabled: bool,
    pub rate_limit_requests: i64,
    pub rate_limit_duration_seconds: i64,

    // ---- 熔断 ----
    pub circuit_breaker_enabled: bool,
    pub circuit_breaker_threshold: i64,
    pub circuit_breaker_timeout_seconds: i64,

    // ---- 日志 ----
    pub log_level: String,
    pub log_format: String,
    pub log_output: String,
    pub log_path: String,

    // ---- HTTP 服务 ----
    pub max_request_body_size: i64,
    pub read_timeout_seconds: i64,
    pub write_timeout_seconds: i64,
    pub idle_timeout_seconds: i64,
    pub trusted_proxies: Vec<String>,

    // ---- 分页 / 慢请求 ----
    pub default_page_size: i64,
    pub max_page_size: i64,
    pub slow_request_threshold_ms: i64,

    // ---- 功能开关 ----
    pub feature_gpu_allocation: bool,
    pub feature_job_scheduler: bool,
    pub feature_monitoring: bool,
    pub feature_security_policies: bool,

    // ---- 注册 / Cookie ----
    pub allow_public_registration: bool,
    pub cookie_same_site: String,
}

impl Default for Config {
    /// 空环境下的默认值，逐一对齐 Go `LoadConfig()` 的默认分支。
    fn default() -> Self {
        Self {
            // 服务
            server_host: "0.0.0.0".to_string(),
            server_port: 8000,
            environment: "development".to_string(),
            allowed_origins: Vec::new(),
            // 数据库
            use_sqlite: true,
            memory_store_enabled: true,
            database_host: "localhost".to_string(),
            database_port: "5432".to_string(),
            database_user: "metaclouds".to_string(),
            database_password: String::new(),
            database_name: "metaclouds".to_string(),
            database_ssl_mode: "disable".to_string(),
            database_url: "sqlite::memory:".to_string(),
            // Redis
            redis_enabled: false,
            redis_host: "localhost".to_string(),
            redis_port: "6379".to_string(),
            redis_password: String::new(),
            redis_db: 0,
            redis_url: String::new(),
            // JWT
            jwt_secret: String::new(),
            jwt_expiration_hours: 24,
            jwt_refresh_expiration_hours: 168,
            jwt_expires: Duration::from_secs(86_400),
            // 监控
            prometheus_enabled: true,
            prometheus_port: "9090".to_string(),
            monitoring_enabled: true,
            alert_enabled: true,
            metrics_collection_interval: 15,
            // K8S
            k8s_enabled: true,
            k8s_namespace: "metaclouds".to_string(),
            k8s_config_path: "~/.kube/config".to_string(),
            k8s_simulation_mode: true,
            // 调度
            scheduler_enabled: true,
            scheduler_interval_seconds: 10,
            // 链路追踪
            tracing_enabled: false,
            tracing_service_name: "metaclouds-backend".to_string(),
            jaeger_endpoint: "http://localhost:14268/api/traces".to_string(),
            otel_enabled: false,
            otel_endpoint: "http://localhost:4317".to_string(),
            otel_sample_rate: 1.0,
            slow_query_threshold_ms: 500,
            // 配置中心
            config_center_enabled: false,
            config_center_endpoints: "localhost:2379".to_string(),
            config_center_prefix: "/metaclouds/config/".to_string(),
            // 限流（默认关闭：中间件按 RATE_LIMIT_ENABLED 环境变量读取，未显式开启即放行）
            rate_limit_enabled: false,
            rate_limit_requests: 100,
            rate_limit_duration_seconds: 60,
            // 熔断（默认关闭：中间件按 CIRCUIT_BREAKER_ENABLED 环境变量读取，未显式开启即放行）
            circuit_breaker_enabled: false,
            circuit_breaker_threshold: 10,
            circuit_breaker_timeout_seconds: 30,
            // 日志
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            log_output: "console".to_string(),
            log_path: "/var/log/metaclouds/backend.log".to_string(),
            // HTTP
            max_request_body_size: 10_485_760,
            read_timeout_seconds: 30,
            write_timeout_seconds: 30,
            idle_timeout_seconds: 60,
            trusted_proxies: Vec::new(),
            // 分页
            default_page_size: 10,
            max_page_size: 100,
            slow_request_threshold_ms: 2000,
            // 功能开关
            feature_gpu_allocation: true,
            feature_job_scheduler: true,
            feature_monitoring: true,
            feature_security_policies: true,
            // 注册 / Cookie
            allow_public_registration: false,
            cookie_same_site: "lax".to_string(),
        }
    }
}

impl Config {
    /// 从进程环境（优先 `.env`）加载并校验配置，启动期 fail-fast。
    ///
    /// 逐字段回写是有意为之：对齐 Go `LoadConfig()` 的 env 读取分支。
    #[allow(clippy::field_reassign_with_default)]
    pub fn from_env() -> AppResult<Self> {
        // 最佳努力加载 .env；生产环境无该文件属正常。
        let _ = dotenvy::dotenv();

        let mut cfg = Config::default();

        // 服务
        cfg.server_host = env_or("SERVER_HOST", "0.0.0.0");
        cfg.server_port = env_int("SERVER_PORT", 8000) as u16;
        cfg.environment = env_or("SERVER_ENV", "development");
        cfg.allowed_origins = parse_list(&env_or("ALLOWED_ORIGINS", ""));

        // 数据库
        cfg.use_sqlite = env_bool("USE_SQLITE", true);
        cfg.memory_store_enabled = env_bool("MEMORY_STORE_ENABLED", true);
        cfg.database_host = env_or("DATABASE_HOST", "localhost");
        cfg.database_port = env_or("DATABASE_PORT", "5432");
        cfg.database_user = env_or("DATABASE_USER", "metaclouds");
        cfg.database_password = env_or("DATABASE_PASSWORD", "");
        cfg.database_name = env_or("DATABASE_NAME", "metaclouds");
        cfg.database_ssl_mode = env_or("DATABASE_SSL_MODE", "disable");
        // Phase 0 兼容连接串（DATABASE_URL）。
        cfg.database_url = env_or("DATABASE_URL", "sqlite::memory:");

        // Redis
        cfg.redis_enabled = env_bool("REDIS_ENABLED", false);
        cfg.redis_host = env_or("REDIS_HOST", "localhost");
        cfg.redis_port = env_or("REDIS_PORT", "6379");
        cfg.redis_password = env_or("REDIS_PASSWORD", "");
        cfg.redis_db = env_int("REDIS_DB", 0) as i32;
        cfg.redis_url = env_or("REDIS_URL", "");

        // JWT
        cfg.jwt_secret = env_or("JWT_SECRET", "");
        cfg.jwt_expiration_hours = env_int("JWT_EXPIRATION_HOURS", 24);
        cfg.jwt_refresh_expiration_hours = env_int("JWT_REFRESH_EXPIRATION_HOURS", 168);
        // Phase 0 兼容：访问令牌有效期（秒）。
        let jwt_expires_secs = env_int("JWT_EXPIRES_SECONDS", 86_400) as u64;
        cfg.jwt_expires = Duration::from_secs(jwt_expires_secs);

        // 监控
        cfg.prometheus_enabled = env_bool("PROMETHEUS_ENABLED", true);
        cfg.prometheus_port = env_or("PROMETHEUS_PORT", "9090");
        cfg.monitoring_enabled = env_bool("MONITORING_ENABLED", true);
        cfg.alert_enabled = env_bool("ALERT_ENABLED", true);
        cfg.metrics_collection_interval = env_int("METRICS_COLLECTION_INTERVAL_SECONDS", 15);

        // K8S
        cfg.k8s_enabled = env_bool("K8S_ENABLED", true);
        cfg.k8s_namespace = env_or("K8S_NAMESPACE", "metaclouds");
        cfg.k8s_config_path = env_or("K8S_CONFIG_PATH", "~/.kube/config");
        cfg.k8s_simulation_mode = env_bool("K8S_SIMULATION_MODE", true);

        // 调度
        cfg.scheduler_enabled = env_bool("SCHEDULER_ENABLED", true);
        cfg.scheduler_interval_seconds = env_int("SCHEDULER_INTERVAL_SECONDS", 10);

        // 链路追踪
        cfg.tracing_enabled = env_bool("TRACING_ENABLED", false);
        cfg.tracing_service_name = env_or("TRACING_SERVICE_NAME", "metaclouds-backend");
        cfg.jaeger_endpoint = env_or("JAEGER_ENDPOINT", "http://localhost:14268/api/traces");
        // P3-04 OpenTelemetry OTLP（本机无 collector，默认关闭）。
        cfg.otel_enabled = env_bool("OTEL_ENABLED", false);
        cfg.otel_endpoint = env_or("OTEL_EXPORTER_OTLP_ENDPOINT", "http://localhost:4317");
        cfg.otel_sample_rate = env_float("OTEL_SAMPLE_RATE", 1.0);
        cfg.slow_query_threshold_ms = env_int("SLOW_QUERY_THRESHOLD_MS", 500) as u64;

        // 配置中心
        cfg.config_center_enabled = env_bool("CONFIG_CENTER_ENABLED", false);
        cfg.config_center_endpoints = env_or("CONFIG_CENTER_ENDPOINTS", "localhost:2379");
        cfg.config_center_prefix = env_or("CONFIG_CENTER_PREFIX", "/metaclouds/config/");

        // 限流（默认关闭，与中间件 std::env::var 直读行为一致；生产显式置 true 开启）
        cfg.rate_limit_enabled = env_bool("RATE_LIMIT_ENABLED", false);
        cfg.rate_limit_requests = env_int("RATE_LIMIT_REQUESTS", 100);
        cfg.rate_limit_duration_seconds = env_int("RATE_LIMIT_DURATION_SECONDS", 60);

        // 熔断（默认关闭，与中间件 std::env::var 直读行为一致；生产显式置 true 开启）
        cfg.circuit_breaker_enabled = env_bool("CIRCUIT_BREAKER_ENABLED", false);
        cfg.circuit_breaker_threshold = env_int("CIRCUIT_BREAKER_THRESHOLD", 10);
        cfg.circuit_breaker_timeout_seconds = env_int("CIRCUIT_BREAKER_TIMEOUT_SECONDS", 30);

        // 日志
        cfg.log_level = env_or("LOG_LEVEL", "info");
        cfg.log_format = env_or("LOG_FORMAT", "json");
        cfg.log_output = env_or("LOG_OUTPUT", "console");
        cfg.log_path = env_or("LOG_PATH", "/var/log/metaclouds/backend.log");

        // HTTP
        cfg.max_request_body_size = env_int("MAX_REQUEST_BODY_SIZE", 10_485_760);
        cfg.read_timeout_seconds = env_int("READ_TIMEOUT_SECONDS", 30);
        cfg.write_timeout_seconds = env_int("WRITE_TIMEOUT_SECONDS", 30);
        cfg.idle_timeout_seconds = env_int("IDLE_TIMEOUT_SECONDS", 60);
        cfg.trusted_proxies = parse_list(&env_or("TRUSTED_PROXIES", ""));

        // 分页 / 慢请求
        cfg.default_page_size = env_int("DEFAULT_PAGE_SIZE", 10);
        cfg.max_page_size = env_int("MAX_PAGE_SIZE", 100);
        cfg.slow_request_threshold_ms = env_int("SLOW_REQUEST_THRESHOLD_MS", 2000);

        // 功能开关
        cfg.feature_gpu_allocation = env_bool("FEATURE_GPU_ALLOCATION", true);
        cfg.feature_job_scheduler = env_bool("FEATURE_JOB_SCHEDULER", true);
        cfg.feature_monitoring = env_bool("FEATURE_MONITORING", true);
        cfg.feature_security_policies = env_bool("FEATURE_SECURITY_POLICIES", true);

        // 注册 / Cookie
        cfg.allow_public_registration = env_bool("ALLOW_PUBLIC_REGISTRATION", false);
        cfg.cookie_same_site = env_or("COOKIE_SAME_SITE", "lax");

        cfg.validate()?;
        Ok(cfg)
    }

    /// 启动期校验，规则逐一对齐 Go 版 `Validate()`。
    pub fn validate(&self) -> AppResult<()> {
        if self.jwt_secret.is_empty() {
            return Err(AppError::bad_request("JWT_SECRET is required"));
        }
        if self.jwt_secret.len() < 32 {
            return Err(AppError::bad_request(
                "JWT_SECRET must be at least 32 characters long for security",
            ));
        }
        if self.jwt_expiration_hours <= 0 {
            return Err(AppError::bad_request(
                "JWT_EXPIRATION_HOURS must be greater than 0",
            ));
        }

        // 仅当启用限流时才校验限流参数；关闭时默认 0 不应阻断启动。
        if self.rate_limit_enabled {
            if self.rate_limit_requests <= 0 {
                return Err(AppError::bad_request(
                    "RATE_LIMIT_REQUESTS must be greater than 0 when RATE_LIMIT_ENABLED is true",
                ));
            }
            if self.rate_limit_duration_seconds <= 0 {
                return Err(AppError::bad_request(
                    "RATE_LIMIT_DURATION_SECONDS must be greater than 0 when RATE_LIMIT_ENABLED is true",
                ));
            }
        }

        let is_prod = self.environment == "production";
        if is_prod && self.memory_store_enabled {
            return Err(AppError::bad_request(
                "MEMORY_STORE_ENABLED must be false in production environment",
            ));
        }
        // 生产环境禁止开放注册：账号必须由管理员或 IdP 开通。
        if is_prod && self.allow_public_registration {
            return Err(AppError::bad_request(
                "ALLOW_PUBLIC_REGISTRATION must be false in production environment",
            ));
        }
        if is_prod && self.use_sqlite {
            return Err(AppError::bad_request(
                "USE_SQLITE must be false in production environment",
            ));
        }
        if is_prod && self.database_ssl_mode == "disable" {
            return Err(AppError::bad_request(
                "DATABASE_SSL_MODE must not be 'disable' in production environment",
            ));
        }

        // 生产环境 CORS 与可信代理必须显式声明。
        if is_prod {
            if self.allowed_origins.is_empty() {
                return Err(AppError::bad_request(
                    "ALLOWED_ORIGINS must be set in production environment",
                ));
            }
            if self.allowed_origins.iter().any(|o| o == "*") {
                return Err(AppError::bad_request(
                    "ALLOWED_ORIGINS must not contain '*' when credentials are allowed",
                ));
            }
        }

        // SameSite=None 要求 Secure，而 Secure 仅在 https（生产）下有效。
        if self.cookie_same_site.trim().eq_ignore_ascii_case("none") && !is_prod {
            return Err(AppError::bad_request(
                "COOKIE_SAME_SITE=none requires SERVER_ENV=production (Secure cookies require HTTPS)",
            ));
        }

        Ok(())
    }

    /// PostgreSQL 连接串（对齐 Go `GetDatabaseDSN`）。
    pub fn get_database_dsn(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={} sslmode={}",
            self.database_host,
            self.database_port,
            self.database_user,
            self.database_password,
            self.database_name,
            self.database_ssl_mode,
        )
    }

    /// Redis 地址（对齐 Go `GetRedisAddr`）。
    pub fn get_redis_addr(&self) -> String {
        format!("{}:{}", self.redis_host, self.redis_port)
    }

    /// Redis 连接串（供 redis-rs `Client::open`）。
    ///
    /// 优先使用显式配置的 [`Self::redis_url`]；否则从 host/port/password/db
    /// 拼装 `redis://[password@]host:port/db`。
    pub fn get_redis_url(&self) -> String {
        if !self.redis_url.is_empty() {
            return self.redis_url.clone();
        }
        let auth = if self.redis_password.is_empty() {
            String::new()
        } else {
            format!(":{}@", self.redis_password)
        };
        format!(
            "redis://{}{}/{}",
            auth,
            self.get_redis_addr(),
            self.redis_db
        )
    }

    /// Prometheus 地址（对齐 Go `GetPrometheusURL`）。
    pub fn get_prometheus_url(&self) -> String {
        format!("http://localhost:{}", self.prometheus_port)
    }

    /// 监听地址（对齐 Go `GetServerAddr`）。
    pub fn get_server_addr(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }

    /// axum 绑定地址（Phase 0 兼容方法，等价于 `get_server_addr`）。
    pub fn bind_addr(&self) -> String {
        self.get_server_addr()
    }

    /// 把配置的 SameSite 字符串映射为枚举（对齐 Go `CookieSameSiteMode`）。
    /// 非法值（含空串）回退到 [`SameSiteMode::Lax`]。
    pub fn cookie_same_site_mode(&self) -> SameSiteMode {
        match self.cookie_same_site.trim().to_ascii_lowercase().as_str() {
            "none" => SameSiteMode::None,
            "strict" => SameSiteMode::Strict,
            _ => SameSiteMode::Lax,
        }
    }
}

/// 读取字符串环境变量；未设置或为空时返回 `default`（对齐 Go `getEnv`）。
fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// 读取布尔环境变量；解析失败回退 `default`（对齐 Go `strconv.ParseBool` 语义）。
fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key).ok().filter(|v| !v.is_empty()) {
        Some(v) => v.eq_ignore_ascii_case("true") || v == "1",
        None => default,
    }
}

/// 读取整数环境变量；解析失败回退 `default`。
fn env_int(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// 读取浮点环境变量；解析失败回退 `default`。
fn env_float(key: &str, default: f64) -> f64 {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// 把逗号分隔配置拆成列表并去除空白项（对齐 Go `parseList`）。
pub fn parse_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}
