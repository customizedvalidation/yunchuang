-- Phase 1 扩展迁移：tenants / clusters / resources，并为 users 增加软删除列。
--
-- 语义对齐 Go 版 models/*.go 的 GORM AutoMigrate 结构：
--   - tenants:  name uniqueIndex / status default 'active' / 四项配额
--   - clusters: name uniqueIndex / status default 'active' / 节点/GPU 规格 / 多厂商字段
--   - resources: cluster_id 关联 / type / status default 'available' / 计数与显存字段
--   - users:    增加 deleted_at（gorm.DeletedAt，软删除 + index）
--
-- 方言说明：本文件按 SQLite 方言编写（Phase 1 仅在 SQLite 上实际执行）。
--   - SQLite 自增主键使用 INTEGER PRIMARY KEY AUTOINCREMENT；
--   - JSON / UUID 类字段统一以 TEXT 存储（Go 侧 gpu_vendors / scheduler_types 本就是 JSON 字符串）；
--   - 软删除时间戳统一为 TEXT（ISO8601，由 sqlx 的 chrono 编解码层负责读写）。
-- 后续接入 Postgres 时，应在 migrations/postgres/ 下提供对应变体（SERIAL/BIGSERIAL、JSONB、TIMESTAMPTZ），
-- 并在 db.rs 的 run_migrations 中按驱动选择迁移目录。

-- 1) users 表扩展：软删除列（Phase 0 的 001_initial.sql 未包含该列）
ALTER TABLE users ADD COLUMN deleted_at TEXT NULL;
CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users (deleted_at);

-- 2) tenants 租户表
CREATE TABLE IF NOT EXISTS tenants (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL,
    deleted_at    TEXT    NULL,
    name          TEXT    NOT NULL UNIQUE,
    description   TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'active',
    gpu_quota     INTEGER NOT NULL DEFAULT 0,
    cpu_quota     INTEGER NOT NULL DEFAULT 0,
    memory_quota  INTEGER NOT NULL DEFAULT 0,
    storage_quota INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_tenants_deleted_at ON tenants (deleted_at);

-- 3) clusters 集群表（对齐 Go Cluster 模型全部字段）
CREATE TABLE IF NOT EXISTS clusters (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at           TEXT    NOT NULL,
    updated_at           TEXT    NOT NULL,
    deleted_at           TEXT    NULL,
    name                 TEXT    NOT NULL UNIQUE,
    description          TEXT    NOT NULL DEFAULT '',
    status               TEXT    NOT NULL DEFAULT 'active',
    nodes                INTEGER NOT NULL DEFAULT 0,
    gpus                 INTEGER NOT NULL DEFAULT 0,
    cpus                 INTEGER NOT NULL DEFAULT 0,
    memory               INTEGER NOT NULL DEFAULT 0,
    storage              INTEGER NOT NULL DEFAULT 0,
    network_type         TEXT    NOT NULL DEFAULT '',
    location             TEXT    NOT NULL DEFAULT '',
    -- Go 侧为 JSON 字符串数组，SQLite 以 TEXT 存放 JSON，由 orm::Json<T> 负责编解码
    gpu_vendors          TEXT    NOT NULL DEFAULT '[]',
    scheduler_types      TEXT    NOT NULL DEFAULT '[]',
    multi_cluster_enabled INTEGER NOT NULL DEFAULT 0,
    federation_id        TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_clusters_deleted_at ON clusters (deleted_at);

-- 4) resources 资源表（对齐 Go Resource 模型全部字段）
CREATE TABLE IF NOT EXISTS resources (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    deleted_at  TEXT    NULL,
    cluster_id  INTEGER NOT NULL DEFAULT 0,
    type        TEXT    NOT NULL DEFAULT '',
    name        TEXT    NOT NULL DEFAULT '',
    status      TEXT    NOT NULL DEFAULT 'available',
    total       INTEGER NOT NULL DEFAULT 0,
    used        INTEGER NOT NULL DEFAULT 0,
    available   INTEGER NOT NULL DEFAULT 0,
    utilization REAL    NOT NULL DEFAULT 0,
    details     TEXT    NOT NULL DEFAULT '',
    vendor      TEXT    NOT NULL DEFAULT '',
    gpu_model   TEXT    NOT NULL DEFAULT '',
    vram_total_mb                INTEGER NOT NULL DEFAULT 0,
    vram_used_mb                 INTEGER NOT NULL DEFAULT 0,
    vram_oversubscription_ratio  REAL    NOT NULL DEFAULT 1.0,
    mig_enabled                  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_resources_deleted_at ON resources (deleted_at);
CREATE INDEX IF NOT EXISTS idx_resources_cluster_id ON resources (cluster_id);
CREATE INDEX IF NOT EXISTS idx_resources_type      ON resources (type);
CREATE INDEX IF NOT EXISTS idx_resources_status    ON resources (status);
