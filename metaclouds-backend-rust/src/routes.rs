//! Router assembly: public auth routes + protected domain routes.
//!
//! 对齐 Go `api/routes.go`：公开路由仅 login；受保护路由需 JWT，
//! 各域写操作按 RBAC 权限中间件保护。中间件栈由
//! `middleware::apply_core_stack` 统一应用（request_id → request_logger →
//! timing → security_headers → error_handler → panic_recover）。
//!
//! Phase 2 (B1-B6) 全量注册：Tenant / Resource / Cluster / Topology /
//! K8s(mock) / Job / GPU / Partition / Quota / Scheduler / Dataset /
//! Checkpoint / Acceleration / Alert / Security / Monitoring。

use axum::routing::{delete, get, post, put};
use axum::Router;
use tower_cookies::CookieManagerLayer;

use crate::auth::csrf::csrf_protect;
use crate::auth::handler::{change_password, get_csrf_token, get_profile, login, logout, refresh};
use crate::auth::middleware::{jwt_auth, AppState};
use crate::authz::{permissions, require_permission};
use crate::handlers::acceleration::{
    create_suite, delete_suite, get_suite, list_suites, start_suite, stop_suite, update_suite,
};
use crate::handlers::alert::{
    acknowledge_alert, create_alert, delete_alert, get_alert, get_alert_stats, list_alerts,
    resolve_alert, update_alert,
};
use crate::handlers::checkpoint::{
    create_checkpoint, delete_checkpoint, get_checkpoint, get_latest_checkpoint, list_checkpoints,
    update_checkpoint,
};
use crate::handlers::cluster::{
    create_cluster, delete_cluster, get_cluster, list_clusters, update_cluster,
};
use crate::handlers::dataset::{
    create_dataset, delete_dataset, get_dataset, list_datasets, update_dataset,
};
use crate::handlers::fluid_cache::{
    create_fluid_cache, delete_fluid_cache, disable_fluid_cache, enable_fluid_cache,
    list_fluid_caches, prefetch_fluid_cache, update_fluid_cache,
};
use crate::handlers::gpu::{
    allocate_gpu, create_gpu_device, delete_gpu_device, get_gpu_device, get_gpu_utilization,
    list_allocations, list_gpu_devices, release_gpu, update_gpu_device,
};
use crate::handlers::health::health;
use crate::handlers::job::{
    cancel_job, create_job, delete_job, get_job, get_job_stats, get_job_status, list_jobs,
    submit_job_to_k8s, update_job,
};
use crate::handlers::k8s::{
    cluster_health, cluster_status, list_nodes as k8s_list_nodes, list_pods,
};
use crate::handlers::monitoring::{
    evaluate_alert_rules, get_dashboard, get_metrics, list_alert_rules, list_monitoring_alerts,
};
use crate::handlers::partition::{
    create_partition, delete_partition, get_partition, get_partition_resources, grant_permission,
    list_partition_permissions, list_partitions, revoke_permission, revoke_permission_by_id,
    update_max_runtime, update_partition, update_priority,
};
use crate::handlers::quota::{
    check_quota, create_quota, delete_quota, get_quota, get_quota_usage, list_quotas, update_quota,
};
use crate::handlers::resource::{
    create_resource, delete_resource, get_resource, list_gpu_resources, list_resources,
    update_resource,
};
use crate::handlers::scheduler::{
    create_scheduler, delete_scheduler, get_scheduler, list_scheduler_nodes, list_scheduler_queues,
    list_schedulers, scheduler_health, sync_resources, test_connection, update_scheduler,
};
use crate::handlers::security::{
    create_policy, delete_policy, disable_policy, enable_policy, get_policy, list_policies,
    update_policy,
};
use crate::handlers::tenant::{
    create_tenant, delete_tenant, get_tenant, list_tenants, update_tenant,
};
use crate::handlers::topology::{
    calculate_topology_score, create_node, delete_node, get_node, list_nodes, update_node,
};
use crate::handlers::user::{create_user, delete_user, get_user, list_users, update_user};
use crate::metrics::metrics_handler;
use crate::middleware::apply_core_stack;
use crate::openapi::ApiDoc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Build the full application router. `state` is shared with all handlers.
pub fn build_router(state: AppState) -> Router {
    // ── Public routes (no JWT required) ──────────────────────────────────
    let public = Router::new().route("/auth/login", post(login));

    // ── Auth routes (JWT only, no extra permission) ────────────────────
    let auth_routes = Router::new()
        .route("/auth/logout", post(logout))
        .route("/auth/refresh", post(refresh))
        .route("/auth/profile", get(get_profile))
        .route("/auth/csrf", get(get_csrf_token))
        .route("/auth/change-password", put(change_password));

    // ── Users CRUD (JWT + admin) ───────────────────────────────────────
    let users = Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/{id}", get(get_user).put(update_user).delete(delete_user))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ADMIN.to_string(),
            require_permission,
        ));

    // ── Tenants (B1): read=tenant:read, write=tenant:write ─────────────
    let tenants_read = Router::new()
        .route("/tenants", get(list_tenants))
        .route("/tenants/{id}", get(get_tenant))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_READ.to_string(),
            require_permission,
        ));
    let tenants_write = Router::new()
        .route("/tenants", post(create_tenant))
        .route("/tenants/{id}", put(update_tenant).delete(delete_tenant))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_WRITE.to_string(),
            require_permission,
        ));
    let tenants = tenants_read.merge(tenants_write);

    // ── Clusters (B2): read=JWT, write=cluster:write ───────────────────
    // GET /clusters/:id/status 对齐 Go k8sController.GetClusterStatus（JWT，无额外权限）。
    let clusters_read = Router::new()
        .route("/clusters", get(list_clusters))
        .route("/clusters/{id}", get(get_cluster))
        .route("/clusters/{id}/status", get(cluster_status));
    let clusters_write = Router::new()
        .route("/clusters", post(create_cluster))
        .route("/clusters/{id}", put(update_cluster).delete(delete_cluster))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_WRITE.to_string(),
            require_permission,
        ));
    let clusters = clusters_read.merge(clusters_write);

    // ── Resources (B2): read=JWT, write=resource:write ─────────────────
    // `/resources/gpu`（静态段）优先于 `/resources/{id}`（动态段），matchit 按静态优先匹配。
    let resources_read = Router::new()
        .route("/resources", get(list_resources))
        .route("/resources/gpu", get(list_gpu_resources))
        .route("/resources/{id}", get(get_resource));
    let resources_write = Router::new()
        .route("/resources", post(create_resource))
        .route(
            "/resources/{id}",
            put(update_resource).delete(delete_resource),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::RESOURCE_WRITE.to_string(),
            require_permission,
        ));
    let resources = resources_read.merge(resources_write);

    // ── Topology (B2): read=JWT, write=topology:write ──────────────────
    // Primary paths (/topology) + Vue3 aliases (/topology/nodes).
    let topology_read = Router::new()
        .route("/topology", get(list_nodes))
        .route("/topology/{id}", get(get_node))
        .route("/topology/nodes", get(list_nodes))
        .route("/topology/nodes/{id}", get(get_node));
    let topology_write = Router::new()
        .route("/topology", post(create_node))
        .route("/topology/{id}", put(update_node).delete(delete_node))
        .route("/topology/nodes", post(create_node))
        .route("/topology/nodes/{id}", put(update_node).delete(delete_node))
        // Vue3 别名：前端 `topologyApi.calculateScore` POST /topology/score。
        // 静态段 `score` 优先于 `/topology/{id}` 动态段匹配。
        .route("/topology/score", post(calculate_topology_score))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TOPOLOGY_WRITE.to_string(),
            require_permission,
        ));
    let topology = topology_read.merge(topology_write);

    // ── K8s mock (B2): JWT only ────────────────────────────────────────
    let k8s = Router::new()
        .route("/k8s/clusters/{id}/pods", get(list_pods))
        .route("/k8s/clusters/{id}/nodes", get(k8s_list_nodes))
        .route("/k8s/clusters/{id}/health", get(cluster_health));

    // ── Jobs (B3): read=JWT, write=job:write ────────────────────────────
    let jobs_read = Router::new()
        .route("/jobs", get(list_jobs))
        .route("/jobs/stats", get(get_job_stats))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/status", get(get_job_status));
    let jobs_write = Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{id}", put(update_job).delete(delete_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route("/jobs/{id}/submit", post(submit_job_to_k8s))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::JOB_WRITE.to_string(),
            require_permission,
        ));
    let jobs = jobs_read.merge(jobs_write);

    // ── GPUs (B3): read=JWT, write=gpu:write; allocations=job:write ────
    let gpus_read = Router::new()
        .route("/gpus", get(list_gpu_devices))
        .route("/gpus/{id}", get(get_gpu_device))
        .route("/gpus/allocations", get(list_allocations))
        .route("/gpus/utilization", get(get_gpu_utilization));
    let gpus_write = Router::new()
        .route("/gpus", post(create_gpu_device))
        .route(
            "/gpus/{id}",
            put(update_gpu_device).delete(delete_gpu_device),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::GPU_WRITE.to_string(),
            require_permission,
        ));
    // GPU allocations write uses job:write (aligned with Go).
    let gpu_alloc_write = Router::new()
        .route("/gpus/allocations", post(allocate_gpu))
        .route("/gpus/allocations/{id}", delete(release_gpu))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::JOB_WRITE.to_string(),
            require_permission,
        ));
    let gpus = gpus_read.merge(gpus_write).merge(gpu_alloc_write);

    // GPU Vue3 aliases: /gpu/devices, /gpu/allocations, /gpu/utilization
    let gpu_alias_read = Router::new()
        .route("/gpu/devices", get(list_gpu_devices))
        .route("/gpu/devices/{id}", get(get_gpu_device))
        .route("/gpu/allocations", get(list_allocations))
        .route("/gpu/utilization", get(get_gpu_utilization));
    let gpu_alias_write = Router::new()
        .route("/gpu/devices", post(create_gpu_device))
        .route(
            "/gpu/devices/{id}",
            put(update_gpu_device).delete(delete_gpu_device),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::GPU_WRITE.to_string(),
            require_permission,
        ));
    let gpu_alias_alloc_write = Router::new()
        .route("/gpu/allocations", post(allocate_gpu))
        .route("/gpu/allocations/{id}/release", post(release_gpu))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::JOB_WRITE.to_string(),
            require_permission,
        ));
    let gpu_aliases = gpu_alias_read
        .merge(gpu_alias_write)
        .merge(gpu_alias_alloc_write);

    // ── Partitions (B4): read=JWT, write=partition:write ───────────────
    // 对齐 Go partitions 组：GET /:id/permissions（JWT）、
    // PUT /:id/priority、PUT /:id/max-runtime（partition:write）。
    // Go 无 GET /:id/priority 与 GET /:id/max-runtime，故不补。
    let partitions_read = Router::new()
        .route("/partitions", get(list_partitions))
        .route("/partitions/{id}", get(get_partition))
        .route("/partitions/{id}/resources", get(get_partition_resources))
        .route(
            "/partitions/{id}/permissions",
            get(list_partition_permissions),
        );
    let partitions_write = Router::new()
        .route("/partitions", post(create_partition))
        .route(
            "/partitions/{id}",
            put(update_partition).delete(delete_partition),
        )
        .route("/partitions/{id}/priority", put(update_priority))
        .route("/partitions/{id}/max-runtime", put(update_max_runtime))
        .route("/partitions/{id}/permissions", post(grant_permission))
        .route(
            "/partitions/{id}/permissions/{perm_id}",
            delete(revoke_permission),
        )
        // Go 风格兼容别名：前端 `removePermission(id)` 走扁平路径 `/partitions/permissions/:id`。
        .route(
            "/partitions/permissions/{id}",
            delete(revoke_permission_by_id),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::PARTITION_WRITE.to_string(),
            require_permission,
        ));
    let partitions = partitions_read.merge(partitions_write);

    // ── Quotas (B4): read=JWT, write=quota:write ──────────────────────
    let quotas_read = Router::new()
        .route("/quotas", get(list_quotas))
        .route("/quotas/usage", get(get_quota_usage))
        .route("/quotas/{id}", get(get_quota));
    let quotas_write = Router::new()
        .route("/quotas", post(create_quota))
        .route("/quotas/{id}", put(update_quota).delete(delete_quota))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::QUOTA_WRITE.to_string(),
            require_permission,
        ));
    // POST /quotas/check — JWT only (aligned with Go: no permission).
    let quotas_check = Router::new().route("/quotas/check", post(check_quota));
    let quotas = quotas_read.merge(quotas_write).merge(quotas_check);

    // ── Schedulers (B4): read=JWT, write=scheduler:write ───────────────
    let schedulers_read = Router::new()
        .route("/schedulers", get(list_schedulers))
        .route("/schedulers/{id}", get(get_scheduler))
        .route("/schedulers/{id}/queues", get(list_scheduler_queues))
        .route("/schedulers/{id}/nodes", get(list_scheduler_nodes))
        .route("/schedulers/{id}/health", get(scheduler_health));
    let schedulers_write = Router::new()
        .route("/schedulers", post(create_scheduler))
        .route(
            "/schedulers/{id}",
            put(update_scheduler).delete(delete_scheduler),
        )
        .route("/schedulers/{id}/sync", post(sync_resources))
        .route("/schedulers/{id}/test-connection", post(test_connection))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::SCHEDULER_WRITE.to_string(),
            require_permission,
        ));
    let schedulers = schedulers_read.merge(schedulers_write);

    // ── Datasets (B5): read=JWT, write=dataset:write ──────────────────
    // FluidCache 对齐 Go：嵌套在 datasets 下（/datasets/:id/caches、Vue 别名
    // /datasets/:id/fluid-caches），并在顶层 /fluid-caches/:cacheId 上提供
    // update/delete/enable/disable/prefetch。Go 无顶层 list/detail/create，故不补。
    let datasets_read = Router::new()
        .route("/datasets", get(list_datasets))
        .route("/datasets/{id}", get(get_dataset))
        .route("/datasets/{id}/caches", get(list_fluid_caches))
        .route("/datasets/{id}/fluid-caches", get(list_fluid_caches));
    let datasets_write = Router::new()
        .route("/datasets", post(create_dataset))
        .route("/datasets/{id}", put(update_dataset).delete(delete_dataset))
        .route("/datasets/{id}/caches", post(create_fluid_cache))
        .route("/datasets/{id}/fluid-caches", post(create_fluid_cache))
        .route(
            "/fluid-caches/{cache_id}",
            put(update_fluid_cache).delete(delete_fluid_cache),
        )
        .route("/fluid-caches/{cache_id}/enable", post(enable_fluid_cache))
        .route(
            "/fluid-caches/{cache_id}/disable",
            post(disable_fluid_cache),
        )
        .route(
            "/fluid-caches/{cache_id}/prefetch",
            post(prefetch_fluid_cache),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::DATASET_WRITE.to_string(),
            require_permission,
        ));
    let datasets = datasets_read.merge(datasets_write);

    // ── Checkpoints (B5): read=JWT, write=checkpoint:write ─────────────
    let checkpoints_read = Router::new()
        .route("/checkpoints", get(list_checkpoints))
        .route("/checkpoints/latest/{job_id}", get(get_latest_checkpoint))
        .route("/checkpoints/{id}", get(get_checkpoint));
    let checkpoints_write = Router::new()
        .route("/checkpoints", post(create_checkpoint))
        .route(
            "/checkpoints/{id}",
            put(update_checkpoint).delete(delete_checkpoint),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CHECKPOINT_WRITE.to_string(),
            require_permission,
        ));
    let checkpoints = checkpoints_read.merge(checkpoints_write);

    // ── Acceleration (B5): read=JWT, write=acceleration:write ──────────
    let acceleration_read = Router::new()
        .route("/acceleration", get(list_suites))
        .route("/acceleration/{id}", get(get_suite));
    let acceleration_write = Router::new()
        .route("/acceleration", post(create_suite))
        .route("/acceleration/{id}", put(update_suite).delete(delete_suite))
        .route("/acceleration/{id}/start", post(start_suite))
        .route("/acceleration/{id}/stop", post(stop_suite))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ACCELERATION_WRITE.to_string(),
            require_permission,
        ));
    let acceleration = acceleration_read.merge(acceleration_write);

    // ── Alerts (B6): read=JWT, write=alert:write ──────────────────────
    let alerts_read = Router::new()
        .route("/alerts", get(list_alerts))
        .route("/alerts/stats", get(get_alert_stats))
        .route("/alerts/{id}", get(get_alert));
    let alerts_write = Router::new()
        .route("/alerts", post(create_alert))
        .route("/alerts/{id}", put(update_alert).delete(delete_alert))
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ALERT_WRITE.to_string(),
            require_permission,
        ));
    let alerts = alerts_read.merge(alerts_write);

    // ── Security (B6): read=JWT, write=security:write ─────────────────
    let security_read = Router::new()
        .route("/security/policies", get(list_policies))
        .route("/security/policies/{id}", get(get_policy));
    let security_write = Router::new()
        .route("/security/policies", post(create_policy))
        .route(
            "/security/policies/{id}",
            put(update_policy).delete(delete_policy),
        )
        .route("/security/policies/{id}/enable", post(enable_policy))
        .route("/security/policies/{id}/disable", post(disable_policy))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::SECURITY_WRITE.to_string(),
            require_permission,
        ));
    let security = security_read.merge(security_write);

    // ── Monitoring (B6): read=JWT, write=monitoring:write ─────────────
    // `/monitoring/alerts` 为 Vue3 别名（正式路由为 `/alerts`），对齐前端监控页。
    let monitoring_read = Router::new()
        .route("/monitoring/dashboard", get(get_dashboard))
        .route("/monitoring/metrics", get(get_metrics))
        .route("/monitoring/alert-rules", get(list_alert_rules))
        .route("/monitoring/alerts", get(list_monitoring_alerts));
    let monitoring_write = Router::new()
        .route(
            "/monitoring/alert-rules/evaluate",
            post(evaluate_alert_rules),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::MONITORING_WRITE.to_string(),
            require_permission,
        ));
    let monitoring = monitoring_read.merge(monitoring_write);

    // ── Assemble protected router ──────────────────────────────────────
    let protected = Router::new()
        .merge(auth_routes)
        .nest("/users", users)
        .merge(tenants)
        .merge(clusters)
        .merge(resources)
        .merge(topology)
        .merge(k8s)
        .merge(jobs)
        .merge(gpus)
        .merge(gpu_aliases)
        .merge(partitions)
        .merge(quotas)
        .merge(schedulers)
        .merge(datasets)
        .merge(checkpoints)
        .merge(acceleration)
        .merge(alerts)
        .merge(security)
        .merge(monitoring)
        // CSRF 双提交校验：仅对携带 access_token Cookie 的写操作生效，Bearer 通道跳过
        .route_layer(axum::middleware::from_fn(csrf_protect))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    // ── /metrics：Prometheus 文本格式端点（横切，无 JWT，对齐 Go）─────
    // Prometheus scraper 直接抓取，不挂 JWT / CSRF 中间件。
    let metrics_route = Router::new().route("/metrics", get(metrics_handler));

    // ── /health：存活/就绪探针（根级，无 /api/v1 前缀，无 JWT，对齐 Go）──
    // K8s livenessProbe/readinessProbe 直接抓取。
    let health_route = Router::new().route("/health", get(health));

    // ── Swagger UI + OpenAPI spec（横切，无 JWT，对齐 Go /api/docs）─────
    // /swagger-ui 提供交互式文档；/api-docs/openapi.json 返回原始 OpenAPI 3.0 JSON。
    let swagger = SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    let app = Router::new()
        .merge(health_route)
        .merge(metrics_route)
        .merge(swagger)
        .nest("/api/v1", public.merge(protected))
        .layer(CookieManagerLayer::new());

    // 应用核心中间件栈（request_id/request_logger/timing/metrics/security_headers/error_handler/panic_recover）
    apply_core_stack(app).with_state(state)
}
