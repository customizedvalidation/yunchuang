-- ==============================================================================
-- 000002_add_indexes.up.sql
-- 性能优化索引迁移
-- 从 deploy/migrations/20260528_add_indexes.sql 迁移并标准化。
-- 所有索引使用 IF NOT EXISTS，保证迁移幂等。
--
-- 注意：本迁移中的索引均为普通 CREATE INDEX（非 CONCURRENTLY）。
--       golang-migrate 默认在事务中执行迁移，而 CREATE INDEX CONCURRENTLY
--       不能在事务块中运行。如需零停机创建索引，应使用单独的迁移文件并
--       在 migrate 命令中加 --no-transaction 选项，或在应用层手动执行。
-- ==============================================================================

-- ------------------------------------------------------------------------------
-- 资源表（resources）索引
-- ------------------------------------------------------------------------------

-- 按集群查询资源
CREATE INDEX IF NOT EXISTS idx_resources_cluster_id
    ON resources (cluster_id);

-- 按资源类型查询
CREATE INDEX IF NOT EXISTS idx_resources_type
    ON resources (type);

-- 按资源状态查询
CREATE INDEX IF NOT EXISTS idx_resources_status
    ON resources (status);

-- 复合索引：按集群和状态查询资源（常用查询模式）
CREATE INDEX IF NOT EXISTS idx_resources_cluster_status
    ON resources (cluster_id, status);

-- 复合索引：按类型和状态查询资源
CREATE INDEX IF NOT EXISTS idx_resources_type_status
    ON resources (type, status);

-- ------------------------------------------------------------------------------
-- 作业表（jobs）索引
-- ------------------------------------------------------------------------------

-- 复合索引：按状态和优先级查询作业（调度器核心查询路径）
CREATE INDEX IF NOT EXISTS idx_jobs_status_priority
    ON jobs (status, priority);

-- 复合索引：按租户和状态查询作业（多租户场景）
CREATE INDEX IF NOT EXISTS idx_jobs_tenant_status
    ON jobs (tenant_id, status);

-- 复合索引：按集群和状态查询作业
CREATE INDEX IF NOT EXISTS idx_jobs_cluster_status
    ON jobs (cluster_id, status);

-- ------------------------------------------------------------------------------
-- 用户表（users）索引
-- ------------------------------------------------------------------------------

-- 按邮箱查询用户（登录场景，email 已有 UNIQUE 约束但显式索引可加速）
CREATE INDEX IF NOT EXISTS idx_users_email
    ON users (email);

-- 按租户查询用户
CREATE INDEX IF NOT EXISTS idx_users_tenant_id
    ON users (tenant_id);

-- ------------------------------------------------------------------------------
-- 集群表（clusters）索引
-- ------------------------------------------------------------------------------

-- 按状态查询集群
CREATE INDEX IF NOT EXISTS idx_clusters_status
    ON clusters (status);

-- ------------------------------------------------------------------------------
-- 调度计划表（schedules）索引
-- ------------------------------------------------------------------------------

-- 按启用状态和下次运行时间查询（调度器轮询路径）
CREATE INDEX IF NOT EXISTS idx_schedules_enabled_next_run
    ON schedules (enabled, next_run);

-- ==============================================================================
-- 索引创建完成
-- 索引清单：
--   resources: idx_resources_cluster_id, idx_resources_type, idx_resources_status,
--              idx_resources_cluster_status, idx_resources_type_status
--   jobs:      idx_jobs_status_priority, idx_jobs_tenant_status, idx_jobs_cluster_status
--   users:     idx_users_email, idx_users_tenant_id
--   clusters:  idx_clusters_status
--   schedules: idx_schedules_enabled_next_run
-- ==============================================================================
