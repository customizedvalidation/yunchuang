# metaclouds-backend-rust

Rust rewrite of the Metaclouds backend, API-compatible with the Go v1 backend
(`metaclouds-backend/`). Phase 0 (Spike) and Phase 1 (infrastructure skeleton)
are complete. The project currently covers auth, user CRUD, and the full
cross-cutting middleware stack; domain modules (clusters, jobs, resources, …)
are planned for Phase 2.

## Architecture

```
src/
├── main.rs              # binary entrypoint (loads config, starts server)
├── lib.rs               # library entrypoint (exposed for integration tests)
├── config.rs            # env loading + validation (~60 fields, 1:1 with Go config.go)
├── db.rs                # DatabasePool (Sqlite/Postgres), migrations, admin seed
├── error.rs             # AppError + ErrorCode (thiserror, 9+ codes)
├── response.rs          # JSON envelope (success/data/message/code/timestamp)
├── routes.rs            # router assembly (public vs protected, CSRF layer)
├── middleware/
│   ├── mod.rs           # apply_core_stack() — composes the full middleware chain
│   ├── request_id.rs    # X-Request-ID propagation + tracing span
│   ├── request_logger.rs# method/path/status/duration_ms structured log
│   ├── timing.rs        # X-Response-Time header
│   ├── security_headers.rs # CSP/HSTS/X-Frame-Options/… (env-aware)
│   ├── error_handler.rs # 404/405 plain-text → JSON envelope
│   └── panic_recover.rs # catch panic → 500 envelope
├── auth/
│   ├── password.rs      # argon2id hash + bcrypt dual-read (Go migration compat)
│   ├── jwt.rs           # HS256 issue/verify + Claims (user_id/username/email/role/tenant_id/exp/iat/jti)
│   ├── csrf.rs          # double-submit token (csrf_token cookie + X-CSRF-Token header)
│   ├── middleware.rs    # jwt_auth + require_permission + AppState
│   └── handler.rs       # login / logout / refresh / profile / csrf
├── authz/
│   └── mod.rs           # 30 permission constants + Role enum + role-permission matrix
├── orm/
│   └── mod.rs           # HasTimestamps / SoftDelete / Pagination / Json<T> (GORM parity layer)
├── models/
│   ├── mod.rs
│   ├── user.rs          # User + UserResponse + DTOs (HasTimestamps + SoftDelete)
│   ├── tenant.rs        # Tenant + CRUD (ORM layer demo)
│   └── cluster.rs       # Cluster + CRUD (ORM layer demo)
└── handlers/
    └── user.rs          # CRUD + pagination + search
```

### Middleware stack order

Aligned with Go `middlewares/stack.go::ApplyCoreStack`:

```
request_id → request_logger → timing → security_headers → error_handler → panic_recover → handler
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
| `models/`              | `src/models/` + `src/orm/`     |
| `migrations/`          | `migrations/`                    |

## Tech stack

- axum 0.8 + tokio (HTTP)
- sqlx 0.8 (SQLite + Postgres dual-driver, runtime queries)
- jsonwebtoken 9 (HS256) + argon2 0.5 + bcrypt 0.15 (dual-read)
- tower-cookies 0.11 (axum 0.8 / axum-core 0.5 compatible)
- validator 0.18
- tracing + tracing-subscriber (JSON)
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

| Method | Path                      | Auth                | Notes                                        |
|--------|---------------------------|---------------------|----------------------------------------------|
| POST   | `/api/v1/auth/login`      | public              | sets `access_token` + `csrf_token` cookies  |
| POST   | `/api/v1/auth/logout`     | JWT                 | clears cookies                               |
| POST   | `/api/v1/auth/refresh`    | JWT                 | re-issues token + cookies                    |
| GET    | `/api/v1/auth/profile`    | JWT                 | current user                                 |
| GET    | `/api/v1/auth/csrf`       | JWT                 | returns `csrf_token` from cookie             |
| GET    | `/api/v1/users`           | JWT + `admin`       | `?page=&page_size=&search=`                  |
| POST   | `/api/v1/users`           | JWT + `admin`       | 201 on success                               |
| GET    | `/api/v1/users/{id}`      | JWT + `admin`       | 404 if missing                               |
| PUT    | `/api/v1/users/{id}`      | JWT + `admin`       | partial update                               |
| DELETE | `/api/v1/users/{id}`      | JWT + `admin`       | 200 on success (soft-delete in ORM layer)   |

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

Phase 1 delivers 61 tests across 8 suites:

| Suite          | Tests | Coverage area                        |
|----------------|-------|--------------------------------------|
| `api_test`     | 11    | login / user CRUD / RBAC / envelope |
| `auth_test`    | 6     | JWT issue/verify / bcrypt dual-read |
| `config_test`  | 18    | defaults / validation / prod rules  |
| `db_test`      | 5     | pool / migrations / seed             |
| `error_test`   | 4     | 6 error codes + envelope shape       |
| `middleware_test` | 4  | request_id / timing / security_headers |
| `orm_test`     | 8     | timestamps / soft-delete / pagination / 3 models |
| `rbac_test`    | 5     | permission matrix allow/deny         |
