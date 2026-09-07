-- ==============================================================================
-- 000001_init_schema.down.sql
-- 回滚 000001_init_schema.up.sql
-- 按外键依赖反序删除所有表，使用 IF EXISTS 和 CASCADE 保证幂等。
-- ==============================================================================

-- 先删除有外键依赖的关联表
DROP TABLE IF EXISTS role_permissions CASCADE;

-- RBAC 表
DROP TABLE IF EXISTS permissions CASCADE;
DROP TABLE IF EXISTS roles CASCADE;

-- 业务表（无外键依赖或被其他表引用的表先删）
DROP TABLE IF EXISTS alerts CASCADE;
DROP TABLE IF EXISTS security_policies CASCADE;
DROP TABLE IF EXISTS acceleration_suites CASCADE;

-- 调度计划表（外键 -> jobs）
DROP TABLE IF EXISTS schedules CASCADE;

-- 作业表（外键 -> clusters, users）
DROP TABLE IF EXISTS jobs CASCADE;

-- 资源表（外键 -> clusters）
DROP TABLE IF EXISTS resources CASCADE;

-- 集群表
DROP TABLE IF EXISTS clusters CASCADE;

-- 用户表
DROP TABLE IF EXISTS users CASCADE;

-- 租户表（最后删除，无被依赖关系）
DROP TABLE IF EXISTS tenants CASCADE;

-- ==============================================================================
-- 回滚完成
-- 所有 000001 中创建的表已删除。
-- ==============================================================================
