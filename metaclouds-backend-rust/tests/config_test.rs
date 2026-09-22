//! `Config` 单元测试：默认值、校验规则、Cookie SameSite 映射、列表解析、辅助方法。
//!
//! 对齐 Go 版 `config_test.go` 与 `config_extended_test.go` 的测试分支。
//! 直接构造 `Config`（而非读进程环境），避免并行测试间的环境变量竞争。

use metaclouds_backend_rust::config::{parse_list, Config, SameSiteMode};

/// 生产环境下可通过校验的最小合法配置（对齐 Go `validProductionConfig`）。
#[allow(clippy::field_reassign_with_default)]
fn valid_prod() -> Config {
    let mut c = Config::default();
    c.environment = "production".to_string();
    c.use_sqlite = false;
    c.memory_store_enabled = false;
    c.allow_public_registration = false;
    c.database_ssl_mode = "require".to_string();
    c.allowed_origins = vec!["https://example.com".to_string()];
    c.jwt_secret = "s".repeat(32);
    c.jwt_expiration_hours = 24;
    c.rate_limit_enabled = true;
    c.rate_limit_requests = 100;
    c.rate_limit_duration_seconds = 60;
    c
}

#[test]
fn defaults_match_go() {
    let c = Config::default();
    assert_eq!(c.server_host, "0.0.0.0");
    assert_eq!(c.server_port, 8000);
    assert_eq!(c.environment, "development");
    assert!(c.use_sqlite);
    assert!(c.memory_store_enabled);
    assert_eq!(c.database_host, "localhost");
    assert_eq!(c.database_port, "5432");
    assert_eq!(c.database_user, "metaclouds");
    assert_eq!(c.database_name, "metaclouds");
    assert_eq!(c.database_ssl_mode, "disable");
    assert!(!c.redis_enabled);
    assert_eq!(c.redis_host, "localhost");
    assert_eq!(c.redis_port, "6379");
    assert_eq!(c.redis_db, 0);
    assert!(c.jwt_secret.is_empty());
    assert_eq!(c.jwt_expiration_hours, 24);
    assert_eq!(c.jwt_refresh_expiration_hours, 168);
    assert!(c.prometheus_enabled);
    assert_eq!(c.prometheus_port, "9090");
    assert!(c.monitoring_enabled);
    assert!(c.alert_enabled);
    assert_eq!(c.metrics_collection_interval, 15);
    assert!(c.k8s_enabled);
    assert_eq!(c.k8s_namespace, "metaclouds");
    assert!(c.k8s_simulation_mode);
    assert!(c.scheduler_enabled);
    assert_eq!(c.scheduler_interval_seconds, 10);
    assert!(!c.tracing_enabled);
    assert_eq!(c.tracing_service_name, "metaclouds-backend");
    assert!(!c.config_center_enabled);
    assert!(!c.rate_limit_enabled);
    assert_eq!(c.rate_limit_requests, 100);
    assert_eq!(c.rate_limit_duration_seconds, 60);
    assert!(!c.circuit_breaker_enabled);
    assert_eq!(c.circuit_breaker_threshold, 10);
    assert_eq!(c.circuit_breaker_timeout_seconds, 30);
    assert_eq!(c.log_level, "info");
    assert_eq!(c.log_format, "json");
    assert_eq!(c.log_output, "console");
    assert_eq!(c.max_request_body_size, 10_485_760);
    assert_eq!(c.read_timeout_seconds, 30);
    assert_eq!(c.write_timeout_seconds, 30);
    assert_eq!(c.idle_timeout_seconds, 60);
    assert_eq!(c.default_page_size, 10);
    assert_eq!(c.max_page_size, 100);
    assert_eq!(c.slow_request_threshold_ms, 2000);
    assert!(c.feature_gpu_allocation);
    assert!(c.feature_job_scheduler);
    assert!(c.feature_monitoring);
    assert!(c.feature_security_policies);
    assert!(!c.allow_public_registration);
    assert_eq!(c.cookie_same_site, "lax");
    assert!(c.allowed_origins.is_empty());
    assert!(c.trusted_proxies.is_empty());
}

#[test]
fn valid_prod_passes() {
    assert!(valid_prod().validate().is_ok());
}

#[test]
fn jwt_secret_required() {
    let mut c = valid_prod();
    c.jwt_secret = String::new();
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("JWT_SECRET is required"));
}

#[test]
fn jwt_secret_too_short() {
    let mut c = valid_prod();
    c.jwt_secret = "short".to_string();
    let err = c.validate().unwrap_err();
    assert!(
        err.to_string().contains("at least 32 characters"),
        "unexpected: {err}"
    );
}

#[test]
fn jwt_expiration_must_be_positive() {
    let mut c = valid_prod();
    c.jwt_expiration_hours = 0;
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("JWT_EXPIRATION_HOURS"));
}

#[test]
fn prod_forbids_sqlite() {
    let mut c = valid_prod();
    c.use_sqlite = true;
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("USE_SQLITE"));
}

#[test]
fn prod_forbids_memory_store() {
    let mut c = valid_prod();
    c.memory_store_enabled = true;
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("MEMORY_STORE_ENABLED"));
}

#[test]
fn prod_forbids_public_registration() {
    let mut c = valid_prod();
    c.allow_public_registration = true;
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("ALLOW_PUBLIC_REGISTRATION"));
}

#[test]
fn prod_forbids_disable_ssl() {
    let mut c = valid_prod();
    c.database_ssl_mode = "disable".to_string();
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("DATABASE_SSL_MODE"));
}

#[test]
fn prod_requires_allowed_origins() {
    let mut c = valid_prod();
    c.allowed_origins.clear();
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("ALLOWED_ORIGINS must be set"));
}

#[test]
fn prod_forbids_wildcard_origin() {
    let mut c = valid_prod();
    c.allowed_origins = vec!["*".to_string()];
    let err = c.validate().unwrap_err();
    assert!(err.to_string().contains("must not contain"));
}

#[test]
fn rate_limit_disabled_skips_check() {
    let mut c = valid_prod();
    c.rate_limit_enabled = false;
    c.rate_limit_requests = 0;
    c.rate_limit_duration_seconds = 0;
    assert!(c.validate().is_ok());
}

#[test]
fn rate_limit_enabled_requires_positive() {
    let mut c = valid_prod();
    c.rate_limit_requests = 0;
    assert!(c
        .validate()
        .unwrap_err()
        .to_string()
        .contains("RATE_LIMIT_REQUESTS"));

    c.rate_limit_requests = 100;
    c.rate_limit_duration_seconds = 0;
    assert!(c
        .validate()
        .unwrap_err()
        .to_string()
        .contains("RATE_LIMIT_DURATION_SECONDS"));
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn development_allows_sqlite() {
    let mut c = Config::default();
    c.environment = "development".to_string();
    c.use_sqlite = true;
    c.memory_store_enabled = true;
    c.database_ssl_mode = "disable".to_string();
    c.jwt_secret = "s".repeat(32);
    c.jwt_expiration_hours = 24;
    c.rate_limit_enabled = false;
    assert!(c.validate().is_ok());
}

#[test]
fn cookie_same_site_none_requires_production() {
    // 开发环境设 none 应报错。
    let mut c = valid_prod();
    c.environment = "development".to_string();
    c.use_sqlite = true;
    c.cookie_same_site = "none".to_string();
    assert!(c.validate().is_err());

    // 生产环境设 none 应通过。
    c.environment = "production".to_string();
    c.use_sqlite = false;
    assert!(c.validate().is_ok());
}

#[test]
fn cookie_same_site_mapping() {
    let mode = |s: &str| {
        Config {
            cookie_same_site: s.to_string(),
            ..Config::default()
        }
        .cookie_same_site_mode()
    };

    assert_eq!(mode(""), SameSiteMode::Lax);
    assert_eq!(mode("lax"), SameSiteMode::Lax);
    assert_eq!(mode("LAX"), SameSiteMode::Lax);
    assert_eq!(mode("strict"), SameSiteMode::Strict);
    assert_eq!(mode("none"), SameSiteMode::None);
    assert_eq!(mode("bogus"), SameSiteMode::Lax);
}

#[test]
fn helper_addrs() {
    let c = Config {
        database_host: "localhost".to_string(),
        database_port: "5432".to_string(),
        database_user: "user".to_string(),
        database_password: "pass".to_string(),
        database_name: "mydb".to_string(),
        database_ssl_mode: "require".to_string(),
        redis_host: "redis.local".to_string(),
        redis_port: "6380".to_string(),
        server_host: "0.0.0.0".to_string(),
        server_port: 8080,
        prometheus_port: "9091".to_string(),
        ..Config::default()
    };
    let dsn = c.get_database_dsn();
    assert!(dsn.contains("host=localhost"));
    assert!(dsn.contains("port=5432"));
    assert!(dsn.contains("user=user"));
    assert!(dsn.contains("password=pass"));
    assert!(dsn.contains("dbname=mydb"));
    assert!(dsn.contains("sslmode=require"));

    assert_eq!(c.get_redis_addr(), "redis.local:6380");
    assert_eq!(c.get_server_addr(), "0.0.0.0:8080");
    assert_eq!(c.get_prometheus_url(), "http://localhost:9091");
}

#[test]
fn parse_list_cases() {
    assert!(parse_list("").is_empty());
    assert_eq!(parse_list("a"), vec!["a".to_string()]);
    assert_eq!(
        parse_list("a,b,c"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    // 去空白 + 过滤空项。
    assert_eq!(
        parse_list("a, b , c"),
        vec!["a".to_string(), "b".to_string(), "c".to_string()]
    );
    assert_eq!(parse_list("a,,b,"), vec!["a".to_string(), "b".to_string()]);
    assert!(parse_list(",,,").is_empty());
}
