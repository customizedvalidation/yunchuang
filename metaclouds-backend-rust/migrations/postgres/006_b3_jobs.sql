-- WP-P2-B3 作业域迁移：jobs / gpu_devices / gpu_allocations。
-- PostgreSQL variant，语义等价于 ../006_b3_jobs.sql。
--
-- 方言映射：
--   id                        -> BIGSERIAL PRIMARY KEY
--   时间戳（start/end/软删除）-> TIMESTAMPTZ
--   布尔标志（elastic_enabled / checkpoint_enabled / topology_anti_affinity /
--             mig_enabled）   -> BOOLEAN（Rust 侧 bool）
--   浮点（gpu_fraction / utilization / fraction）-> DOUBLE PRECISION（Rust 侧 f64）
--   JSON 文本列（node_selector / affinity / tolerations / scaling_policy /
--                mig_profiles）-> TEXT（Rust 侧为 String 逐字承载 JSON 文本，
--                                     非 orm::Json<T>，故保持 TEXT 不用 JSONB）
--   `type` 是 SQL 非保留字，列名保留；Rust 模型用 kind + rename 映射。
--   `index` 用 gpu_index 承载（JSON 输出仍为 "index"）。

-- 1) jobs 作业表
CREATE TABLE IF NOT EXISTS jobs (
    id            BIGSERIAL    PRIMARY KEY,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ  NULL,
    cluster_id    BIGINT       NOT NULL DEFAULT 0,
    tenant_id     BIGINT       NOT NULL DEFAULT 0,
    user_id       BIGINT       NOT NULL DEFAULT 0,
    name          TEXT         NOT NULL DEFAULT '',
    description   TEXT         NOT NULL DEFAULT '',
    status        TEXT         NOT NULL DEFAULT 'pending',   -- pending | running | completed | failed | cancelled
    type          TEXT         NOT NULL DEFAULT '',           -- training | inference | batch
    priority      BIGINT       NOT NULL DEFAULT 0,            -- 0=low 1=medium 2=high 3=critical
    gpus          BIGINT       NOT NULL DEFAULT 0,
    cpus          BIGINT       NOT NULL DEFAULT 0,
    memory        BIGINT       NOT NULL DEFAULT 0,            -- GB
    duration      BIGINT       NOT NULL DEFAULT 0,            -- 预计运行分钟
    start_time    TIMESTAMPTZ  NULL,
    end_time      TIMESTAMPTZ  NULL,
    progress      BIGINT       NOT NULL DEFAULT 0,            -- 0-100
    output_path   TEXT         NOT NULL DEFAULT '',
    error_msg     TEXT         NOT NULL DEFAULT '',

    -- P0/P1 对齐：调度/分片/GPU 细粒度字段
    partition_id                BIGINT       NOT NULL DEFAULT 0,
    scheduler_type              TEXT         NOT NULL DEFAULT 'k8s_native',
    scheduler_job_id            TEXT         NOT NULL DEFAULT '',
    gpu_fraction                DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    gpu_memory_gb               BIGINT       NOT NULL DEFAULT 0,
    gpu_vendor                  TEXT         NOT NULL DEFAULT '',
    qos                         TEXT         NOT NULL DEFAULT '',
    nodes_requested             BIGINT       NOT NULL DEFAULT 1,
    node_selector               TEXT         NOT NULL DEFAULT '{}',
    affinity                    TEXT         NOT NULL DEFAULT '',
    tolerations                 TEXT         NOT NULL DEFAULT '[]',

    -- 弹性 / 容错 / 检查点
    elastic_enabled             BOOLEAN      NOT NULL DEFAULT FALSE,
    min_gpus                    BIGINT       NOT NULL DEFAULT 0,
    max_gpus                    BIGINT       NOT NULL DEFAULT 0,
    scaling_policy              TEXT         NOT NULL DEFAULT '',
    checkpoint_enabled          BOOLEAN      NOT NULL DEFAULT FALSE,
    checkpoint_interval_minutes BIGINT       NOT NULL DEFAULT 0,
    max_retries                 BIGINT       NOT NULL DEFAULT 0,
    retry_count                 BIGINT       NOT NULL DEFAULT 0,
    fault_tolerance_level       TEXT         NOT NULL DEFAULT 'none',  -- none | node | rack | switch
    topology_affinity           TEXT         NOT NULL DEFAULT '',      -- node | rack | switch | cluster
    topology_anti_affinity      BOOLEAN      NOT NULL DEFAULT FALSE,
    network_requirement         TEXT         NOT NULL DEFAULT 'any'    -- rdma | ethernet | any
);

CREATE INDEX IF NOT EXISTS idx_jobs_deleted_at ON jobs (deleted_at);
CREATE INDEX IF NOT EXISTS idx_jobs_status_cluster_user ON jobs (status, cluster_id, user_id);
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_id ON jobs (tenant_id);
CREATE INDEX IF NOT EXISTS idx_jobs_type ON jobs (type);

-- 2) gpu_devices GPU 设备表
CREATE TABLE IF NOT EXISTS gpu_devices (
    id                  BIGSERIAL    PRIMARY KEY,
    created_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ  NULL,
    cluster_id          BIGINT       NOT NULL DEFAULT 0,
    node_name           TEXT         NOT NULL DEFAULT '',
    vendor              TEXT         NOT NULL DEFAULT '',      -- nvidia | enflame | moore_threads | domestic_x
    model               TEXT         NOT NULL DEFAULT '',      -- A100 / V100 / T4 ...
    gpu_index           BIGINT       NOT NULL DEFAULT 0,       -- JSON 输出名为 "index"
    total_memory_gb     BIGINT       NOT NULL DEFAULT 0,
    allocatable_memory_gb BIGINT      NOT NULL DEFAULT 0,
    used_memory_gb      BIGINT       NOT NULL DEFAULT 0,
    mig_enabled         BOOLEAN      NOT NULL DEFAULT FALSE,
    mig_profiles        TEXT         NOT NULL DEFAULT '[]',    -- JSON array of MIG slices（Rust String 承载）
    driver_version      TEXT         NOT NULL DEFAULT '',
    cuda_version        TEXT         NOT NULL DEFAULT '',
    status              TEXT         NOT NULL DEFAULT 'available', -- available | allocated | maintenance | fault
    utilization         DOUBLE PRECISION NOT NULL DEFAULT 0,
    temperature         BIGINT       NOT NULL DEFAULT 0,
    power_draw          BIGINT       NOT NULL DEFAULT 0,
    details             TEXT         NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_gpu_devices_deleted_at ON gpu_devices (deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_devices_cluster_status ON gpu_devices (cluster_id, status);

-- 3) gpu_allocations GPU 分配记录表
CREATE TABLE IF NOT EXISTS gpu_allocations (
    id            BIGSERIAL    PRIMARY KEY,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ  NULL,
    device_id     BIGINT       NOT NULL DEFAULT 0,
    job_id        BIGINT       NOT NULL DEFAULT 0,
    tenant_id     BIGINT       NOT NULL DEFAULT 0,
    user_id       BIGINT       NOT NULL DEFAULT 0,
    fraction      DOUBLE PRECISION NOT NULL DEFAULT 1.0,   -- 1.0 | 0.5 | 0.25
    memory_gb     BIGINT       NOT NULL DEFAULT 0,
    mig_profile   TEXT         NOT NULL DEFAULT '',
    status        TEXT         NOT NULL DEFAULT 'active',   -- active | released | failed
    started_at    TIMESTAMPTZ  NULL,
    ended_at      TIMESTAMPTZ  NULL
);

CREATE INDEX IF NOT EXISTS idx_gpu_allocations_deleted_at ON gpu_allocations (deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_job_status ON gpu_allocations (job_id, status);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_device_id ON gpu_allocations (device_id);
