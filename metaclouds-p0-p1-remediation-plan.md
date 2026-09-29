# Metaclouds P0/P1 差距代码改造方案

> 对应报告：`metaclouds-architecture-review-2026-09-30.md`
> 日期：2026-09-30
> 范围：P0（Postgres 移植、分布式协调）+ P1（消息总线、审计日志、强制改密、防爆破、前端门禁）
> 原则：每项给出**受影响文件**、**改造步骤**、**关键代码片段**、**验收标准**

---

## 总览

| 编号 | 项目 | 优先级 | 预估改动文件数 | 风险 |
|---|---|---|---|---|
| P0-1 | Postgres 请求层移植 | 阻断 | ~40 | 高 |
| P0-2 | 分布式限流 + 调度选主 | 阻断 | 6 | 中 |
| P1-1 | 异步消息总线 | 高 | 10 | 中 |
| P1-2 | 审计日志中间件 | 高 | 4 | 低 |
| P1-3 | 默认管理员强制改密 | 高 | 5 | 低 |
| P1-4 | 登录防爆破 | 高 | 4 | 低 |
| P1-5 | 前端质量门禁硬化 | 高 | 6 | 低 |

---

## P0-1：Postgres 请求层移植

### 现状

- `AppState.pool` 类型为 `sqlx::SqlitePool`（[auth/middleware.rs:19](file:///workspace/metaclouds-backend-rust/src/auth/middleware.rs#L19)）
- `main.rs` 第 48-53 行：Postgres 连接成功后请求层直接报错退出
- 所有 handler/service 使用 `?N` 占位符与 `last_insert_rowid()`，为 SQLite 方言
- `db.rs` 已有 `DatabasePool` 枚举与 Postgres 迁移目录（`migrations/postgres/`）

### 改造策略

采用 **sqlx `Any` 运行时多驱动 + 方言适配层**，避免为每个查询写两套 SQL。`AnyPool` 支持 SQLite 与 Postgres 运行时切换，`?` 占位符由 sqlx 自动翻译为 `$N`。

### 受影响文件

```
src/db.rs                    # DatabasePool → 持有 AnyPool
src/auth/middleware.rs       # AppState.pool 类型变更
src/services/*.rs            # 全部查询改用 AnyPool
src/handlers/*.rs            # 间接影响
src/models/*.rs              # 类型映射调整
```

### 改造步骤

#### 步骤 1：将连接池统一为 AnyPool

修改 `src/db.rs`：

```rust
// 替换 DatabasePool 枚举
use sqlx::any::{AnyPool, AnyPoolOptions, AnyConnectOptions};

pub struct DatabasePool {
    pub pool: AnyPool,
    pub dialect: DbDialect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbDialect { Sqlite, Postgres }

impl DatabasePool {
    pub async fn connect(config: &Config) -> AppResult<Self> {
        let (url, dialect) = if wants_postgres(config) {
            (config.get_database_url(), DbDialect::Postgres)
        } else {
            (config.database_url.clone(), DbDialect::Sqlite)
        };

        let pool = AnyPoolOptions::new()
            .max_connections(100)
            .min_connections(20)
            .max_lifetime(Duration::from_secs(300))
            .idle_timeout(Duration::from_secs(60))
            .acquire_timeout(Duration::from_secs(10))
            .connect(&url)
            .await
            .map_err(|e| AppError::with_source(
                ErrorCode::InternalServerError, "failed to connect database", e))?;

        Ok(Self { pool, dialect })
    }
}
```

#### 步骤 2：修改 AppState

修改 `src/auth/middleware.rs`：

```rust
use crate::db::DatabasePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: DatabasePool,        // 原 sqlx::SqlitePool
    pub config: Arc<Config>,
}
```

#### 步骤 3：main.rs 移除 Postgres 阻断

修改 `src/main.rs`，删除第 48-53 行的错误返回，直接使用 `DatabasePool`：

```rust
let pool = if db::wants_postgres(&config) {
    let db_pool = db::DatabasePool::connect(&config).await?;
    db::run_migrations(&db_pool).await?;
    db::seed_admin_if_empty(&db_pool).await?;
    db_pool
} else {
    let db_pool = db::DatabasePool::connect(&config).await?;
    db::run_migrations(&db_pool).await?;
    db::seed_admin_if_empty(&db_pool).await?;
    db_pool
};
```

#### 步骤 4：SQL 占位符统一为 `?`

`AnyPool` 下 `?` 占位符会被自动翻译。需将所有 `$N`（若有）改回 `?`，并移除 `last_insert_rowid()` 调用。

对于需要返回自增 ID 的 INSERT，改为：

```rust
// 原 SQLite 写法
let id = sqlx::query("INSERT INTO tenants (...) VALUES (?1, ?2)")
    .bind(now).bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();

// 改为 Any 兼容写法（Postgres 用 RETURNING，SQLite 用 last_insert_rowid）
// 方案 A：分方言执行
let id = match pool.dialect {
    DbDialect::Postgres => {
        sqlx::query_scalar::<_, i64>(
            "INSERT INTO tenants (...) VALUES ($1, $2) RETURNING id"
        ).bind(now).bind(now).fetch_one(&pool.pool).await?
    }
    DbDialect::Sqlite => {
        let res = sqlx::query("INSERT INTO tenants (...) VALUES (?, ?)")
            .bind(now).bind(now).execute(&pool.pool).await?;
        res.last_insert_rowid()
    }
};
```

> 建议抽出辅助宏 `insert_returning_id!` 减少重复。

#### 步骤 5：JSONB / TIMESTAMPTZ 类型映射

Postgres 方言下，`json` 字段对应 `serde_json::Value`，`timestamptz` 对应 `chrono::DateTime<Utc>`。在 `models/` 中确保类型标注正确，sqlx 的 `Any` 驱动会按目标方言映射。

### 验收标准

1. `USE_SQLITE=false` + `DATABASE_URL=postgres://...` 下 `cargo test --all-features` 全绿
2. Postgres 集成测试覆盖全部 B1-B6 域 CRUD
3. SQLite 内存库测试仍然通过（双驱动不回归）
4. 生产配置下服务正常启动并处理请求

---

## P0-2：分布式限流 + 调度选主

### P0-2a：分布式限流（Redis 滑动窗口）

#### 现状
[rate_limit.rs](file:///workspace/metaclouds-backend-rust/src/middleware/rate_limit.rs) 使用 `HashMap<String, Vec<Instant>>` + `Mutex`，进程内状态。

#### 改造方案

用 Redis + Lua 脚本实现原子滑动窗口计数。

#### 受影响文件
```
src/middleware/rate_limit.rs    # 重写为 Redis 实现
src/cache/redis.rs              # 暴露 Redis 客户端给限流器
```

#### 关键代码

新增 `src/middleware/rate_limit_redis.rs`：

```rust
use redis::aio::ConnectionManager;
use redis::Script;

// 滑动窗口 Lua 脚本：原子地裁剪窗口外元素 + 计数 + 追加当前时间戳
const SLIDING_WINDOW_SCRIPT: &str = r#"
local key = KEYS[1]
local window = tonumber(ARGV[1])
local limit = tonumber(ARGV[2])
local now = tonumber(ARGV[3])
local cutoff = now - window
redis.call('ZREMRANGEBYSCORE', key, 0, cutoff)
local count = redis.call('ZCARD', key)
if count >= limit then
    return count
end
redis.call('ZADD', key, now, now .. '-' .. math.random())
redis.call('EXPIRE', key, window)
return count + 1
"#;

pub struct DistributedRateLimiter {
    conn: ConnectionManager,
    max_requests: u32,
    window_secs: u64,
}

impl DistributedRateLimiter {
    pub async fn check(&self, key: &str, now_ms: u128) -> Result<u32, redis::RedisError> {
        let count: u32 = Script::new(SLIDING_WINDOW_SCRIPT)
            .key(format!("metaclouds:ratelimit:{}", key))
            .arg(self.window_secs * 1000)
            .arg(self.max_requests)
            .arg(now_ms as u64)
            .invoke_async(&mut self.conn.clone())
            .await?;
        Ok(count)
    }
}
```

中间件改造：

```rust
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    if !enabled() { return next.run(request).await; }
    let path = request.uri().path().to_string();
    if is_whitelisted(&path) { return next.run(request).await; }

    let ip = client_ip(&request);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_millis();

    // 从 AppState 取限流器
    let state = request.extensions().get::<AppState>().cloned();
    // ... 调用 DistributedRateLimiter::check
    // 超过 max_requests 返回 429
}
```

#### 降级策略
Redis 不可用时回退到原内存限流器（单 Pod 生效，记录 warn 日志），保证可用性优先。

### P0-2b：调度器 Leader Election

#### 现状
`scheduler/mod.rs` 使用 `tokio-cron-scheduler`，3 副本各自执行 cron，导致任务重复。

#### 改造方案
基于 Redis `SET key value NX EX ttl` 实现租约选主。

#### 受影响文件
```
src/scheduler/mod.rs    # 启动前抢锁，抢到才注册 cron
src/cache/redis.rs      # 提供 set_nx 原语
```

#### 关键代码

在 `src/scheduler/mod.rs` 启动逻辑中加入选主：

```rust
const LEADER_KEY: &str = "metaclouds:scheduler:leader";
const LEASE_SECS: u64 = 30;

/// 尝试获取调度器领导权；返回是否成功。
pub async fn try_acquire_leadership(redis: &ConnectionManager, instance_id: &str) -> bool {
    let mut conn = redis.clone();
    let result: Result<String, _> = redis::cmd("SET")
        .arg(LEADER_KEY).arg(instance_id)
        .arg("NX").arg("EX").arg(LEASE_SECS)
        .query_async(&mut conn).await;
    matches!(result, Ok(s) if s == "OK")
}

/// 续租：若当前持有者是自己则刷新 TTL。
pub async fn renew_lease(redis: &ConnectionManager, instance_id: &str) -> bool {
    let mut conn = redis.clone();
    // Lua: 仅当 value 匹配时续期
    let script = "if redis.call('GET', KEYS[1]) == ARGV[1] then \
                  return redis.call('EXPIRE', KEYS[1], ARGV[2]) else return 0 end";
    let ok: i32 = redis::Script::new(script)
        .key(LEADER_KEY).arg(instance_id).arg(LEASE_SECS)
        .invoke_async(&mut conn).await.unwrap_or(0);
    ok == 1
}
```

调度器主循环：

```rust
pub async fn run_scheduler(state: AppState) {
    let instance_id = uuid::Uuid::new_v4().to_string();
    let redis = state.redis.clone(); // 需在 AppState 增加 redis 字段

    loop {
        if try_acquire_leadership(&redis, &instance_id).await {
            // 成为 leader：启动 cron 任务
            let mut sched = Scheduler::new();
            sched.insert(Schedule::from_str("0 */1 * * * *").unwrap(), job_sync_resources);
            sched.start().await;

            // 续租循环
            loop {
                tokio::time::sleep(Duration::from_secs(LEASE_SECS / 2)).await;
                if !renew_lease(&redis, &instance_id).await {
                    break; // 丢失领导权，退出
                }
            }
        } else {
            // 非 leader：等待后重试
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }
}
```

需在 `main.rs` 中 `tokio::spawn(run_scheduler(state))`。

#### 验收标准
- 3 副本同时启动，仅 1 个执行 cron 任务
- Leader 宕机后 30 秒内其他副本接管
- Redis 不可用时降级（所有副本不执行，避免重复；或允许单实例模式）

---

## P1-1：异步消息总线

### 现状
作业提交（`handlers/job.rs` 的 `submit_job_to_k8s`）同步调用 K8s API，高并发下易阻塞、失败无重试。

### 改造方案
引入 **NATS**（轻量、 Rust 生态好，`async-nats` crate）作为作业提交总线。Redis Streams 可作为备选（零新增依赖）。

### 受影响文件
```
Cargo.toml                    # 新增 async-nats
src/config.rs                 # 新增 NATS 配置
src/messagebus/mod.rs         # 新增：生产者/消费者抽象
src/services/job.rs           # submit 改为发布消息
src/worker/job_worker.rs      # 新增：消费消息并提交 K8s
main.rs                       # 启动 worker
```

### 关键代码

`src/messagebus/mod.rs`：

```rust
use async_nats::{Client, ConnectOptions};

pub struct MessageBus {
    client: Client,
}

impl MessageBus {
    pub async fn connect(addr: &str) -> AppResult<Self> {
        let client = ConnectOptions::new()
            .connect(addr)
            .await
            .map_err(|e| AppError::with_source(
                ErrorCode::InternalServerError, "failed to connect NATS", e))?;
        Ok(Self { client })
    }

    pub async fn publish(&self, subject: &str, payload: &[u8]) -> AppResult<()> {
        self.client.publish(subject.to_string(), payload.into())
            .await
            .map_err(|e| AppError::with_source(
                ErrorCode::InternalServerError, "publish failed", e))?;
        Ok(())
    }
}
```

`src/services/job.rs` 提交改造：

```rust
pub async fn submit_job(state: &AppState, job_id: i64) -> AppResult<()> {
    let payload = serde_json::json!({ "job_id": job_id, "action": "submit" });
    state.message_bus
        .publish("jobs.submit", &serde_json::to_vec(&payload)?)
        .await?;
    // 更新 job 状态为 queued
    Ok(())
}
```

`src/worker/job_worker.rs`：

```rust
pub async fn run_job_worker(state: AppState) {
    let mut sub = state.message_bus.client
        .subscribe("jobs.submit".to_string()).await.unwrap();

    while let Some(msg) = sub.next().await {
        let payload: serde_json::Value = serde_json::from_slice(&msg.payload).unwrap();
        let job_id = payload["job_id"].as_i64().unwrap();

        // 重试 3 次，失败入死信队列
        for attempt in 1..=3 {
            match submit_to_k8s(&state, job_id).await {
                Ok(_) => break,
                Err(e) => {
                    if attempt == 3 {
                        state.message_bus.publish(
                            "jobs.dlq", &msg.payload).await.ok();
                    }
                    tokio::time::sleep(Duration::from_secs(attempt * 2)).await;
                }
            }
        }
    }
}
```

### 验收标准
- 作业提交接口 P99 < 100ms（仅发布消息）
- K8s API 不可用时消息不丢失，进入 DLQ
- 消费者水平扩展（NATS queue group）

---

## P1-2：审计日志中间件

### 改造方案
新增 axum 中间件，记录所有写操作（POST/PUT/DELETE）的审计信息到 `audit_logs` 表。

### 受影响文件
```
migrations/postgres/010_audit_logs.sql   # 新增表
migrations/010_audit_logs.sql
src/middleware/audit.rs                  # 新增
src/middleware/mod.rs                    # 注册
src/routes.rs                            # 挂载到受保护路由
```

### 关键代码

迁移脚本 `migrations/postgres/010_audit_logs.sql`：

```sql
CREATE TABLE IF NOT EXISTS audit_logs (
    id BIGSERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    user_id BIGINT,
    tenant_id BIGINT,
    username VARCHAR(255),
    action VARCHAR(32) NOT NULL,          -- POST/PUT/DELETE
    resource_type VARCHAR(64) NOT NULL,   -- tenants/clusters/jobs...
    resource_id VARCHAR(64),
    path TEXT NOT NULL,
    method VARCHAR(16) NOT NULL,
    status_code INT NOT NULL,
    client_ip VARCHAR(64),
    request_body JSONB,                   -- 脱敏后
    user_agent TEXT
);
CREATE INDEX idx_audit_user ON audit_logs(user_id);
CREATE INDEX idx_audit_time ON audit_logs(created_at);
```

中间件 `src/middleware/audit.rs`：

```rust
use axum::{extract::Request, middleware::Next, response::Response};
use crate::auth::jwt::Claims;
use crate::db::DatabasePool;

pub async fn audit_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 仅审计写操作
    if !matches!(method.as_str(), "POST" | "PUT" | "DELETE") {
        return next.run(request).await;
    }

    let claims = request.extensions().get::<Claims>().cloned();
    let client_ip = request.headers()
        .get("x-forwarded-for").and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").to_string());

    let response = next.run(request).await;
    let status = response.status().as_u16();

    if let Some(state) = response.extensions().get::<AppState>() {
        if let Some(claims) = claims {
            let resource_type = path.split('/').nth(3).unwrap_or("unknown");
            sqlx::query(
                "INSERT INTO audit_logs \
                 (user_id, tenant_id, username, action, resource_type, path, method, status_code, client_ip) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(claims.sub.parse::<i64>().ok())
            .bind(claims.tenant_id)
            .bind(&claims.username)
            .bind(method.as_str())
            .bind(resource_type)
            .bind(&path)
            .bind(method.as_str())
            .bind(status as i32)
            .bind(client_ip)
            .execute(&state.pool.pool)
            .await
            .ok(); // 审计失败不影响业务响应
        }
    }
    response
}
```

挂载：在 `routes.rs` 的 `protected` 路由层增加 `.route_layer(from_fn(audit_middleware))`。

### 验收标准
- 所有写操作产生审计记录
- 审计写入失败不影响主流程
- 可按用户/时间/资源查询审计日志

---

## P1-3：默认管理员强制改密

### 现状
`db.rs` 第 279 行播种 `Admin@123456`，无首次登录改密机制。

### 改造方案
在 `users` 表新增 `must_change_password BOOLEAN DEFAULT false`，默认管理员设为 `true`；登录时若该字段为 true，返回专用错误码 `MUST_CHANGE_PASSWORD`，前端跳转改密页。

### 受影响文件
```
migrations/postgres/011_force_password_change.sql
migrations/011_force_password_change.sql
src/models/user.rs          # 新增字段
src/auth/handler.rs         # login 返回 MUST_CHANGE_PASSWORD
src/auth/password.rs        # change_password 清除标记
src/error.rs                # 新增错误码
metaclouds-frontend-vue/src/pages/Login.vue
metaclouds-frontend-vue/src/api/index.ts
```

### 关键代码

`src/error.rs` 新增：

```rust
pub enum ErrorCode {
    // ... 现有
    MustChangePassword,  // 新增
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            // ...
            ErrorCode::MustChangePassword => "MUST_CHANGE_PASSWORD",
        }
    }
    pub fn http_status(self) -> StatusCode {
        match self {
            // 403 Forbidden 但带专用 code，前端识别后跳改密页
            ErrorCode::MustChangePassword => StatusCode::FORBIDDEN,
        }
    }
}
```

`src/auth/handler.rs` 的 `login`：

```rust
pub async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> AppResult<Json<ApiResponse<LoginResponse>>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.pool.pool)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid credentials"))?;

    if !verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::unauthorized("invalid credentials"));
    }

    if user.must_change_password {
        return Err(AppError::new(
            ErrorCode::MustChangePassword,
            "Please change your password before continuing",
        ));
    }
    // ... 正常发 token
}
```

播种时设置标记 `db.rs`：

```rust
// 原
"INSERT INTO users (..., must_change_password) VALUES (?, ?, 'admin', ?, 'admin', ?, true)"
```

`change_password` 成功后清除标记：

```rust
sqlx::query("UPDATE users SET password_hash = ?, must_change_password = false WHERE id = ?")
    .bind(new_hash).bind(user_id).execute(&pool).await?;
```

### 验收标准
- 默认 admin 首次登录返回 `MUST_CHANGE_PASSWORD`
- 改密成功后可正常登录
- 非默认用户不受影响

---

## P1-4：登录防爆破

### 改造方案
对 `/auth/login` 单独配置更严格的限流（5 次/分钟/IP），并引入账户锁定：连续失败 5 次锁定账户 15 分钟。

### 受影响文件
```
src/auth/handler.rs         # 登录逻辑增加失败计数与锁定
src/cache/redis.rs          # 提供 incr + expire 原语
migrations/012_account_lockout.sql  # users 增加 locked_until
```

### 关键代码

`src/auth/handler.rs` 登录逻辑：

```rust
const MAX_FAILED_ATTEMPTS: i64 = 5;
const LOCKOUT_MINUTES: u64 = 15;

pub async fn login(...) -> AppResult<...> {
    // 1. IP 级限流：复用分布式限流器，key = login:ip，阈值 5/min
    // 2. 账户级锁定检查
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE username = ? AND (locked_until IS NULL OR locked_until < NOW())"
    ).bind(&req.username).fetch_optional(&state.pool.pool).await?;

    let Some(user) = user else {
        return Err(AppError::unauthorized("invalid credentials"));
    };

    if !verify_password(&req.password, &user.password_hash)? {
        // 失败计数（Redis INCR + EXPIRE）
        let fail_key = format!("metaclouds:loginfail:{}", user.id);
        let fails: i64 = redis_cmd_incr(&state.redis, &fail_key, 300).await?;

        if fails >= MAX_FAILED_ATTEMPTS {
            let until = chrono::Utc::now() + chrono::Duration::minutes(LOCKOUT_MINUTES as i64);
            sqlx::query("UPDATE users SET locked_until = ? WHERE id = ?")
                .bind(until).bind(user.id).execute(&state.pool.pool).await?;
        }
        return Err(AppError::unauthorized("invalid credentials"));
    }

    // 登录成功，清除失败计数
    redis_cmd_del(&state.redis, &format!("metaclouds:loginfail:{}", user.id)).await.ok();
    // ... 发 token
}
```

### 验收标准
- 同一 IP 1 分钟内超过 5 次登录返回 429
- 同一账户连续 5 次失败后锁定 15 分钟
- 锁定期间返回明确错误（不泄露是否存在账户）

---

## P1-5：前端质量门禁硬化

### 改造方案
1. 接入 ESLint + Prettier
2. CI 中 vitest 改为阻塞
3. 增加最小覆盖率门槛

### 受影响文件
```
metaclouds-frontend-vue/package.json          # 新增脚本
metaclouds-frontend-vue/.eslintrc.cjs         # 新增
metaclouds-frontend-vue/.prettierrc           # 新增
.github/workflows/ci-cd.yml                   # 测试改为阻塞
```

### 关键代码

`package.json` 新增依赖与脚本：

```json
{
  "scripts": {
    "lint": "eslint . --ext .vue,.ts,.tsx",
    "lint:fix": "eslint . --ext .vue,.ts,.tsx --fix",
    "format": "prettier --write \"src/**/*.{vue,ts,css}\"",
    "test": "vitest run",
    "test:coverage": "vitest run --coverage"
  },
  "devDependencies": {
    "@typescript-eslint/eslint-plugin": "^8.0.0",
    "@typescript-eslint/parser": "^8.0.0",
    "eslint": "^9.0.0",
    "eslint-plugin-vue": "^9.0.0",
    "prettier": "^3.0.0"
  }
}
```

`.eslintrc.cjs`：

```js
module.exports = {
  root: true,
  extends: [
    'eslint:recommended',
    'plugin:vue/vue3-recommended',
    'plugin:@typescript-eslint/recommended',
  ],
  parser: 'vue-eslint-parser',
  parserOptions: { parser: '@typescript-eslint/parser' },
  rules: {
    'vue/multi-word-component-names': 'off',
    '@typescript-eslint/no-explicit-any': 'warn',
  },
};
```

CI 修改 `.github/workflows/ci-cd.yml`：

```yaml
# 原（不阻塞）
- name: Run frontend tests
  run: npm run test -- --passWithNoTests
  continue-on-error: true

# 改为（阻塞 + 覆盖率门槛）
- name: Run frontend tests
  run: npm run test:coverage -- --coverage.thresholds.statements=70

- name: Lint frontend
  run: npm run lint
```

### 验收标准
- ESLint 无 error 才能合并
- vitest 失败阻断 PR
- 语句覆盖率 ≥ 70%

---

## 实施顺序建议

```
Phase 1 (阻断生产，1-2 周)
  └── P0-1 Postgres 移植
  └── P0-2 分布式限流 + 调度选主

Phase 2 (韧性与安全，1 周)
  └── P1-3 强制改密 + P1-4 防爆破（安全入口）
  └── P1-2 审计日志

Phase 3 (扩展与质量，1 周)
  └── P1-1 消息总线
  └── P1-5 前端门禁
```

## 风险与回滚

| 风险 | 缓解措施 |
|---|---|
| P0-1 AnyPool 性能/兼容性问题 | 保持 SQLite 测试通道，灰度切 Postgres，保留回退开关 |
| P0-2 Redis 单点 | Redis 主从 + Sentinels；降级到内存限流（单 Pod 生效） |
| P1-1 NATS 新增依赖复杂度 | 可用 Redis Streams 替代，零新增中间件 |
| P1-3 强制改密阻断现有用户 | 仅对 `must_change_password=true` 的账户生效，存量用户不受影响 |

---

## 附录：需新增的依赖

```toml
# Cargo.toml
async-nats = "0.35"              # P1-1 消息总线（或用 redis streams 替代）
uuid = { version = "1", features = ["v4"] }  # 已有，调度选主 instance_id

# 前端 package.json
eslint, @typescript-eslint/*, eslint-plugin-vue, prettier  # P1-5
```
