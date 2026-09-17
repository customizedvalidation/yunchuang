-- WP-P2-B5：数据加速域（Dataset / FluidCache / DistributedTrainingConfig /
-- InferenceConfig / AccelerationSuite / Checkpoint）。
--
-- 方言：SQLite。JSON 字段统一以 TEXT 存储，由 orm::Json<T> 负责编解码。
-- 所有表含 id / created_at / updated_at / deleted_at 软删除三件套 + tenant_id FK。

-- 1) datasets 数据集表
CREATE TABLE IF NOT EXISTS datasets (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    deleted_at  TEXT    NULL,
    name        TEXT    NOT NULL UNIQUE,
    description TEXT    NOT NULL DEFAULT '',
    type        TEXT    NOT NULL DEFAULT 'private',
    source_path TEXT    NOT NULL DEFAULT '',
    format      TEXT    NOT NULL DEFAULT '',
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    tenant_id   INTEGER NOT NULL DEFAULT 1,
    created_by  INTEGER NOT NULL DEFAULT 0,
    status      TEXT    NOT NULL DEFAULT 'active',
    labels      TEXT    NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_datasets_deleted_at ON datasets (deleted_at);
CREATE INDEX IF NOT EXISTS idx_datasets_tenant_type ON datasets (tenant_id, type);

-- 2) fluid_caches Fluid 分布式缓存表
CREATE TABLE IF NOT EXISTS fluid_caches (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    deleted_at  TEXT    NULL,
    name        TEXT    NOT NULL DEFAULT '',
    dataset_id  INTEGER NOT NULL,
    namespace   TEXT    NOT NULL DEFAULT '',
    path        TEXT    NOT NULL DEFAULT '',
    cache_class TEXT    NOT NULL DEFAULT '',
    replicas    INTEGER NOT NULL DEFAULT 1,
    status      TEXT    NOT NULL DEFAULT 'inactive'
);

CREATE INDEX IF NOT EXISTS idx_fluid_caches_deleted_at ON fluid_caches (deleted_at);
CREATE INDEX IF NOT EXISTS idx_fluid_caches_dataset_id ON fluid_caches (dataset_id);

-- 3) distributed_training_configs 分布式训练配置表
CREATE TABLE IF NOT EXISTS distributed_training_configs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    deleted_at          TEXT    NULL,
    name                TEXT    NOT NULL DEFAULT '',
    description         TEXT    NOT NULL DEFAULT '',
    framework           TEXT    NOT NULL DEFAULT 'pytorch',
    worker_replicas     INTEGER NOT NULL DEFAULT 1,
    gpu_per_worker      INTEGER NOT NULL DEFAULT 0,
    cpu_per_worker      INTEGER NOT NULL DEFAULT 1,
    memory_per_worker_gb INTEGER NOT NULL DEFAULT 4,
    entrypoint          TEXT    NOT NULL DEFAULT '',
    env_vars            TEXT    NOT NULL DEFAULT '{}',
    tenant_id           INTEGER NOT NULL DEFAULT 1,
    created_by          INTEGER NOT NULL DEFAULT 0,
    status              TEXT    NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_dtc_deleted_at ON distributed_training_configs (deleted_at);
CREATE INDEX IF NOT EXISTS idx_dtc_tenant_id ON distributed_training_configs (tenant_id);

-- 4) inference_configs 推理配置表
CREATE TABLE IF NOT EXISTS inference_configs (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    deleted_at          TEXT    NULL,
    name                TEXT    NOT NULL DEFAULT '',
    description         TEXT    NOT NULL DEFAULT '',
    model_path          TEXT    NOT NULL DEFAULT '',
    runtime             TEXT    NOT NULL DEFAULT 'triton',
    replicas            INTEGER NOT NULL DEFAULT 1,
    gpu_per_replica     INTEGER NOT NULL DEFAULT 0,
    cpu_per_replica     INTEGER NOT NULL DEFAULT 1,
    memory_per_replica_gb INTEGER NOT NULL DEFAULT 4,
    port                INTEGER NOT NULL DEFAULT 8000,
    health_check_path   TEXT    NOT NULL DEFAULT '',
    tenant_id           INTEGER NOT NULL DEFAULT 1,
    created_by          INTEGER NOT NULL DEFAULT 0,
    status              TEXT    NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_inference_configs_deleted_at ON inference_configs (deleted_at);
CREATE INDEX IF NOT EXISTS idx_inference_configs_tenant_id ON inference_configs (tenant_id);

-- 5) acceleration_suites 加速套件表（组合对象）
CREATE TABLE IF NOT EXISTS acceleration_suites (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    deleted_at          TEXT    NULL,
    name                TEXT    NOT NULL UNIQUE,
    description         TEXT    NOT NULL DEFAULT '',
    suite_type          TEXT    NOT NULL DEFAULT 'training',
    dataset_id          INTEGER NULL,
    training_config_id  INTEGER NULL,
    inference_config_id INTEGER NULL,
    fluid_cache_id      INTEGER NULL,
    acceleration_config TEXT    NOT NULL DEFAULT '{}',
    tenant_id           INTEGER NOT NULL DEFAULT 1,
    created_by          INTEGER NOT NULL DEFAULT 0,
    status              TEXT    NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS idx_accel_suites_deleted_at ON acceleration_suites (deleted_at);
CREATE INDEX IF NOT EXISTS idx_accel_suites_tenant_status ON acceleration_suites (tenant_id, status);

-- 6) checkpoints 检查点表
CREATE TABLE IF NOT EXISTS checkpoints (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    deleted_at  TEXT    NULL,
    name        TEXT    NOT NULL DEFAULT '',
    description TEXT    NOT NULL DEFAULT '',
    job_id      INTEGER NULL,
    dataset_id  INTEGER NULL,
    path        TEXT    NOT NULL DEFAULT '',
    format      TEXT    NOT NULL DEFAULT 'pytorch',
    size_bytes  INTEGER NOT NULL DEFAULT 0,
    step        INTEGER NOT NULL DEFAULT 0,
    epoch       INTEGER NOT NULL DEFAULT 0,
    metrics     TEXT    NOT NULL DEFAULT '{}',
    tenant_id   INTEGER NOT NULL DEFAULT 1,
    created_by  INTEGER NOT NULL DEFAULT 0,
    status      TEXT    NOT NULL DEFAULT 'completed'
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_deleted_at ON checkpoints (deleted_at);
CREATE INDEX IF NOT EXISTS idx_checkpoints_tenant_job ON checkpoints (tenant_id, job_id);
