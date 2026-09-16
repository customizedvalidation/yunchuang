# metaclouds-backend-rust

Rust spike PoC of the Metaclouds backend, intended to be API-compatible with the
Go v1 backend (`metaclouds-backend/`) on the endpoints listed below. This repo
is a **spike**: it demonstrates that the Go contract (envelope shape, error
codes, JWT claims, RBAC, user CRUD) can be implemented on the Tokio/axum
stack. It is not production-ready.

## Architecture

```
src/
├── main.rs            # binary entrypoint (loads config, starts server)
├── lib.rs             # library entrypoint (exposed for integration tests)
├── config.rs          # env loading + validation
├── db.rs              # SQLite pool, migrations, admin seed
├── error.rs           # AppError + ErrorCode (thiserror)
├── response.rs        # JSON envelope (success/data/timestamp)
├── routes.rs          # router assembly (public vs protected)
├── middleware/
│   ├── request_id.rs  # X-Request-ID propagation
│   └── panic_recover.rs
├── auth/
│   ├── password.rs    # argon2id hashing
│   ├── jwt.rs         # HS256 issue/verify + Claims
│   ├── middleware.rs  # jwt_auth + require_permission + AppState
│   └── handler.rs     # login / logout / profile
├── models/
│   └── user.rs        # User row + UserResponse + create/update DTOs
└── handlers/
    └── user.rs        # CRUD + pagination + search
```

### Mapping to the Go backend

| Go package             | Rust module              |
|------------------------|--------------------------|
| `config/`              | `src/config.rs`          |
| `pkg/response/`        | `src/response.rs`        |
| `pkg/errors/`          | `src/error.rs`           |
| `middlewares/`         | `src/middleware/`        |
| `middlewares/auth.go`  | `src/auth/middleware.rs` |
| `controllers/auth.go`  | `src/auth/handler.rs`   |
| `controllers/user.go`  | `src/handlers/user.rs`   |
| `models/user.go`      | `src/models/user.rs`     |
| `migrations/`          | `migrations/`            |

## Tech stack

- axum 0.8 + tokio (HTTP)
- sqlx 0.8 (SQLite, runtime queries — no compile-time DB dependency)
- jsonwebtoken 9 (HS256) + argon2 0.5
- tower-cookies 0.11 (axum 0.8 / axum-core 0.5 compatible; 0.10 targets axum-core 0.4)
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

### Smoke test

```powershell
# login
Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8001/api/v1/auth/login `
  -ContentType 'application/json' `
  -Body '{"username":"admin","password":"Admin@123456"}'

# list users with the returned bearer token
Invoke-RestMethod -Uri http://127.0.0.1:8001/api/v1/users `
  -Headers @{ Authorization = "Bearer <token>" }
```

## Environment variables

| Var                | Default                  | Notes                                  |
|--------------------|--------------------------|----------------------------------------|
| `SERVER_HOST`      | `0.0.0.0`                |                                        |
| `SERVER_PORT`      | `8001`                   | different from Go's `8000`              |
| `DATABASE_URL`     | `sqlite:metaclouds.db?mode=rwc` | file SQLite; tests use `sqlite::memory:` |
| `JWT_SECRET`       | *(required)*             | must be ≥ 32 chars                      |
| `JWT_EXPIRES_SECONDS` | `86400`               | token lifetime                          |
| `LOG_LEVEL`        | `info`                   | tracing filter                          |

## Endpoints (spike scope)

| Method | Path                   | Auth                | Notes                                  |
|--------|------------------------|---------------------|----------------------------------------|
| POST   | `/api/v1/auth/login`   | public              | sets `access_token` + `csrf_token` cookies |
| POST   | `/api/v1/auth/logout`  | public              | clears cookies                         |
| GET    | `/api/v1/auth/profile` | JWT                 | current user                           |
| GET    | `/api/v1/users`        | JWT + `admin`       | `?page=&page_size=&search=`            |
| POST   | `/api/v1/users`        | JWT + `admin`       | 201 on success                         |
| GET    | `/api/v1/users/:id`    | JWT + `admin`       | 404 if missing                         |
| PUT    | `/api/v1/users/:id`    | JWT + `admin`       | partial update                         |
| DELETE | `/api/v1/users/:id`    | JWT + `admin`       | 200 on success                         |

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
`timestamp` is Unix seconds (i64). `data`/`message`/`code` are omitted when
not applicable (`skip_serializing_if = Option::is_none`).

### Error codes → HTTP status

| code                    | HTTP |
|-------------------------|------|
| `BAD_REQUEST`           | 400  |
| `UNAUTHORIZED`          | 401  |
| `FORBIDDEN`             | 403  |
| `NOT_FOUND`             | 404  |
| `CONFLICT`              | 409  |
| `INTERNAL_SERVER_ERROR`| 500  |
| `VALIDATION_ERROR`      | 400  |

### RBAC

Roles: `admin` / `manager` / `user`. `admin` short-circuits every permission.
User CRUD requires the `admin` permission string. The full permission list
(`cluster:read`, `job:submit`, `gpu:write`, …) lives in
`src/auth/middleware.rs::permissions`.

## Differences from the Go backend (intentional)

- **Password hashing**: Go uses bcrypt; this spike uses argon2id. Because the
  spike starts from an empty SQLite database there is no migration path. If
  this is ever pointed at a production user table, plan a gradual
  re-hash-on-login strategy.
- **DB**: Go uses PostgreSQL; this spike uses SQLite for local ergonomics.
- **Port**: Go runs on 8000, this spike runs on 8001.

## CI

`.github/workflows/ci-rust.yml` runs `fmt --check`, `clippy -D warnings`,
`cargo test` and a release build on every push/PR to `main`.
