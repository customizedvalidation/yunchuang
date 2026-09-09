-- ==============================================================================
-- 000004_skill_md_alignment.down.sql
-- 回滚 P0/P1 差距对齐迁移
-- ==============================================================================

-- 删除新增表（按依赖反序）
DROP TABLE IF EXISTS checkpoints;
DROP TABLE IF EXISTS inference_configs;
DROP TABLE IF EXISTS distributed_training_configs;
DROP TABLE IF EXISTS fluid_caches;
DROP TABLE IF EXISTS datasets;
DROP TABLE IF EXISTS node_topologies;
DROP TABLE IF EXISTS scheduler_integrations;
DROP TABLE IF EXISTS resource_quotas;
DROP TABLE IF EXISTS partition_permissions;
DROP TABLE IF EXISTS partitions;
DROP TABLE IF EXISTS gpu_allocations;
DROP TABLE IF EXISTS gpu_devices;

-- 删除 jobs 表新增列
ALTER TABLE jobs DROP COLUMN IF EXISTS partition_id;
ALTER TABLE jobs DROP COLUMN IF EXISTS scheduler_type;
ALTER TABLE jobs DROP COLUMN IF EXISTS scheduler_job_id;
ALTER TABLE jobs DROP COLUMN IF EXISTS gpu_fraction;
ALTER TABLE jobs DROP COLUMN IF EXISTS gpu_memory_gb;
ALTER TABLE jobs DROP COLUMN IF EXISTS gpu_vendor;
ALTER TABLE jobs DROP COLUMN IF EXISTS qos;
ALTER TABLE jobs DROP COLUMN IF EXISTS nodes_requested;
ALTER TABLE jobs DROP COLUMN IF EXISTS node_selector;
ALTER TABLE jobs DROP COLUMN IF EXISTS affinity;
ALTER TABLE jobs DROP COLUMN IF EXISTS tolerations;
ALTER TABLE jobs DROP COLUMN IF EXISTS elastic_enabled;
ALTER TABLE jobs DROP COLUMN IF EXISTS min_gpus;
ALTER TABLE jobs DROP COLUMN IF EXISTS max_gpus;
ALTER TABLE jobs DROP COLUMN IF EXISTS scaling_policy;
ALTER TABLE jobs DROP COLUMN IF EXISTS checkpoint_enabled;
ALTER TABLE jobs DROP COLUMN IF EXISTS checkpoint_interval_minutes;
ALTER TABLE jobs DROP COLUMN IF EXISTS max_retries;
ALTER TABLE jobs DROP COLUMN IF EXISTS retry_count;
ALTER TABLE jobs DROP COLUMN IF EXISTS fault_tolerance_level;
ALTER TABLE jobs DROP COLUMN IF EXISTS topology_affinity;
ALTER TABLE jobs DROP COLUMN IF EXISTS topology_anti_affinity;
ALTER TABLE jobs DROP COLUMN IF EXISTS network_requirement;

-- 删除 resources 表新增列
ALTER TABLE resources DROP COLUMN IF EXISTS vendor;
ALTER TABLE resources DROP COLUMN IF EXISTS gpu_model;
ALTER TABLE resources DROP COLUMN IF EXISTS vram_total_mb;
ALTER TABLE resources DROP COLUMN IF EXISTS vram_used_mb;
ALTER TABLE resources DROP COLUMN IF EXISTS vram_oversubscription_ratio;
ALTER TABLE resources DROP COLUMN IF EXISTS mig_enabled;

-- 删除 clusters 表新增列
ALTER TABLE clusters DROP COLUMN IF EXISTS gpu_vendors;
ALTER TABLE clusters DROP COLUMN IF EXISTS scheduler_types;
ALTER TABLE clusters DROP COLUMN IF EXISTS multi_cluster_enabled;
ALTER TABLE clusters DROP COLUMN IF EXISTS federation_id;

-- 删除 acceleration_suites 表新增列
ALTER TABLE acceleration_suites DROP COLUMN IF EXISTS config_json;
ALTER TABLE acceleration_suites DROP COLUMN IF EXISTS category;
ALTER TABLE acceleration_suites DROP COLUMN IF EXISTS vendor;
