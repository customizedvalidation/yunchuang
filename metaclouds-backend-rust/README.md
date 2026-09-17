# metaclouds-backend-rust

Rust rewrite of the Metaclouds backend, API-compatible with the Go v1 backend
(`metaclouds-backend/`). Phase 0 (Spike), Phase 1 (infrastructure skeleton),
Phase 2 (domain layer B1-B6), and Phase 3 (cross-cutting capabilities) are complete.
The project covers auth, user CRUD, tenants, clusters, resources, topology, K8s mock,
jobs, GPUs, partitions, quotas, schedulers, datasets, checkpoints, acceleration suites,
alerts, security policies, monitoring dashboards, Redis caching, scheduled tasks,
Prometheus metrics, OpenTelemetry tracing, OpenAPI docs, and Docker/K8s deployment —
**241 tests, all passing** (1 ignored Postgres smoke test).

## Architecture

```
src/
├── main.rs              # binary entrypoint (loads config, starts server, scheduler, tracing)
├── lib.rs               # library entrypoint (exposed for integration tests)
├── config.rs            # env loading + validation (~65 fields, 1:1 with Go config.go)
├── db.rs                # DatabasePool (Sqlite/Postgres), migrations, admin seed
├── error.rs             # AppError + ErrorCode (thiserror, 9+ codes)
├── response.rs          # JSON envelope (success/data/message/code/timestamp)
├── routes.rs            # router assembly — all B1-B6 domains + /metrics + /swagger-ui
├── cache/                # P3-01: Redis cache layer
│   ├── mod.rs           # Cache trait definition
│   ├── redis.rs         # RedisCache (ConnectionManager) + NoopCache fallback
│   └── session.rs        # session jti store / revocation
├── scheduler/            # P3-02: tokio-cron-scheduler
│   ├── mod.rs           # Scheduler init / shutdown
│   └── tasks.rs          # sample-training + sample-inference jobs
├── middleware/
│   ├── mod.rs           # apply_core_stack() — composes the full middleware chain
│   ├── request_id.rs    # X-Request-ID propagation + tracing span
│   ├── tracing.rs       # P3-04: trace_id/span_id injection, W3C traceparent
│   ├── request_logger.rs# method/path/status/duration_ms structured log
│   ├── metrics.rs       # P3-03: HTTP request CounterVec/HistogramVec/in-flight Gauge
│   ├── timing.rs        # X-Response-Time header
│   ├── security_headers.rs # CSP/HSTS/X-Frame-Options/… (env-aware)
│   ├── error_handler.rs # 404/405 plain-text → JSON envelope
│   └── panic_recover.rs # catch panic → 500 envelope
├── auth/
│   ├── password.rs      # argon2id hash + bcrypt dual-read (Go migration compat)
│   ├── jwt.rs            # HS256 issue/verify + Claims
│   ├── csrf.rs           # double-submit token (csrf_token cookie + X-CSRF-Token header)
│   ├── middleware.rs     # jwt_auth + require_permission + AppState + 30 permission constants
│   └── handler.rs        # login / logout / refresh / profile / csrf / change_password
├── authz/
│   └── mod.rs            # 30 permission constants + role-permission matrix
├── orm/
│   └── mod.rs            # HasTimestamps / SoftDelete / Pagination / Json<T> (GORM parity layer)
├── models/               # domain models (sqlx FromRow + Response DTOs)
│   ├── user.rs, tenant.rs, cluster.rs, resource.rs, topology.rs
│   ├── job.rs, gpu_device.rs, gpu_allocation.rs
│   ├── partition.rs, partition_permission.rs, resource_quota.rs, scheduler_integration.rs
│   ├── dataset.rs, fluid_cache.rs, checkpoint.rs
│   ├── training_config.rs, inference_config.rs, acceleration_suite.rs
│   ├── alert.rs, security_policy.rs
│   └── k8s.rs
├── services/             # business logic (pure, no HTTP)
│   ├── auth.rs, tenant.rs, user.rs
│   ├── cluster.rs, resource.rs, topology.rs, k8s.rs (MockK8sClient)
│   ├── job.rs, gpu.rs
│   ├── partition.rs, partition_permission.rs, quota.rs, scheduler.rs
│   ├── dataset.rs, checkpoint.rs, acceleration.rs
│   ├── alert.rs, security.rs, monitoring.rs (13 dashboard metrics + 16 alert rules)
│   └── fluid_cache.rs, training_config.rs, inference_config.rs
└── handlers/             # HTTP extractors + response envelope
    ├── user.rs, tenant.rs, cluster.rs, resource.rs, topology.rs, k8s.rs
    ├── job.rs, gpu.rs
    ├── partition.rs, quota.rs, scheduler.rs
    ├── dataset.rs, checkpoint.rs, acceleration.rs
    ├── alert.rs, security.rs, monitoring.rs
    └── mod.rs
```

### Middleware stack order

Aligned with Go `middlewares/stack.go::ApplyCoreStack`:

```
request_id → tracing → request_logger → metrics → timing → security_headers → error_handler → panic_recover → handler
```

CSRF (`csrf_protect`) is layered on protected routes *after* `jwt_auth`: it
only validates write methods (POST/PUT/DELETE/PATCH) when an `access_token`
cookie is present; Bearer-token clients are exempt.

### Mapping to the Go backend

| Go package             | Rust module                     |
|------------------------|---------------------------------|
| `config/`              | `src/config.rs`                 |
| `pkg/response/`        | `src/response.rs`               |
| `pkg/errors/`          | `src/error.rs`                  |
| `pkg/authz/`           | `src/authz/mod.rs`              |
| `middlewares/`         | `src/middleware/`               |
| `middlewares/jwt_auth.go` | `src/auth/middleware.rs`     |
| `middlewares/csrf.go`  | `src/auth/csrf.rs`              |
| `controllers/auth.go`  | `src/auth/handler.rs`           |
| `controllers/user.go`  | `src/handlers/user.rs`          |
| `controllers/tenant.go`| `src/handlers/tenant.rs`        |
| `controllers/cluster.go`| `src/handlers/cluster.rs`      |
| `controllers/resource.go`| `src/handlers/resource.rs`    |
| `controllers/topology.go`| `src/handlers/topology.rs`    |
| `controllers/k8s.go`   | `src/handlers/k8s.rs`           |
| `controllers/job.go`   | `src/handlers/job.rs`           |
| `controllers/gpu.go`   | `src/handlers/gpu.rs`           |
| `controllers/partition.go`| `src/handlers/partition.rs` |
| `controllers/quota.go` | `src/handlers/quota.rs`         |
| `controllers/scheduler.go`| `src/handlers/scheduler.rs` |
| `controllers/dataset.go`| `src/handlers/dataset.rs`      |
| `controllers/checkpoint.go`| `src/handlers/checkpoint.rs` |
| `controllers/acceleration.go`| `src/handlers/acceleration.rs` |
| `controllers/alert.go` | `src/handlers/alert.rs`         |
| `controllers/security.go`| `src/handlers/security.rs`    |
| `controllers/monitoring.go`| `src/handlers/monitoring.rs` |
| `models/`               | `src/models/` + `src/orm/`     |
| `migrations/`           | `migrations/`                    |

## Tech stack

- axum 0.8 + tokio (HTTP)
- sqlx 0.8 (SQLite + Postgres dual-driver, runtime queries)
- jsonwebtoken 9 (HS256) + argon2 0.5 + bcrypt 0.15 (dual-read)
- tower-cookies 0.11 (axum 0.8 / axum-core 0.5 compatible)
- validator 0.18
- redis 0.27 (ConnectionManager, P3-01)
- tokio-cron-scheduler 0.13 (P3-02)
- prometheus 0.13 (CounterVec / HistogramVec / Gauge, P3-03)
- tracing + tracing-subscriber + opentelemetry + opentelemetry_sdk (P3-04)
- utoipa 5 + utoipa-swagger-ui 9 (P3-05)
- thiserror 2 + anyhow 1
- dotenvy 0.15, chrono 0.4, uuid 1

## Quick start

```powershell
# from this directory
cargo build
cargo run          # listens on :8001
cargo test
```

On first build, dependencies are pulled via the rsproxy.cn mirror configured
in `.cargo/config.toml`.

### `just` commands

A `justfile` is provided (requires [just](https://github.com/casey/just)):

```
just fmt        # cargo fmt
just clippy     # cargo clippy --all-targets -- -D warnings
just test       # cargo test
just build      # cargo build --release
just run        # cargo run
just coverage   # cargo llvm-cov (if installed)
just migrate    # run sqlx migrations
```

### Smoke test

```powershell
# login (sets access_token + csrf_token cookies)
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8001/api/v1/auth/login `
  -ContentType 'application/json' `
  -Body '{"username":"admin","password":"Admin@123456"}'

# list users with the returned bearer token (CSRF skipped for Bearer clients)
Invoke-RestMethod -Uri http://127.0.0.1:8001/api/v1/users `
  -Headers @{ Authorization = "Bearer <token>" }
```

## Environment variables

Key variables (full list in `src/config.rs`, ~60 fields 1:1 with Go `config.go`):

| Var                       | Default                          | Notes                                      |
|---------------------------|----------------------------------|--------------------------------------------|
| `SERVER_HOST`             | `0.0.0.0`                        |                                            |
| `SERVER_PORT`             | `8001`                           | different from Go's `8000`                 |
| `SERVER_ENV`              | `development`                    | `production` enforces stricter validation  |
| `DATABASE_URL`            | `sqlite:metaclouds.db?mode=rwc` | file SQLite; tests use `sqlite::memory:`  |
| `USE_SQLITE`              | `true`                           | `false` → Postgres                          |
| `MEMORY_STORE_ENABLED`    | `true`                           | in-memory SQLite (dev only)                |
| `DATABASE_HOST/PORT/USER/PASSWORD/NAME/SSLMODE` | — | Postgres connection              |
| `JWT_SECRET`              | *(required)*                     | must be ≥ 32 chars                          |
| `JWT_EXPIRATION_HOURS`    | `24`                             | token lifetime                              |
| `JWT_REFRESH_EXPIRATION_HOURS` | `168`                       | refresh token lifetime                      |
| `COOKIE_SAME_SITE`        | `lax`                            | `strict`/`lax`/`none` (none requires prod) |
| `LOG_LEVEL`               | `info`                           | tracing filter                              |
| `LOG_FORMAT`              | `json`                           | `json`/`text`                               |
| `REDIS_ENABLED`           | `false`                          |                                            |
| `PROMETHEUS_ENABLED`      | `true`                           |                                            |
| `ALLOW_PUBLIC_REGISTRATION`| `false`                         |                                            |
| `RATE_LIMIT_ENABLED`      | `true`                           |                                            |
| `DEFAULT_PAGE_SIZE`       | `10`                             |                                            |
| `MAX_PAGE_SIZE`           | `100`                            |                                            |

## Endpoints

All routes are under `/api/v1`. JWT = `Authorization: Bearer <token>` or
`access_token` cookie. Permission column shows the RBAC permission required
(beyond JWT); `admin` role short-circuits all permissions.

### Auth (B1)

| Method | Path                     | Auth | Notes |
|--------|--------------------------|------|-------|
| POST   | `/auth/login`            | public | sets `access_token` + `csrf_token` cookies |
| POST   | `/auth/logout`           | JWT | clears cookies |
| POST   | `/auth/refresh`          | JWT | re-issues token + cookies |
| GET    | `/auth/profile`          | JWT | current user |
| GET    | `/auth/csrf`             | JWT | returns `csrf_token` from cookie |
| PUT    | `/auth/change-password`  | JWT | self-service password change |

### Users (B1, admin only)

| Method | Path               | Auth | Notes |
|--------|--------------------|------|-------|
| GET    | `/users`           | JWT + `admin` | `?page=&page_size=&search=` |
| POST   | `/users`           | JWT + `admin` | 201 |
| GET    | `/users/{id}`      | JWT + `admin` | |
| PUT    | `/users/{id}`      | JWT + `admin` | |
| DELETE | `/users/{id}`      | JWT + `admin` | soft-delete |

### Tenants (B1)

| Method | Path           | Auth | Notes |
|--------|----------------|------|-------|
| GET    | `/tenants`     | JWT + `tenant:read` | paginated |
| POST   | `/tenants`     | JWT + `tenant:write` | 201 |
| GET    | `/tenants/{id}` | JWT + `tenant:read` | |
| PUT    | `/tenants/{id}` | JWT + `tenant:write` | |
| DELETE | `/tenants/{id}` | JWT + `tenant:write` | 204 soft-delete |

### Clusters (B2)

| Method | Path              | Auth | Notes |
|--------|-------------------|------|-------|
| GET    | `/clusters`       | JWT | list + search |
| POST   | `/clusters`       | JWT + `cluster:write` | 201 |
| GET    | `/clusters/{id}`  | JWT | |
| PUT    | `/clusters/{id}`  | JWT + `cluster:write` | |
| DELETE | `/clusters/{id}`  | JWT + `cluster:write` | 204 |

### Resources (B2)

| Method | Path              | Auth | Notes |
|--------|-------------------|------|-------|
| GET    | `/resources`      | JWT | list + filter |
| POST   | `/resources`      | JWT + `resource:write` | 201 |
| GET    | `/resources/{id}` | JWT | |
| PUT    | `/resources/{id}` | JWT + `resource:write` | |
| DELETE | `/resources/{id}` | JWT + `resource:write` | 204 |

### Topology (B2)

| Method | Path                      | Auth | Notes |
|--------|---------------------------|------|-------|
| GET    | `/topology`               | JWT | node list |
| POST   | `/topology`               | JWT + `topology:write` | 201 |
| GET    | `/topology/{id}`          | JWT | |
| PUT    | `/topology/{id}`          | JWT + `topology:write` | |
| DELETE | `/topology/{id}`          | JWT + `topology:write` | 204 |
| GET    | `/topology/nodes`         | JWT | Vue3 alias |
| POST   | `/topology/nodes`         | JWT + `topology:write` | alias |
| GET    | `/topology/nodes/{id}`    | JWT | alias |
| PUT    | `/topology/nodes/{id}`    | JWT + `topology:write` | alias |
| DELETE | `/topology/nodes/{id}`    | JWT + `topology:write` | alias |

### K8s Mock (B2)

| Method | Path                            | Auth | Notes |
|--------|---------------------------------|------|-------|
| GET    | `/k8s/clusters/{id}/pods`       | JWT | mock pods |
| GET    | `/k8s/clusters/{id}/nodes`      | JWT | mock nodes |
| GET    | `/k8s/clusters/{id}/health`     | JWT | mock health |

### Jobs (B3)

| Method | Path                | Auth | Notes |
|--------|---------------------|------|-------|
| GET    | `/jobs`             | JWT | list + filter |
| GET    | `/jobs/stats`       | JWT | count by status |
| POST   | `/jobs`             | JWT + `job:write` | 201 |
| GET    | `/jobs/{id}`        | JWT | |
| PUT    | `/jobs/{id}`        | JWT + `job:write` | state machine |
| DELETE | `/jobs/{id}`        | JWT + `job:write` | 204 |
| POST   | `/jobs/{id}/cancel` | JWT + `job:write` | |

### GPUs (B3)

| Method | Path                          | Auth | Notes |
|--------|-------------------------------|------|-------|
| GET    | `/gpus`                       | JWT | device list |
| POST   | `/gpus`                       | JWT + `gpu:write` | 201 |
| GET    | `/gpus/{id}`                  | JWT | |
| PUT    | `/gpus/{id}`                  | JWT + `gpu:write` | |
| DELETE | `/gpus/{id}`                  | JWT + `gpu:write` | 204 |
| GET    | `/gpus/allocations`           | JWT | allocation records |
| POST   | `/gpus/allocations`           | JWT + `job:write` | 201 allocate |
| DELETE | `/gpus/allocations/{id}`      | JWT + `job:write` | 204 release |
| GET    | `/gpus/utilization`           | JWT | utilization summary |

#### GPU Vue3 aliases (`/gpu/...`)

| Method | Path                              | Auth |
|--------|-----------------------------------|------|
| GET    | `/gpu/devices`                    | JWT |
| POST   | `/gpu/devices`                    | JWT + `gpu:write` |
| GET    | `/gpu/devices/{id}`               | JWT |
| PUT    | `/gpu/devices/{id}`               | JWT + `gpu:write` |
| DELETE | `/gpu/devices/{id}`               | JWT + `gpu:write` |
| GET    | `/gpu/allocations`                | JWT |
| POST   | `/gpu/allocations`                | JWT + `job:write` |
| POST   | `/gpu/allocations/{id}/release`   | JWT + `job:write` |
| GET    | `/gpu/utilization`                | JWT |

### Partitions (B4)

| Method | Path                              | Auth | Notes |
|--------|-----------------------------------|------|-------|
| GET    | `/partitions`                     | JWT | list + filter |
| POST   | `/partitions`                     | JWT + `partition:write` | 201 |
| GET    | `/partitions/{id}`                | JWT | |
| PUT    | `/partitions/{id}`                | JWT + `partition:write` | |
| DELETE | `/partitions/{id}`                | JWT + `partition:write` | 204 |
| GET    | `/partitions/{id}/resources`      | JWT | resource usage |
| POST   | `/partitions/{id}/permissions`    | JWT + `partition:write` | grant |
| DELETE | `/partitions/{id}/permissions/{perm_id}` | JWT + `partition:write` | revoke |

### Quotas (B4)

| Method | Path                  | Auth | Notes |
|--------|-----------------------|------|-------|
| GET    | `/quotas`             | JWT | list + filter |
| POST   | `/quotas`             | JWT + `quota:write` | 201 |
| GET    | `/quotas/{id}`        | JWT | |
| PUT    | `/quotas/{id}`        | JWT + `quota:write` | |
| DELETE | `/quotas/{id}`        | JWT + `quota:write` | 204 |
| POST   | `/quotas/{id}/check`  | JWT | check against usage |

### Schedulers (B4)

| Method | Path                          | Auth | Notes |
|--------|-------------------------------|------|-------|
| GET    | `/schedulers`                 | JWT | list + filter |
| POST   | `/schedulers`                 | JWT + `scheduler:write` | 201 |
| GET    | `/schedulers/{id}`            | JWT | |
| PUT    | `/schedulers/{id}`            | JWT + `scheduler:write` | |
| DELETE | `/schedulers/{id}`            | JWT + `scheduler:write` | 204 |
| POST   | `/schedulers/{id}/sync`       | JWT + `scheduler:write` | mock sync |
| POST   | `/schedulers/{id}/test-connection` | JWT + `scheduler:write` | mock test |

### Datasets (B5)

| Method | Path              | Auth | Notes |
|--------|-------------------|------|-------|
| GET    | `/datasets`       | JWT | list + filter |
| POST   | `/datasets`       | JWT + `dataset:write` | 201 |
| GET    | `/datasets/{id}`  | JWT | |
| PUT    | `/datasets/{id}`  | JWT + `dataset:write` | |
| DELETE | `/datasets/{id}`  | JWT + `dataset:write` | 204 |

### Checkpoints (B5)

| Method | Path                | Auth | Notes |
|--------|---------------------|------|-------|
| GET    | `/checkpoints`      | JWT | list + filter |
| POST   | `/checkpoints`      | JWT + `checkpoint:write` | 201 |
| GET    | `/checkpoints/{id}` | JWT | |
| PUT    | `/checkpoints/{id}` | JWT + `checkpoint:write` | |
| DELETE | `/checkpoints/{id}` | JWT + `checkpoint:write` | 204 |

### Acceleration (B5)

| Method | Path                        | Auth | Notes |
|--------|-----------------------------|------|-------|
| GET    | `/acceleration`             | JWT | suite list |
| POST   | `/acceleration`             | JWT + `acceleration:write` | 201 |
| GET    | `/acceleration/{id}`        | JWT | |
| PUT    | `/acceleration/{id}`        | JWT + `acceleration:write` | |
| DELETE | `/acceleration/{id}`        | JWT + `acceleration:write` | 204 |
| POST   | `/acceleration/{id}/start`  | JWT + `acceleration:write` | |
| POST   | `/acceleration/{id}/stop`   | JWT + `acceleration:write` | |

### Alerts (B6)

| Method | Path                      | Auth | Notes |
|--------|---------------------------|------|-------|
| GET    | `/alerts`                 | JWT | list + filter |
| GET    | `/alerts/stats`           | JWT | alert stats |
| POST   | `/alerts`                 | JWT + `alert:write` | 201 |
| GET    | `/alerts/{id}`            | JWT | |
| PUT    | `/alerts/{id}`            | JWT + `alert:write` | |
| DELETE | `/alerts/{id}`            | JWT + `alert:write` | 204 |
| POST   | `/alerts/{id}/acknowledge` | JWT + `alert:write` | |
| POST   | `/alerts/{id}/resolve`    | JWT + `alert:write` | |

### Security Policies (B6)

| Method | Path                              | Auth | Notes |
|--------|-----------------------------------|------|-------|
| GET    | `/security/policies`              | JWT | list + filter |
| POST   | `/security/policies`              | JWT + `security:write` | 201 |
| GET    | `/security/policies/{id}`         | JWT | |
| PUT    | `/security/policies/{id}`         | JWT + `security:write` | |
| DELETE | `/security/policies/{id}`         | JWT + `security:write` | 204 |
| POST   | `/security/policies/{id}/enable`  | JWT + `security:write` | |
| POST   | `/security/policies/{id}/disable` | JWT + `security:write` | |

### Monitoring (B6)

| Method | Path                                  | Auth | Notes |
|--------|---------------------------------------|------|-------|
| GET    | `/monitoring/dashboard`               | JWT | 13 business metrics |
| GET    | `/monitoring/metrics`                 | JWT | metrics, `?name=` filter |
| GET    | `/monitoring/alert-rules`             | JWT | 16 alert rule definitions |
| POST   | `/monitoring/alert-rules/evaluate`    | JWT + `monitoring:write` | evaluate all rules |

### Observability endpoints (P3)

| Method | Path                  | Auth | Notes |
|--------|-----------------------|------|-------|
| GET    | `/metrics`            | public | Prometheus text format, 13 business + 3 HTTP metrics |
| GET    | `/swagger-ui/`        | public | Interactive Swagger UI |
| GET    | `/api-docs/openapi.json` | public | OpenAPI 3.1.0 spec (61 paths / 107 methods) |

## Observability & Operations

### Redis Cache Layer (P3-01)

- `Cache` trait in `src/cache/mod.rs` with two implementations:
  - **`RedisCache`**: backed by `redis::aio::ConnectionManager`, key prefix `metaclouds:`, 5s connection timeout (aligned with Go)
  - **`NoopCache`**: graceful fallback when Redis is disabled or unreachable (all operations are no-ops)
- Session jti storage / revocation in `src/cache/session.rs`
- Config: `REDIS_ENABLED`, `REDIS_URL`, `REDIS_HOST/PORT/PASSWORD/DB`
- 16 tests covering NoopCache behavior, URL building, prefix handling, TTL conversion, jti lifecycle

### Scheduled Tasks (P3-02)

- Built on `tokio-cron-scheduler`, 7-field cron expressions (seconds first), UTC timezone
- Two default jobs (registered only in non-production):
  - `sample-training`: `0 */30 * * * * *` (every 30 minutes)
  - `sample-inference`: `0 0 */2 * * * *` (every 2 hours)
- Panic isolation: a task panicking does not kill the scheduler loop
- Startup / shutdown integration in `main.rs`
- Migration reference: `docs/cron-migration-reference.md`
- 8 tests covering trigger alignment, dev/prod job registration, panic isolation, start/shutdown

### Prometheus Metrics (P3-03)

- **`GET /metrics`** endpoint (no JWT required, aligned with Go)
- 13 business Gauges with `metaclouds_` prefix:
  `total_users`, `active_users`, `total_tenants`, `total_clusters`, `total_resources`,
  `total_jobs`, `running_jobs`, `total_gpus`, `allocated_gpus`, `total_datasets`,
  `total_alerts`, `active_alerts`, `system_uptime`
- 3 HTTP metrics via middleware:
  - `http_requests_total` (CounterVec, labels: method, path, status)
  - `http_request_duration_seconds` (HistogramVec, buckets: 0.005~10s)
  - `http_requests_in_flight` (Gauge)
- Path normalization: dynamic segments (e.g. `/users/{id}`) are collapsed to template paths
- 10 tests covering endpoint auth, metric presence, histogram buckets, label cardinality

### Tracing + OpenTelemetry (P3-04)

- **JSON structured logs**: `timestamp`, `level`, `target`, `message`, `trace_id`, `span_id`, `request_id`
- **OpenTelemetry OTLP gRPC exporter**: disabled by default (`OTEL_EXPORTER_OTLP_ENDPOINT` unset → graceful no-op)
- **W3C traceparent header propagation**: incoming `traceparent` header is inherited; otherwise a new trace_id is generated (32 hex chars)
- **`X-Trace-Id` response header**: injected on every response by the tracing middleware
- Middleware stack order: `request_id → tracing → request_logger → metrics → ...`
- Slow query logging threshold configurable via `DB_SLOW_QUERY_THRESHOLD_MS`
- 8 tests covering init/shutdown, OTEL disabled fallback, trace_id format, traceparent inheritance, response header presence

### OpenAPI Documentation (P3-05)

- **`GET /swagger-ui/`**: interactive Swagger UI (utoipa-swagger-ui 9, vendored for offline builds)
- **`GET /api-docs/openapi.json`**: OpenAPI 3.1.0 spec (~291KB)
- 107 handlers annotated with `#[utoipa::path]`, all request/response structs derive `ToSchema`
- **61 paths / 107 methods** (Go reference: 28 paths / 44 methods)
- Diff from Go: Rust adds users/alerts/k8s/acceleration start-stop domains; Go has datasets/caches CRUD, clusters/status, jobs/submit not yet implemented (Phase 2 backlog)
- 5 tests covering swagger UI accessibility, spec validity, path/method counts, key schemas

### Docker Deployment (P3-06)

- **Multi-stage Dockerfile**: `rust:1.81-alpine` builder → `alpine:3.20` runtime
- Runs as non-root UID 10001
- `.dockerignore` excludes `target/`, `.env`, `*.db`
- **Estimated image size**: 31–41 MB (critical at 40MB; LTO/strip/UPX optimizations recommended)
- `docker-compose.yml`: backend + postgres + redis, port mapping `8001:8000`
- `docker-compose.prod.yml`: production overrides

### Kubernetes Deployment (P3-06)

- 14 YAML manifests in `k8s/` (00-namespace → 13-kustomization)
- **Deployment**: three probes (liveness/readiness/startup via `/metrics`), non-root, topology spread
- **HPA**: autoscaling on CPU/memory
- **PDB**: pod disruption budget
- **NetworkPolicy**: ingress/egress rules
- **Ingress**: TLS termination
- **ServiceMonitor + PrometheusRule**: 16 alert rules
- Validation script: `scripts/validate-k8s-yaml.ps1` (63 structural checks, all PASS)

## Conventions

### Response envelope

Success:
```json
{"success":true,"data":{...},"timestamp":1700000000}
```
Error:
```json
{"success":false,"message":"...","code":"UNAUTHORIZED","timestamp":1700000000}
```
`timestamp` is Unix seconds (i64). On success `data` is always present
(`null` when empty); on error `data` is omitted and `message`/`code` are set.

### Error codes → HTTP status

| code                     | HTTP |
|--------------------------|------|
| `BAD_REQUEST`            | 400  |
| `UNAUTHORIZED`           | 401  |
| `FORBIDDEN`              | 403  |
| `NOT_FOUND`              | 404  |
| `CONFLICT`               | 409  |
| `VALIDATION_ERROR`       | 400  |
| `RATE_LIMIT_EXCEEDED`    | 429  |
| `INTERNAL_SERVER_ERROR`   | 500  |
| `SERVICE_UNAVAILABLE`    | 503  |

### CSRF (double-submit cookie)

- Cookie: `csrf_token` (non-HttpOnly, readable by same-origin JS)
- Header: `X-CSRF-Token`
- Validated only on POST/PUT/DELETE/PATCH when `access_token` cookie is present
- Bearer-token clients (no cookie) are exempt — they are not vulnerable to CSRF
- Comparison is constant-time

### RBAC

Roles: `admin` / `manager` / `user`. `admin` short-circuits every permission.
30 permission constants in `src/authz/mod.rs` (1:1 with Go `pkg/authz/authz.go`):

```
cluster:read/write, resource:read/write, job:read/write/submit,
tenant:read/write, monitoring:read/write, acceleration:read/write,
security:read/write, admin, gpu:read/write, partition:read/write,
quota:read/write, scheduler:read/write, topology:read/write,
dataset:read/write, checkpoint:read/write
```

### ORM layer (GORM parity)

`src/orm/mod.rs` provides traits that replace GORM's automatic behavior:

- **`HasTimestamps`**: `created_at`/`updated_at` auto-filled on insert/update
- **`SoftDelete`**: `deleted_at IS NULL` filter by default; DELETE → UPDATE
- **`PaginationParams`/`PaginatedResult`**: page/page_size/total/total_pages
- **`Json<T>`**: newtype for JSON columns (TEXT on SQLite, JSONB on Postgres)

## Differences from the Go backend (intentional)

- **Password hashing**: Go uses bcrypt; Rust uses argon2id for new hashes.
  **bcrypt dual-read is implemented**: `verify_password` detects `$2a$/$2b$/$2y$`
  prefixes and verifies via bcrypt, enabling seamless migration of existing users.
- **Path syntax**: axum 0.8 uses `{id}` instead of Go gin's `:id`.
- **Port**: Go runs on 8000, Rust runs on 8001.
- **DB driver**: SQLite is the default for local dev; Postgres is fully supported
  via `USE_SQLITE=false` and the `DatabasePool` enum.

## CI

`.github/workflows/ci-rust.yml` runs three jobs on every push/PR to `main`:

1. **lint-test**: `cargo fmt --check` → `cargo clippy --all-targets -- -D warnings` → `cargo test`
2. **coverage**: `cargo-llvm-cov` (informational, does not block PR)
3. **release-build**: `cargo build --release`

All jobs use `actions/cache` for the cargo registry and `target/` directory.

## Test coverage

**241 tests** across 32 suites (Phase 0/1: ~73 + Phase 2 B1-B6: ~120 + Phase 3: 47 + lib unit tests: ~13), all passing:

| Suite              | Tests | Coverage area                        |
|--------------------|-------|--------------------------------------|
| `lib` (unit)       | 13    | auth/jwt/password/csrf, authz matrix, cache session |
| `api_test`         | 11    | login / user CRUD / RBAC / envelope  |
| `auth_test`        | 6     | JWT issue/verify / bcrypt dual-read  |
| `config_test`      | 18    | defaults / validation / prod rules    |
| `db_test`          | 5     | pool / migrations / seed             |
| `error_test`       | 4     | 6 error codes + envelope shape       |
| `middleware_test`  | 4     | request_id / timing / security_headers |
| `orm_test`         | 8     | timestamps / soft-delete / pagination / models |
| `rbac_test`        | 5     | permission matrix allow/deny          |
| `b1_auth_test`     | 13    | login / logout / refresh / profile / change-password / lockout |
| `b1_tenant_test`   | 12    | tenant CRUD / pagination / RBAC       |
| `b2_cluster_test`  | 7     | cluster CRUD / search / soft-delete   |
| `b2_k8s_test`      | 2     | mock pods/nodes/health shape + auth   |
| `b2_resource_test` | 6     | resource CRUD / filter / RBAC         |
| `b2_topology_test` | 3     | node CRUD / list filter              |
| `b3_gpu_test`      | 8     | GPU device CRUD / allocate-release / filter |
| `b3_job_test`      | 12    | job CRUD / state machine / cancel / stats / RBAC |
| `b4_partition_test`| 7     | partition CRUD / grant-revoke / resources |
| `b4_quota_test`    | 6     | quota CRUD / check / filter / RBAC    |
| `b4_scheduler_test`| 5     | scheduler CRUD / test-connection / sync |
| `b5_acceleration_test` | 7 | suite CRUD / start-stop / pagination |
| `b5_checkpoint_test`| 4    | checkpoint CRUD / filter by job       |
| `b5_dataset_test`  | 7     | dataset CRUD / filter / pagination    |
| `b5_fluid_training_test` | 2 | FluidCache + TrainingConfig basic CRUD |
| `b6_alert_test`    | 8     | alert CRUD / acknowledge-resolve / stats / filter |
| `b6_monitoring_test`| 5    | dashboard 13 metrics / metrics filter / 16 alert rules / evaluate |
| `b6_security_test` | 6     | policy CRUD / enable-disable / filter / RBAC |
| `p3_redis_cache_test` | 16 | Cache trait / NoopCache / session jti / URL building / prefix / TTL |
| `p3_cron_test`     | 8     | cron expressions / dev-prod job registration / panic isolation / start-shutdown |
| `p3_metrics_test`  | 10    | /metrics endpoint / 13 business gauges / HTTP histogram / labels / no-auth |
| `p3_tracing_test`  | 8     | JSON logs / X-Trace-Id / traceparent inheritance / OTEL disabled fallback |
| `p3_openapi_test` | 5     | swagger UI / openapi.json / path+method counts / key schemas |
| `postgres_smoke_test` | 0 (1 ignored) | Postgres connect — `#[ignore]`, runs in CI |
