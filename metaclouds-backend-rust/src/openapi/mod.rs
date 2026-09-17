//! P3-05 OpenAPI 3.0 文档构建（utoipa 5）。
//!
//! 聚合所有业务 handler 的 `#[utoipa::path]` 注解与请求/响应 schema，
//! 对外提供 [`ApiDoc`]（可序列化 OpenAPI JSON）与统一错误响应 schema
//! [`ErrorResponse`]（对齐 Go `pkg/response` 的错误信封）。
//!
//! 安全方案：
//! - `bearer_auth`：HTTP Bearer JWT（`Authorization: Bearer <token>`）；
//! - `cookie_auth`：apiKey in cookie（`access_token`，浏览器会话通道）。
//!
//! 路径前缀统一为 `/api/v1`（对齐 Go `api/routes.go` 与本仓 `routes.rs`）。

use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// 统一错误响应 schema（对齐 `ApiResponse` 错误形式）。
///
/// 实际信封：`{"success":false,"message":"...","code":"...","timestamp":<sec>}`。
/// 成功响应的 `data` 字段由各 handler 对应的业务 schema 描述。
#[allow(missing_docs)]
#[derive(utoipa::ToSchema, serde::Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
    pub code: String,
    pub timestamp: i64,
}

/// 注入安全方案（bearer_auth + cookie_auth）。
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "cookie_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("access_token"))),
        );
    }
}

/// 全量 OpenAPI 文档。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Metaclouds Backend API",
        version = "0.1.0",
        description = "Rust 版 Metaclouds AI 算力调度后端，对齐 Go v1 API 契约。\n\n统一响应信封：`{\"success\":bool,\"data\":...,\"message\":...,\"code\":...,\"timestamp\":sec}`。错误响应对见 `ErrorResponse`。"
    ),
    servers(
        (url = "http://localhost:8001", description = "development")
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "auth", description = "认证：登录/登出/刷新/profile/CSRF/改密"),
        (name = "users", description = "用户管理（管理员）"),
        (name = "tenants", description = "租户管理"),
        (name = "clusters", description = "集群管理"),
        (name = "resources", description = "资源管理"),
        (name = "topology", description = "节点拓扑"),
        (name = "k8s", description = "K8s 只读观测（mock）"),
        (name = "jobs", description = "作业管理"),
        (name = "gpus", description = "GPU 设备与分配"),
        (name = "partitions", description = "分区管理"),
        (name = "quotas", description = "配额管理"),
        (name = "schedulers", description = "外部调度器集成"),
        (name = "datasets", description = "数据集管理"),
        (name = "checkpoints", description = "检查点管理"),
        (name = "acceleration", description = "加速套件"),
        (name = "alerts", description = "告警管理"),
        (name = "security", description = "安全策略"),
        (name = "monitoring", description = "监控与告警规则"),
    ),
    paths(
        crate::auth::handler::login,
        crate::auth::handler::logout,
        crate::auth::handler::refresh,
        crate::auth::handler::get_profile,
        crate::auth::handler::change_password,
        crate::auth::handler::get_csrf_token,
        crate::handlers::user::list_users,
        crate::handlers::user::create_user,
        crate::handlers::user::get_user,
        crate::handlers::user::update_user,
        crate::handlers::user::delete_user,
        crate::handlers::tenant::list_tenants,
        crate::handlers::tenant::get_tenant,
        crate::handlers::tenant::create_tenant,
        crate::handlers::tenant::update_tenant,
        crate::handlers::tenant::delete_tenant,
        crate::handlers::cluster::list_clusters,
        crate::handlers::cluster::get_cluster,
        crate::handlers::cluster::create_cluster,
        crate::handlers::cluster::update_cluster,
        crate::handlers::cluster::delete_cluster,
        crate::handlers::resource::list_resources,
        crate::handlers::resource::get_resource,
        crate::handlers::resource::create_resource,
        crate::handlers::resource::update_resource,
        crate::handlers::resource::delete_resource,
        crate::handlers::topology::list_nodes,
        crate::handlers::topology::get_node,
        crate::handlers::topology::create_node,
        crate::handlers::topology::update_node,
        crate::handlers::topology::delete_node,
        crate::handlers::k8s::list_pods,
        crate::handlers::k8s::list_nodes,
        crate::handlers::k8s::cluster_health,
        crate::handlers::job::list_jobs,
        crate::handlers::job::get_job_stats,
        crate::handlers::job::get_job,
        crate::handlers::job::create_job,
        crate::handlers::job::update_job,
        crate::handlers::job::delete_job,
        crate::handlers::job::cancel_job,
        crate::handlers::gpu::list_gpu_devices,
        crate::handlers::gpu::get_gpu_device,
        crate::handlers::gpu::create_gpu_device,
        crate::handlers::gpu::update_gpu_device,
        crate::handlers::gpu::delete_gpu_device,
        crate::handlers::gpu::list_allocations,
        crate::handlers::gpu::allocate_gpu,
        crate::handlers::gpu::release_gpu,
        crate::handlers::gpu::get_gpu_utilization,
        crate::handlers::partition::list_partitions,
        crate::handlers::partition::get_partition,
        crate::handlers::partition::create_partition,
        crate::handlers::partition::update_partition,
        crate::handlers::partition::delete_partition,
        crate::handlers::partition::get_partition_resources,
        crate::handlers::partition::grant_permission,
        crate::handlers::partition::revoke_permission,
        crate::handlers::quota::list_quotas,
        crate::handlers::quota::get_quota,
        crate::handlers::quota::create_quota,
        crate::handlers::quota::update_quota,
        crate::handlers::quota::delete_quota,
        crate::handlers::quota::check_quota,
        crate::handlers::scheduler::list_schedulers,
        crate::handlers::scheduler::get_scheduler,
        crate::handlers::scheduler::create_scheduler,
        crate::handlers::scheduler::update_scheduler,
        crate::handlers::scheduler::delete_scheduler,
        crate::handlers::scheduler::test_connection,
        crate::handlers::scheduler::sync_resources,
        crate::handlers::dataset::list_datasets,
        crate::handlers::dataset::get_dataset,
        crate::handlers::dataset::create_dataset,
        crate::handlers::dataset::update_dataset,
        crate::handlers::dataset::delete_dataset,
        crate::handlers::checkpoint::list_checkpoints,
        crate::handlers::checkpoint::get_checkpoint,
        crate::handlers::checkpoint::create_checkpoint,
        crate::handlers::checkpoint::update_checkpoint,
        crate::handlers::checkpoint::delete_checkpoint,
        crate::handlers::acceleration::list_suites,
        crate::handlers::acceleration::get_suite,
        crate::handlers::acceleration::create_suite,
        crate::handlers::acceleration::update_suite,
        crate::handlers::acceleration::delete_suite,
        crate::handlers::acceleration::start_suite,
        crate::handlers::acceleration::stop_suite,
        crate::handlers::alert::list_alerts,
        crate::handlers::alert::get_alert,
        crate::handlers::alert::create_alert,
        crate::handlers::alert::update_alert,
        crate::handlers::alert::delete_alert,
        crate::handlers::alert::acknowledge_alert,
        crate::handlers::alert::resolve_alert,
        crate::handlers::alert::get_alert_stats,
        crate::handlers::security::list_policies,
        crate::handlers::security::get_policy,
        crate::handlers::security::create_policy,
        crate::handlers::security::update_policy,
        crate::handlers::security::delete_policy,
        crate::handlers::security::enable_policy,
        crate::handlers::security::disable_policy,
        crate::handlers::monitoring::get_dashboard,
        crate::handlers::monitoring::get_metrics,
        crate::handlers::monitoring::list_alert_rules,
        crate::handlers::monitoring::evaluate_alert_rules,
    )
)]
pub struct ApiDoc;
