-- WP-P2-B4 调度域迁移：partitions / partition_permissions / resource_quotas /
-- scheduler_integrations。
-- PostgreSQL variant，语义等价于 ../007_b4_scheduler.sql。
--
-- 方言映射：
--   id                                  -> BIGSERIAL PRIMARY KEY
--   时间戳（granted_at/expires_at/last_heartbeat/软删除）-> TIMESTAMPTZ
--   JSON 对象（node_selector / labels / credentials / config，Rust orm::Json<Value>）-> JSONB
--   浮点（cpu_cores / memory_gb / cpu_* / memory_* / storage_*）-> DOUBLE PRECISION

-- 1) partitions 分区表
CREATE TABLE IF NOT EXISTS partitions (
    id              BIGSERIAL    PRIMARY KEY,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ  NULL,
    cluster_id      BIGINT       NOT NULL DEFAULT 0,
    name            TEXT         NOT NULL DEFAULT '',
    description     TEXT         NOT NULL DEFAULT '',
    partition_type  TEXT         NOT NULL DEFAULT 'shared',   -- exclusive | shared | reserved
    gpu_count       BIGINT       NOT NULL DEFAULT 0,
    cpu_cores       DOUBLE PRECISION NOT NULL DEFAULT 0,
    memory_gb       DOUBLE PRECISION NOT NULL DEFAULT 0,
    status          TEXT         NOT NULL DEFAULT 'active',   -- active | inactive | maintenance
    node_selector   JSONB        NOT NULL DEFAULT '{}'::jsonb,
    labels          JSONB        NOT NULL DEFAULT '{}'::jsonb,
    tenant_id       BIGINT       NULL
);

CREATE INDEX IF NOT EXISTS idx_partitions_deleted_at ON partitions (deleted_at);
CREATE INDEX IF NOT EXISTS idx_partitions_cluster_status ON partitions (cluster_id, status);
CREATE INDEX IF NOT EXISTS idx_partitions_type ON partitions (partition_type);

-- 2) partition_permissions 分区授权表（无软删除列）
CREATE TABLE IF NOT EXISTS partition_permissions (
    id              BIGSERIAL    PRIMARY KEY,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    partition_id    BIGINT       NOT NULL DEFAULT 0,
    user_id         BIGINT       NOT NULL DEFAULT 0,
    tenant_id       BIGINT       NOT NULL DEFAULT 0,
    permission_type TEXT         NOT NULL DEFAULT 'read',      -- read | write | admin
    granted_by      BIGINT       NOT NULL DEFAULT 0,           -- -> users.id
    granted_at      TIMESTAMPTZ  NOT NULL,
    expires_at      TIMESTAMPTZ  NULL
);

CREATE INDEX IF NOT EXISTS idx_part_perms_partition_user ON partition_permissions (partition_id, user_id);
CREATE INDEX IF NOT EXISTS idx_part_perms_user ON partition_permissions (user_id);

-- 3) resource_quotas 多维度资源配额表
CREATE TABLE IF NOT EXISTS resource_quotas (
    id                BIGSERIAL    PRIMARY KEY,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ  NULL,
    name              TEXT         NOT NULL DEFAULT '',
    description       TEXT         NOT NULL DEFAULT '',
    tenant_id         BIGINT       NOT NULL DEFAULT 0,
    partition_id      BIGINT       NULL,
    gpu_limit         BIGINT       NOT NULL DEFAULT 0,
    gpu_used          BIGINT       NOT NULL DEFAULT 0,
    cpu_limit         DOUBLE PRECISION NOT NULL DEFAULT 0,
    cpu_used          DOUBLE PRECISION NOT NULL DEFAULT 0,
    memory_limit_gb   DOUBLE PRECISION NOT NULL DEFAULT 0,
    memory_used_gb    DOUBLE PRECISION NOT NULL DEFAULT 0,
    storage_limit_gb  DOUBLE PRECISION NOT NULL DEFAULT 0,
    storage_used_gb   DOUBLE PRECISION NOT NULL DEFAULT 0,
    status            TEXT         NOT NULL DEFAULT 'active'    -- active | suspended
);

CREATE INDEX IF NOT EXISTS idx_quotas_deleted_at ON resource_quotas (deleted_at);
CREATE INDEX IF NOT EXISTS idx_quotas_tenant_partition ON resource_quotas (tenant_id, partition_id);
CREATE INDEX IF NOT EXISTS idx_quotas_status ON resource_quotas (status);

-- 4) scheduler_integrations 外部调度器集成表
CREATE TABLE IF NOT EXISTS scheduler_integrations (
    id                BIGSERIAL    PRIMARY KEY,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ  NULL,
    name              TEXT         NOT NULL DEFAULT '',
    description       TEXT         NOT NULL DEFAULT '',
    scheduler_type    TEXT         NOT NULL DEFAULT 'kubernetes', -- kubernetes | yarn | slurm | custom
    endpoint          TEXT         NOT NULL DEFAULT '',
    auth_type         TEXT         NOT NULL DEFAULT 'none',       -- none | bearer | tls | mtls
    credentials       JSONB        NOT NULL DEFAULT '{}'::jsonb,
    status            TEXT         NOT NULL DEFAULT 'disconnected', -- connected | disconnected | error
    cluster_id        BIGINT       NOT NULL DEFAULT 0,
    version           TEXT         NOT NULL DEFAULT '',
    last_heartbeat    TIMESTAMPTZ  NULL,
    config            JSONB        NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_sched_deleted_at ON scheduler_integrations (deleted_at);
CREATE INDEX IF NOT EXISTS idx_sched_cluster_status ON scheduler_integrations (cluster_id, status);
CREATE INDEX IF NOT EXISTS idx_sched_type ON scheduler_integrations (scheduler_type);
