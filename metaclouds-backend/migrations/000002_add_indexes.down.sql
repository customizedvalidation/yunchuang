-- ==============================================================================
-- 000002_add_indexes.down.sql
-- 回滚 000002_add_indexes.up.sql
-- 删除所有新增索引，使用 IF EXISTS 保证幂等。
-- ==============================================================================

-- 调度计划表索引
DROP INDEX IF EXISTS idx_schedules_enabled_next_run;

-- 集群表索引
DROP INDEX IF EXISTS idx_clusters_status;

-- 用户表索引
DROP INDEX IF EXISTS idx_users_tenant_id;
DROP INDEX IF EXISTS idx_users_email;

-- 作业表索引
DROP INDEX IF EXISTS idx_jobs_cluster_status;
DROP INDEX IF EXISTS idx_jobs_tenant_status;
DROP INDEX IF EXISTS idx_jobs_status_priority;

-- 资源表索引
DROP INDEX IF EXISTS idx_resources_type_status;
DROP INDEX IF EXISTS idx_resources_cluster_status;
DROP INDEX IF EXISTS idx_resources_status;
DROP INDEX IF EXISTS idx_resources_type;
DROP INDEX IF EXISTS idx_resources_cluster_id;

-- ==============================================================================
-- 索引回滚完成
-- 所有 000002 中创建的索引已删除。
-- ==============================================================================
