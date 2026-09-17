-- WP-P2-B3 作业域迁移：jobs / gpu_devices / gpu_allocations。
--
-- 字段对齐 Go 版 models/job.go / gpu_device.go / gpu_allocation.go：
--   - jobs:            作业核心字段 + 弹性/容错/检查点字段（elastic_* / checkpoint_* / retries /
--                      fault_tolerance_level / topology_* / network_requirement）。
--   - gpu_devices:     GPU 细粒度设备（vendor / model / 显存 / MIG / 利用率遥测）。
--   - gpu_allocations: 分配记录（device_id / job_id / fraction / memory_gb / 状态 active|released）。
--
-- 方言：SQLite（Phase 2 仅在 SQLite 实际执行）。
--   - JSON 列（node_selector / affinity / tolerations / scaling_policy / mig_profiles）以 TEXT 存放，
--     Go 侧本就是 string 类型承载 JSON 文本，Rust 侧按 String 读写保持逐字对齐。
--   - 软删除时间戳为 TEXT（ISO8601），由 sqlx chrono 编解码层读写。
--   - `type` 是 SQL 关键字，jobs 表保留列名 `type`，Rust 模型用 kind + rename 映射。
--   - `index` 作为列名有歧义风险，gpu_devices 表用 `gpu_index` 承载（JSON 输出仍为 "index"）。
--   - 占位符统一用全匿名 `?`（见 services 层），本文件只建表不写查询。

-- 1) jobs 作业表
CREATE TABLE IF NOT EXISTS jobs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL,
    deleted_at    TEXT    NULL,
    cluster_id    INTEGER NOT NULL DEFAULT 0,
    tenant_id     INTEGER NOT NULL DEFAULT 0,
    user_id       INTEGER NOT NULL DEFAULT 0,
    name          TEXT    NOT NULL DEFAULT '',
    description   TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'pending',   -- pending | running | completed | failed | cancelled
    type          TEXT    NOT NULL DEFAULT '',           -- training | inference | batch
    priority      INTEGER NOT NULL DEFAULT 0,            -- 0=low 1=medium 2=high 3=critical
    gpus          INTEGER NOT NULL DEFAULT 0,
    cpus          INTEGER NOT NULL DEFAULT 0,
    memory        INTEGER NOT NULL DEFAULT 0,           -- GB
    duration      INTEGER NOT NULL DEFAULT 0,            -- 预计运行分钟
    start_time    TEXT    NULL,
    end_time      TEXT    NULL,
    progress      INTEGER NOT NULL DEFAULT 0,            -- 0-100
    output_path   TEXT    NOT NULL DEFAULT '',
    error_msg     TEXT    NOT NULL DEFAULT '',

    -- P0/P1 对齐：调度/分片/GPU 细粒度字段
    partition_id                INTEGER NOT NULL DEFAULT 0,
    scheduler_type               TEXT    NOT NULL DEFAULT 'k8s_native',
    scheduler_job_id            TEXT    NOT NULL DEFAULT '',
    gpu_fraction                REAL    NOT NULL DEFAULT 1.0,
    gpu_memory_gb               INTEGER NOT NULL DEFAULT 0,
    gpu_vendor                  TEXT    NOT NULL DEFAULT '',
    qos                         TEXT    NOT NULL DEFAULT '',
    nodes_requested             INTEGER NOT NULL DEFAULT 1,
    node_selector               TEXT    NOT NULL DEFAULT '{}',
    affinity                    TEXT    NOT NULL DEFAULT '',
    tolerations                 TEXT    NOT NULL DEFAULT '[]',

    -- 弹性 / 容错 / 检查点
    elastic_enabled             INTEGER NOT NULL DEFAULT 0,
    min_gpus                    INTEGER NOT NULL DEFAULT 0,
    max_gpus                    INTEGER NOT NULL DEFAULT 0,
    scaling_policy              TEXT    NOT NULL DEFAULT '',
    checkpoint_enabled          INTEGER NOT NULL DEFAULT 0,
    checkpoint_interval_minutes INTEGER NOT NULL DEFAULT 0,
    max_retries                 INTEGER NOT NULL DEFAULT 0,
    retry_count                 INTEGER NOT NULL DEFAULT 0,
    fault_tolerance_level       TEXT    NOT NULL DEFAULT 'none',  -- none | node | rack | switch
    topology_affinity           TEXT    NOT NULL DEFAULT '',      -- node | rack | switch | cluster
    topology_anti_affinity      INTEGER NOT NULL DEFAULT 0,
    network_requirement         TEXT    NOT NULL DEFAULT 'any'   -- rdma | ethernet | any
);

CREATE INDEX IF NOT EXISTS idx_jobs_deleted_at ON jobs (deleted_at);
CREATE INDEX IF NOT EXISTS idx_jobs_status_cluster_user ON jobs (status, cluster_id, user_id);
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_id ON jobs (tenant_id);
CREATE INDEX IF NOT EXISTS idx_jobs_type ON jobs (type);

-- 2) gpu_devices GPU 设备表
CREATE TABLE IF NOT EXISTS gpu_devices (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    deleted_at          TEXT    NULL,
    cluster_id          INTEGER NOT NULL DEFAULT 0,
    node_name           TEXT    NOT NULL DEFAULT '',
    vendor              TEXT    NOT NULL DEFAULT '',      -- nvidia | enflame | moore_threads | domestic_x
    model               TEXT    NOT NULL DEFAULT '',      -- A100 / V100 / T4 ...
    gpu_index           INTEGER NOT NULL DEFAULT 0,       -- JSON 输出名为 "index"
    total_memory_gb     INTEGER NOT NULL DEFAULT 0,
    allocatable_memory_gb INTEGER NOT NULL DEFAULT 0,
    used_memory_gb      INTEGER NOT NULL DEFAULT 0,
    mig_enabled         INTEGER NOT NULL DEFAULT 0,
    mig_profiles        TEXT    NOT NULL DEFAULT '[]',    -- JSON array of MIG slices
    driver_version      TEXT    NOT NULL DEFAULT '',
    cuda_version        TEXT    NOT NULL DEFAULT '',
    status              TEXT    NOT NULL DEFAULT 'available', -- available | allocated | maintenance | fault
    utilization         REAL    NOT NULL DEFAULT 0,
    temperature         INTEGER NOT NULL DEFAULT 0,
    power_draw          INTEGER NOT NULL DEFAULT 0,
    details             TEXT    NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_gpu_devices_deleted_at ON gpu_devices (deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_devices_cluster_status ON gpu_devices (cluster_id, status);

-- 3) gpu_allocations GPU 分配记录表
CREATE TABLE IF NOT EXISTS gpu_allocations (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL,
    deleted_at    TEXT    NULL,
    device_id     INTEGER NOT NULL DEFAULT 0,
    job_id        INTEGER NOT NULL DEFAULT 0,
    tenant_id     INTEGER NOT NULL DEFAULT 0,
    user_id       INTEGER NOT NULL DEFAULT 0,
    fraction      REAL    NOT NULL DEFAULT 1.0,    -- 1.0 | 0.5 | 0.25
    memory_gb     INTEGER NOT NULL DEFAULT 0,
    mig_profile   TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'active', -- active | released | failed
    started_at    TEXT    NULL,
    ended_at      TEXT    NULL
);

CREATE INDEX IF NOT EXISTS idx_gpu_allocations_deleted_at ON gpu_allocations (deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_job_status ON gpu_allocations (job_id, status);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_device_id ON gpu_allocations (device_id);
