-- WP-P2-B4 调度域迁移：partitions / partition_permissions / resource_quotas / scheduler_integrations。
--
-- 字段对齐 WP-P2-B4 任务规格（Rust 版以 SQLite 落库）：
--   - partitions:             分区（cluster_id FK / partition_type / gpu_count·cpu_cores·memory_gb /
--                             status / node_selector·labels JSON TEXT / tenant_id 可空）。
--   - partition_permissions:  分区授权（partition_id/user_id/tenant_id FK / permission_type /
--                             granted_by→users / granted_at / expires_at 可空）。无软删除列。
--   - resource_quotas:        多维度配额（tenant_id FK / partition_id 可空 / gpu·cpu·memory·storage
--                             的 limit/used / status active|suspended）。
--   - scheduler_integrations: 外部调度器集成（cluster_id FK / scheduler_type / endpoint / auth_type /
--                             credentials·config JSON TEXT / status connected|disconnected|error /
--                             version / last_heartbeat 可空）。
--
-- 方言：SQLite（Phase 2 仅在 SQLite 实际执行）。
--   - JSON 列（node_selector / labels / credentials / config）以 TEXT 存放，Rust 侧用
--     crate::orm::Json<T> 读写，保持与 GORM type:json 等价。
--   - 软删除时间戳为 TEXT（ISO8601），由 sqlx chrono 编解码层读写。
--   - 占位符统一用全匿名 `?`（见 services 层），本文件只建表不写查询。

-- 1) partitions 分区表
CREATE TABLE IF NOT EXISTS partitions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL,
    deleted_at      TEXT    NULL,
    cluster_id      INTEGER NOT NULL DEFAULT 0,
    name            TEXT    NOT NULL DEFAULT '',
    description     TEXT    NOT NULL DEFAULT '',
    partition_type  TEXT    NOT NULL DEFAULT 'shared',   -- exclusive | shared | reserved
    gpu_count       INTEGER NOT NULL DEFAULT 0,
    cpu_cores       REAL    NOT NULL DEFAULT 0,
    memory_gb       REAL    NOT NULL DEFAULT 0,
    status          TEXT    NOT NULL DEFAULT 'active',   -- active | inactive | maintenance
    node_selector   TEXT    NOT NULL DEFAULT '{}',       -- JSON object
    labels          TEXT    NOT NULL DEFAULT '{}',       -- JSON object
    tenant_id       INTEGER NULL
);

CREATE INDEX IF NOT EXISTS idx_partitions_deleted_at ON partitions (deleted_at);
CREATE INDEX IF NOT EXISTS idx_partitions_cluster_status ON partitions (cluster_id, status);
CREATE INDEX IF NOT EXISTS idx_partitions_type ON partitions (partition_type);

-- 2) partition_permissions 分区授权表（无软删除列）
CREATE TABLE IF NOT EXISTS partition_permissions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL,
    partition_id    INTEGER NOT NULL DEFAULT 0,
    user_id         INTEGER NOT NULL DEFAULT 0,
    tenant_id       INTEGER NOT NULL DEFAULT 0,
    permission_type TEXT    NOT NULL DEFAULT 'read',      -- read | write | admin
    granted_by      INTEGER NOT NULL DEFAULT 0,          -- → users.id
    granted_at      TEXT    NOT NULL,
    expires_at      TEXT    NULL
);

CREATE INDEX IF NOT EXISTS idx_part_perms_partition_user ON partition_permissions (partition_id, user_id);
CREATE INDEX IF NOT EXISTS idx_part_perms_user ON partition_permissions (user_id);

-- 3) resource_quotas 多维度资源配额表
CREATE TABLE IF NOT EXISTS resource_quotas (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at        TEXT    NOT NULL,
    updated_at        TEXT    NOT NULL,
    deleted_at        TEXT    NULL,
    name              TEXT    NOT NULL DEFAULT '',
    description       TEXT    NOT NULL DEFAULT '',
    tenant_id         INTEGER NOT NULL DEFAULT 0,
    partition_id      INTEGER NULL,
    gpu_limit         INTEGER NOT NULL DEFAULT 0,
    gpu_used          INTEGER NOT NULL DEFAULT 0,
    cpu_limit         REAL    NOT NULL DEFAULT 0,
    cpu_used          REAL    NOT NULL DEFAULT 0,
    memory_limit_gb   REAL    NOT NULL DEFAULT 0,
    memory_used_gb    REAL    NOT NULL DEFAULT 0,
    storage_limit_gb  REAL    NOT NULL DEFAULT 0,
    storage_used_gb   REAL    NOT NULL DEFAULT 0,
    status            TEXT    NOT NULL DEFAULT 'active'    -- active | suspended
);

CREATE INDEX IF NOT EXISTS idx_quotas_deleted_at ON resource_quotas (deleted_at);
CREATE INDEX IF NOT EXISTS idx_quotas_tenant_partition ON resource_quotas (tenant_id, partition_id);
CREATE INDEX IF NOT EXISTS idx_quotas_status ON resource_quotas (status);

-- 4) scheduler_integrations 外部调度器集成表
CREATE TABLE IF NOT EXISTS scheduler_integrations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at        TEXT    NOT NULL,
    updated_at        TEXT    NOT NULL,
    deleted_at        TEXT    NULL,
    name              TEXT    NOT NULL DEFAULT '',
    description       TEXT    NOT NULL DEFAULT '',
    scheduler_type    TEXT    NOT NULL DEFAULT 'kubernetes', -- kubernetes | yarn | slurm | custom
    endpoint          TEXT    NOT NULL DEFAULT '',
    auth_type         TEXT    NOT NULL DEFAULT 'none',       -- none | bearer | tls | mtls
    credentials       TEXT    NOT NULL DEFAULT '{}',         -- JSON object
    status            TEXT    NOT NULL DEFAULT 'disconnected', -- connected | disconnected | error
    cluster_id        INTEGER NOT NULL DEFAULT 0,
    version           TEXT    NOT NULL DEFAULT '',
    last_heartbeat    TEXT    NULL,
    config            TEXT    NOT NULL DEFAULT '{}'          -- JSON object
);

CREATE INDEX IF NOT EXISTS idx_sched_deleted_at ON scheduler_integrations (deleted_at);
CREATE INDEX IF NOT EXISTS idx_sched_cluster_status ON scheduler_integrations (cluster_id, status);
CREATE INDEX IF NOT EXISTS idx_sched_type ON scheduler_integrations (scheduler_type);
