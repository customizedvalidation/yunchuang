-- Phase 1 扩展迁移：tenants / clusters / resources，并为 users 增加软删除列。
-- PostgreSQL variant，语义等价于 ../002_tenants_clusters_resources.sql。
--
-- 方言映射：
--   id INTEGER PRIMARY KEY AUTOINCREMENT -> BIGSERIAL PRIMARY KEY
--   时间戳 TEXT                         -> TIMESTAMPTZ（软删除列可空、无默认）
--   JSON 列（gpu_vendors / scheduler_types）TEXT DEFAULT '[]' -> JSONB DEFAULT '[]'::jsonb
--   布尔标志 multi_cluster_enabled       INTEGER 0/1           -> BOOLEAN DEFAULT FALSE
--   布尔标志 resources.mig_enabled       INTEGER 0/1           -> BOOLEAN DEFAULT FALSE
--   浮点 utilization / vram_oversubscription_ratio REAL        -> DOUBLE PRECISION

-- 1) users 表扩展：软删除列（001_initial.sql 未包含该列）
ALTER TABLE users ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;
CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users (deleted_at);

-- 2) tenants 租户表
CREATE TABLE IF NOT EXISTS tenants (
    id            BIGSERIAL    PRIMARY KEY,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ  NULL,
    name          TEXT         NOT NULL UNIQUE,
    description   TEXT         NOT NULL DEFAULT '',
    status        TEXT         NOT NULL DEFAULT 'active',
    gpu_quota     BIGINT       NOT NULL DEFAULT 0,
    cpu_quota     BIGINT       NOT NULL DEFAULT 0,
    memory_quota  BIGINT       NOT NULL DEFAULT 0,
    storage_quota BIGINT       NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_tenants_deleted_at ON tenants (deleted_at);

-- 3) clusters 集群表（对齐 Go Cluster 模型全部字段）
CREATE TABLE IF NOT EXISTS clusters (
    id                    BIGSERIAL    PRIMARY KEY,
    created_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at            TIMESTAMPTZ  NULL,
    name                  TEXT         NOT NULL UNIQUE,
    description           TEXT         NOT NULL DEFAULT '',
    status                TEXT         NOT NULL DEFAULT 'active',
    nodes                 BIGINT       NOT NULL DEFAULT 0,
    gpus                  BIGINT       NOT NULL DEFAULT 0,
    cpus                  BIGINT       NOT NULL DEFAULT 0,
    memory                BIGINT       NOT NULL DEFAULT 0,
    storage               BIGINT       NOT NULL DEFAULT 0,
    network_type          TEXT         NOT NULL DEFAULT '',
    location              TEXT         NOT NULL DEFAULT '',
    -- JSON 数组：Rust 侧 orm::Json<Vec<String>>，Postgres 用 JSONB
    gpu_vendors           JSONB        NOT NULL DEFAULT '[]'::jsonb,
    scheduler_types       JSONB        NOT NULL DEFAULT '[]'::jsonb,
    multi_cluster_enabled BOOLEAN      NOT NULL DEFAULT FALSE,
    federation_id         TEXT         NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_clusters_deleted_at ON clusters (deleted_at);

-- 4) resources 资源表（对齐 Go Resource 模型全部字段）
CREATE TABLE IF NOT EXISTS resources (
    id          BIGSERIAL    PRIMARY KEY,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ  NULL,
    cluster_id  BIGINT       NOT NULL DEFAULT 0,
    type        TEXT         NOT NULL DEFAULT '',
    name        TEXT         NOT NULL DEFAULT '',
    status      TEXT         NOT NULL DEFAULT 'available',
    total       BIGINT       NOT NULL DEFAULT 0,
    used        BIGINT       NOT NULL DEFAULT 0,
    available   BIGINT       NOT NULL DEFAULT 0,
    utilization DOUBLE PRECISION NOT NULL DEFAULT 0,
    details     TEXT         NOT NULL DEFAULT '',
    vendor      TEXT         NOT NULL DEFAULT '',
    gpu_model   TEXT         NOT NULL DEFAULT '',
    vram_total_mb                BIGINT NOT NULL DEFAULT 0,
    vram_used_mb                 BIGINT NOT NULL DEFAULT 0,
    vram_oversubscription_ratio  DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    mig_enabled                  BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_resources_deleted_at ON resources (deleted_at);
CREATE INDEX IF NOT EXISTS idx_resources_cluster_id ON resources (cluster_id);
CREATE INDEX IF NOT EXISTS idx_resources_type      ON resources (type);
CREATE INDEX IF NOT EXISTS idx_resources_status    ON resources (status);
