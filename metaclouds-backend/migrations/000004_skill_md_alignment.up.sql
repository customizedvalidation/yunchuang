-- ==============================================================================
-- 000004_skill_md_alignment.up.sql
-- P0/P1 差距对齐：GPU 细粒度管理、分区、配额、调度器集成、拓扑、加速套件
-- 所有表均使用 IF NOT EXISTS，列添加使用 IF NOT EXISTS 模式，保证迁移幂等。
-- ==============================================================================

-- ------------------------------------------------------------------------------
-- GPU 细粒度设备表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gpu_devices (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    cluster_id INTEGER,
    node_name VARCHAR(255),
    vendor VARCHAR(50) NOT NULL,
    model VARCHAR(100),
    index INTEGER NOT NULL DEFAULT 0,
    total_memory_gb INTEGER NOT NULL DEFAULT 0,
    allocatable_memory_gb INTEGER NOT NULL DEFAULT 0,
    used_memory_gb INTEGER NOT NULL DEFAULT 0,
    mig_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    mig_profiles VARCHAR(500),
    driver_version VARCHAR(50),
    cuda_version VARCHAR(50),
    status VARCHAR(50) NOT NULL DEFAULT 'available',
    utilization DOUBLE PRECISION NOT NULL DEFAULT 0,
    temperature INTEGER NOT NULL DEFAULT 0,
    power_draw INTEGER NOT NULL DEFAULT 0,
    details VARCHAR(1000)
);
CREATE INDEX IF NOT EXISTS idx_gpu_devices_deleted_at ON gpu_devices(deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_devices_cluster_id ON gpu_devices(cluster_id);

-- ------------------------------------------------------------------------------
-- GPU 细粒度分配记录表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gpu_allocations (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    device_id INTEGER,
    job_id INTEGER,
    tenant_id INTEGER,
    user_id INTEGER,
    fraction DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    memory_gb INTEGER NOT NULL DEFAULT 0,
    mig_profile VARCHAR(50),
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    started_at TIMESTAMP,
    ended_at TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_deleted_at ON gpu_allocations(deleted_at);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_device_id ON gpu_allocations(device_id);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_job_id ON gpu_allocations(job_id);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_tenant_id ON gpu_allocations(tenant_id);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_user_id ON gpu_allocations(user_id);

-- ------------------------------------------------------------------------------
-- 分区表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS partitions (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    cluster_id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    description VARCHAR(1000),
    priority INTEGER NOT NULL DEFAULT 0,
    max_runtime_minutes INTEGER NOT NULL DEFAULT 0,
    node_count INTEGER NOT NULL DEFAULT 0,
    cpu_limit INTEGER NOT NULL DEFAULT 0,
    gpu_count INTEGER NOT NULL DEFAULT 0,
    gpu_vendor VARCHAR(50),
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    allow_sharing BOOLEAN NOT NULL DEFAULT FALSE,
    default_qos VARCHAR(50),
    scheduler_type VARCHAR(50) NOT NULL DEFAULT 'k8s_native',
    nodes VARCHAR(2000),
    taints VARCHAR(1000),
    labels VARCHAR(1000)
);
CREATE INDEX IF NOT EXISTS idx_partitions_deleted_at ON partitions(deleted_at);
CREATE INDEX IF NOT EXISTS idx_partitions_cluster_id ON partitions(cluster_id);

-- ------------------------------------------------------------------------------
-- 分区权限表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS partition_permissions (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    partition_id INTEGER NOT NULL,
    principal_type VARCHAR(20) NOT NULL,
    principal_id INTEGER NOT NULL,
    access_level VARCHAR(20) NOT NULL DEFAULT 'view'
);
CREATE INDEX IF NOT EXISTS idx_partition_permissions_partition_id ON partition_permissions(partition_id);

-- ------------------------------------------------------------------------------
-- 多维度资源配额表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS resource_quotas (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    scope_type VARCHAR(20) NOT NULL,
    scope_id INTEGER NOT NULL,
    resource_type VARCHAR(20) NOT NULL,
    limit INTEGER NOT NULL DEFAULT 0,
    used INTEGER NOT NULL DEFAULT 0,
    gpu_fraction_limit DOUBLE PRECISION NOT NULL DEFAULT 0,
    gpu_memory_limit_gb INTEGER NOT NULL DEFAULT 0,
    gpu_memory_used_gb INTEGER NOT NULL DEFAULT 0,
    max_pod_cpu INTEGER NOT NULL DEFAULT 0,
    min_pod_cpu INTEGER NOT NULL DEFAULT 0,
    max_pod_memory INTEGER NOT NULL DEFAULT 0,
    min_pod_memory INTEGER NOT NULL DEFAULT 0,
    max_pod_gpu DOUBLE PRECISION NOT NULL DEFAULT 0,
    min_pod_gpu DOUBLE PRECISION NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'active'
);
CREATE INDEX IF NOT EXISTS idx_resource_quotas_deleted_at ON resource_quotas(deleted_at);
CREATE INDEX IF NOT EXISTS idx_resource_quotas_scope_type ON resource_quotas(scope_type);
CREATE INDEX IF NOT EXISTS idx_resource_quotas_scope_id ON resource_quotas(scope_id);

-- ------------------------------------------------------------------------------
-- 外部调度器集成表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS scheduler_integrations (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    name VARCHAR(100) NOT NULL UNIQUE,
    type VARCHAR(20) NOT NULL,
    endpoint VARCHAR(500),
    auth_type VARCHAR(20) NOT NULL DEFAULT 'none',
    auth_config VARCHAR(2000),
    version VARCHAR(50),
    status VARCHAR(20) NOT NULL DEFAULT 'inactive',
    default_partition VARCHAR(100),
    default_qos VARCHAR(50),
    max_nodes INTEGER NOT NULL DEFAULT 0,
    max_jobs INTEGER NOT NULL DEFAULT 0,
    last_sync_at TIMESTAMP,
    details VARCHAR(2000)
);
CREATE INDEX IF NOT EXISTS idx_scheduler_integrations_deleted_at ON scheduler_integrations(deleted_at);

-- ------------------------------------------------------------------------------
-- 节点拓扑表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS node_topologies (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    cluster_id INTEGER,
    node_name VARCHAR(255) NOT NULL,
    rack_id VARCHAR(100),
    switch_id VARCHAR(100),
    pod_id VARCHAR(100),
    numa_nodes VARCHAR(1000),
    gpu_topology VARCHAR(2000),
    network_type VARCHAR(50),
    network_bandwidth_gbps INTEGER NOT NULL DEFAULT 0,
    rdma_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    fault_domain VARCHAR(100)
);
CREATE INDEX IF NOT EXISTS idx_node_topologies_cluster_id ON node_topologies(cluster_id);

-- ------------------------------------------------------------------------------
-- 数据集表（Fluid 数据加速）
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS datasets (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    name VARCHAR(255) NOT NULL UNIQUE,
    description VARCHAR(1000),
    source_type VARCHAR(50) NOT NULL,
    source_path VARCHAR(1000),
    mount_path VARCHAR(500),
    size_gb INTEGER NOT NULL DEFAULT 0,
    mount_options VARCHAR(1000),
    access_mode VARCHAR(20) NOT NULL DEFAULT 'ReadWriteMany',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    tenant_id INTEGER
);
CREATE INDEX IF NOT EXISTS idx_datasets_deleted_at ON datasets(deleted_at);
CREATE INDEX IF NOT EXISTS idx_datasets_tenant_id ON datasets(tenant_id);

-- ------------------------------------------------------------------------------
-- Fluid 缓存配置表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fluid_caches (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    dataset_id INTEGER NOT NULL,
    runtime_type VARCHAR(50) NOT NULL DEFAULT 'alluxio',
    cache_capacity_gb INTEGER NOT NULL DEFAULT 0,
    replicas INTEGER NOT NULL DEFAULT 1,
    medium_type VARCHAR(20) NOT NULL DEFAULT 'memory',
    prefetch_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    prefetch_policy VARCHAR(500),
    compression_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    metadata_acceleration_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(20) NOT NULL DEFAULT 'inactive',
    cache_hit_rate DOUBLE PRECISION NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_fluid_caches_deleted_at ON fluid_caches(deleted_at);
CREATE INDEX IF NOT EXISTS idx_fluid_caches_dataset_id ON fluid_caches(dataset_id);

-- ------------------------------------------------------------------------------
-- 分布式训练配置表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS distributed_training_configs (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    job_id INTEGER NOT NULL,
    parallel_strategy VARCHAR(30) NOT NULL DEFAULT 'data',
    world_size INTEGER NOT NULL DEFAULT 1,
    tensor_parallel_size INTEGER NOT NULL DEFAULT 1,
    pipeline_stages INTEGER NOT NULL DEFAULT 1,
    deepspeed_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    deepspeed_config_path VARCHAR(500),
    megatron_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    nccl_config VARCHAR(2000),
    gradient_compression BOOLEAN NOT NULL DEFAULT FALSE,
    communication_backend VARCHAR(20) NOT NULL DEFAULT 'nccl'
);
CREATE INDEX IF NOT EXISTS idx_distributed_training_configs_job_id ON distributed_training_configs(job_id);

-- ------------------------------------------------------------------------------
-- 推理加速配置表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS inference_configs (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    job_id INTEGER NOT NULL,
    backend VARCHAR(30) NOT NULL DEFAULT 'pytorch',
    precision VARCHAR(10) NOT NULL DEFAULT 'fp32',
    quantization_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    calibration_dataset VARCHAR(500),
    batch_size INTEGER NOT NULL DEFAULT 1,
    dynamic_batching BOOLEAN NOT NULL DEFAULT FALSE,
    max_batch_size INTEGER NOT NULL DEFAULT 32,
    max_latency_ms INTEGER NOT NULL DEFAULT 0,
    model_path VARCHAR(1000),
    tensorrt_engine_path VARCHAR(500),
    gpu_utilization_target DOUBLE PRECISION NOT NULL DEFAULT 0.8
);
CREATE INDEX IF NOT EXISTS idx_inference_configs_job_id ON inference_configs(job_id);

-- ------------------------------------------------------------------------------
-- 检查点表
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS checkpoints (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP,
    job_id INTEGER NOT NULL,
    path VARCHAR(1000) NOT NULL,
    step INTEGER NOT NULL DEFAULT 0,
    epoch INTEGER NOT NULL DEFAULT 0,
    size_mb INTEGER NOT NULL DEFAULT 0,
    format VARCHAR(50) NOT NULL DEFAULT 'pytorch',
    is_latest BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(20) NOT NULL DEFAULT 'completed'
);
CREATE INDEX IF NOT EXISTS idx_checkpoints_deleted_at ON checkpoints(deleted_at);
CREATE INDEX IF NOT EXISTS idx_checkpoints_job_id ON checkpoints(job_id);

-- ------------------------------------------------------------------------------
-- ALTER TABLE: 为现有表添加新列（使用 DO $$ BEGIN IF NOT EXISTS 模式）
-- ------------------------------------------------------------------------------

-- jobs 表新增列
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='partition_id') THEN
        ALTER TABLE jobs ADD COLUMN partition_id INTEGER;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='scheduler_type') THEN
        ALTER TABLE jobs ADD COLUMN scheduler_type VARCHAR(20) NOT NULL DEFAULT 'k8s_native';
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='scheduler_job_id') THEN
        ALTER TABLE jobs ADD COLUMN scheduler_job_id VARCHAR(100);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='gpu_fraction') THEN
        ALTER TABLE jobs ADD COLUMN gpu_fraction DOUBLE PRECISION NOT NULL DEFAULT 1.0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='gpu_memory_gb') THEN
        ALTER TABLE jobs ADD COLUMN gpu_memory_gb INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='gpu_vendor') THEN
        ALTER TABLE jobs ADD COLUMN gpu_vendor VARCHAR(50);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='qos') THEN
        ALTER TABLE jobs ADD COLUMN qos VARCHAR(50);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='nodes_requested') THEN
        ALTER TABLE jobs ADD COLUMN nodes_requested INTEGER NOT NULL DEFAULT 1;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='node_selector') THEN
        ALTER TABLE jobs ADD COLUMN node_selector VARCHAR(1000);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='affinity') THEN
        ALTER TABLE jobs ADD COLUMN affinity VARCHAR(2000);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='tolerations') THEN
        ALTER TABLE jobs ADD COLUMN tolerations VARCHAR(1000);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='elastic_enabled') THEN
        ALTER TABLE jobs ADD COLUMN elastic_enabled BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='min_gpus') THEN
        ALTER TABLE jobs ADD COLUMN min_gpus INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='max_gpus') THEN
        ALTER TABLE jobs ADD COLUMN max_gpus INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='scaling_policy') THEN
        ALTER TABLE jobs ADD COLUMN scaling_policy VARCHAR(500);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='checkpoint_enabled') THEN
        ALTER TABLE jobs ADD COLUMN checkpoint_enabled BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='checkpoint_interval_minutes') THEN
        ALTER TABLE jobs ADD COLUMN checkpoint_interval_minutes INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='max_retries') THEN
        ALTER TABLE jobs ADD COLUMN max_retries INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='retry_count') THEN
        ALTER TABLE jobs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='fault_tolerance_level') THEN
        ALTER TABLE jobs ADD COLUMN fault_tolerance_level VARCHAR(20) NOT NULL DEFAULT 'none';
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='topology_affinity') THEN
        ALTER TABLE jobs ADD COLUMN topology_affinity VARCHAR(20);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='topology_anti_affinity') THEN
        ALTER TABLE jobs ADD COLUMN topology_anti_affinity BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='jobs' AND column_name='network_requirement') THEN
        ALTER TABLE jobs ADD COLUMN network_requirement VARCHAR(20);
    END IF;
END $$;

-- resources 表新增列
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='vendor') THEN
        ALTER TABLE resources ADD COLUMN vendor VARCHAR(50);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='gpu_model') THEN
        ALTER TABLE resources ADD COLUMN gpu_model VARCHAR(100);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='vram_total_mb') THEN
        ALTER TABLE resources ADD COLUMN vram_total_mb INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='vram_used_mb') THEN
        ALTER TABLE resources ADD COLUMN vram_used_mb INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='vram_oversubscription_ratio') THEN
        ALTER TABLE resources ADD COLUMN vram_oversubscription_ratio DOUBLE PRECISION NOT NULL DEFAULT 1.0;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='resources' AND column_name='mig_enabled') THEN
        ALTER TABLE resources ADD COLUMN mig_enabled BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;

-- clusters 表新增列
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='clusters' AND column_name='gpu_vendors') THEN
        ALTER TABLE clusters ADD COLUMN gpu_vendors VARCHAR(500);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='clusters' AND column_name='scheduler_types') THEN
        ALTER TABLE clusters ADD COLUMN scheduler_types VARCHAR(500);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='clusters' AND column_name='multi_cluster_enabled') THEN
        ALTER TABLE clusters ADD COLUMN multi_cluster_enabled BOOLEAN NOT NULL DEFAULT FALSE;
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='clusters' AND column_name='federation_id') THEN
        ALTER TABLE clusters ADD COLUMN federation_id VARCHAR(100);
    END IF;
END $$;

-- acceleration_suites 表新增列
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='acceleration_suites' AND column_name='config_json') THEN
        ALTER TABLE acceleration_suites ADD COLUMN config_json VARCHAR(4000);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='acceleration_suites' AND column_name='category') THEN
        ALTER TABLE acceleration_suites ADD COLUMN category VARCHAR(50);
    END IF;
END $$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='acceleration_suites' AND column_name='vendor') THEN
        ALTER TABLE acceleration_suites ADD COLUMN vendor VARCHAR(50);
    END IF;
END $$;

-- 为 jobs.partition_id 创建索引
CREATE INDEX IF NOT EXISTS idx_jobs_partition_id ON jobs(partition_id);
