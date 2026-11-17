-- SQLite 方言回滚 SQL，与 metaclouds-backend-rust/migrations/NNN_*.sql 一一对应。
--
-- 段落标记 `-- @down NNN` 表示「撤销第 NNN 号 up 迁移」。
-- 文件内按 NNN 降序排列，因此整文件顺序执行即等价于完整回滚。
-- scripts/rollback-migrations.sh 会按 `--to` 目标版本截取需要的段落。
--
-- 注意：SQLite 的 ALTER TABLE DROP COLUMN（3.35+）不能删除仍被索引引用的列，
-- 因此每段都先 DROP INDEX 再 DROP COLUMN；DROP TABLE 会连带删除其索引。
-- 本文件不含对 _sqlx_migrations 的修改——回滚脚本负责同步该元数据表，
-- 否则重新 up 时 SQLx 会误以为迁移已应用而跳过。

-- @down 017
DROP INDEX IF EXISTS idx_jobs_preemption_victim;
ALTER TABLE jobs DROP COLUMN preempt_exempt_until;
ALTER TABLE jobs DROP COLUMN preempted_by;

-- @down 016
DROP INDEX IF EXISTS idx_exec_tasks_lease;
DROP TABLE IF EXISTS exec_tasks;
ALTER TABLE jobs DROP COLUMN exec_attempt;
ALTER TABLE jobs DROP COLUMN executor_backend;

-- @down 015
ALTER TABLE jobs DROP COLUMN usage_source;

-- @down 014
DROP TABLE IF EXISTS dispatch_lease;

-- @down 013
ALTER TABLE jobs DROP COLUMN gpu_fraction_actual;
ALTER TABLE jobs DROP COLUMN cpu_actual;
ALTER TABLE jobs DROP COLUMN memory_gb_actual;
ALTER TABLE jobs DROP COLUMN usage_sampled_at;

-- @down 012
ALTER TABLE jobs DROP COLUMN gang_replicas;
ALTER TABLE jobs DROP COLUMN gang_min_available;
ALTER TABLE jobs DROP COLUMN gang_fulfilled;

-- @down 011
ALTER TABLE resource_quotas DROP COLUMN borrow_enabled;
ALTER TABLE resource_quotas DROP COLUMN borrow_limit_gpu;
ALTER TABLE resource_quotas DROP COLUMN borrow_limit_cpu;
ALTER TABLE resource_quotas DROP COLUMN borrow_limit_memory_gb;
ALTER TABLE resource_quotas DROP COLUMN borrow_limit_storage_gb;

-- @down 010
DROP INDEX IF EXISTS idx_gpu_allocations_user_status;
DROP INDEX IF EXISTS idx_gpu_allocations_tenant_status;

-- @down 009
DROP INDEX IF EXISTS idx_partitions_priority;
ALTER TABLE partitions DROP COLUMN priority;
ALTER TABLE partitions DROP COLUMN max_runtime_minutes;

-- @down 008
DROP TABLE IF EXISTS security_policies;
DROP TABLE IF EXISTS alerts;

-- @down 007
DROP TABLE IF EXISTS scheduler_integrations;
DROP TABLE IF EXISTS resource_quotas;
DROP TABLE IF EXISTS partition_permissions;
DROP TABLE IF EXISTS partitions;

-- @down 006
DROP TABLE IF EXISTS gpu_allocations;
DROP TABLE IF EXISTS gpu_devices;
DROP TABLE IF EXISTS jobs;

-- @down 005
DROP TABLE IF EXISTS checkpoints;
DROP TABLE IF EXISTS acceleration_suites;
DROP TABLE IF EXISTS inference_configs;
DROP TABLE IF EXISTS distributed_training_configs;
DROP TABLE IF EXISTS fluid_caches;
DROP TABLE IF EXISTS datasets;

-- @down 004
DROP TABLE IF EXISTS topology_nodes;

-- @down 003
DROP INDEX IF EXISTS idx_users_last_login_at;
ALTER TABLE users DROP COLUMN last_login_at;

-- @down 002
DROP INDEX IF EXISTS idx_users_deleted_at;
ALTER TABLE users DROP COLUMN deleted_at;
DROP TABLE IF EXISTS resources;
DROP TABLE IF EXISTS clusters;
DROP TABLE IF EXISTS tenants;

-- @down 001
DROP INDEX IF EXISTS idx_users_username;
DROP INDEX IF EXISTS idx_users_email;
DROP TABLE IF EXISTS users;
