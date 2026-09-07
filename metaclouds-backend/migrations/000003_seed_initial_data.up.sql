-- ==============================================================================
-- 000003_seed_initial_data.up.sql
-- 初始种子数据迁移
-- 包含：默认租户、管理员用户、RBAC 角色与权限、系统配置。
-- 所有 INSERT 使用 ON CONFLICT DO NOTHING，保证迁移幂等可重复执行。
--
-- 安全注意：管理员密码使用占位符 'CHANGE_ME_ON_DEPLOY'，
--           部署后必须立即通过应用或 SQL 修改为强密码。
--           切勿在生产环境使用此占位符密码。
-- ==============================================================================

-- ------------------------------------------------------------------------------
-- 1. 默认租户
-- ------------------------------------------------------------------------------
INSERT INTO tenants (name, description, status, gpu_quota, cpu_quota, memory_quota, storage_quota)
VALUES ('Default Tenant', '默认系统租户，系统初始化时自动创建', 'active', 100, 1000, 10000, 10000)
ON CONFLICT (name) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 2. 管理员用户
-- 注意：password 为占位符，部署后必须修改！
-- ------------------------------------------------------------------------------
INSERT INTO users (username, email, password, role, tenant_id)
VALUES ('admin', 'admin@metaclouds.com', 'CHANGE_ME_ON_DEPLOY', 'admin', 1)
ON CONFLICT (username) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 3. RBAC 角色
-- ------------------------------------------------------------------------------

-- 管理员角色：拥有所有权限
INSERT INTO roles (name, description)
VALUES ('admin', '系统管理员，拥有全部权限')
ON CONFLICT (name) DO NOTHING;

-- 只读角色：仅可查看资源
INSERT INTO roles (name, description)
VALUES ('viewer', '只读用户，仅可查看资源和状态')
ON CONFLICT (name) DO NOTHING;

-- 普通用户角色：可管理自己的作业和资源
INSERT INTO roles (name, description)
VALUES ('user', '普通用户，可创建和管理自己的作业')
ON CONFLICT (name) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 4. RBAC 权限定义
-- 命名规范：<resource>:<action>
-- ------------------------------------------------------------------------------

-- 集群管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('cluster:read', '查看集群信息', 'cluster', 'read'),
    ('cluster:write', '创建和修改集群', 'cluster', 'write'),
    ('cluster:delete', '删除集群', 'cluster', 'delete')
ON CONFLICT (name) DO NOTHING;

-- 作业管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('job:read', '查看作业信息', 'job', 'read'),
    ('job:write', '创建和修改作业', 'job', 'write'),
    ('job:delete', '删除作业', 'job', 'delete'),
    ('job:schedule', '调度和排队作业', 'job', 'schedule')
ON CONFLICT (name) DO NOTHING;

-- 资源管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('resource:read', '查看资源信息', 'resource', 'read'),
    ('resource:write', '创建和修改资源', 'resource', 'write'),
    ('resource:delete', '删除资源', 'resource', 'delete')
ON CONFLICT (name) DO NOTHING;

-- 用户管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('user:read', '查看用户信息', 'user', 'read'),
    ('user:write', '创建和修改用户', 'user', 'write'),
    ('user:delete', '删除用户', 'user', 'delete')
ON CONFLICT (name) DO NOTHING;

-- 租户管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('tenant:read', '查看租户信息', 'tenant', 'read'),
    ('tenant:write', '创建和修改租户', 'tenant', 'write'),
    ('tenant:delete', '删除租户', 'tenant', 'delete')
ON CONFLICT (name) DO NOTHING;

-- 系统管理权限
INSERT INTO permissions (name, description, resource, action)
VALUES
    ('system:admin', '系统管理（配置、监控、告警等）', 'system', 'admin'),
    ('system:read', '查看系统状态和监控', 'system', 'read')
ON CONFLICT (name) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 5. 角色-权限关联
-- ------------------------------------------------------------------------------

-- admin 角色：拥有所有权限（通过子查询关联所有权限 ID）
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'admin'
ON CONFLICT (role_id, permission_id) DO NOTHING;

-- viewer 角色：仅拥有所有 read 权限
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'viewer'
  AND p.action = 'read'
ON CONFLICT (role_id, permission_id) DO NOTHING;

-- user 角色：拥有作业和资源的读写权限，以及系统只读
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'user'
  AND (
      (p.resource IN ('job', 'resource') AND p.action IN ('read', 'write', 'schedule'))
      OR p.name = 'system:read'
      OR p.name = 'cluster:read'
  )
ON CONFLICT (role_id, permission_id) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 6. 系统配置：加速套件
-- ------------------------------------------------------------------------------
INSERT INTO acceleration_suites (name, description, type, version, status, enabled, details)
VALUES ('CUDA-11.7', 'NVIDIA CUDA 11.7 工具包', 'cuda', '11.7.1', 'active', TRUE,
        '{"framework": "PyTorch", "compute_capability": "8.0"}')
ON CONFLICT (name) DO NOTHING;

-- ------------------------------------------------------------------------------
-- 7. 系统配置：安全策略
-- ------------------------------------------------------------------------------
INSERT INTO security_policies (name, description, type, status, enabled, rules, details)
VALUES ('network-isolation', '启用租户间网络隔离', 'network', 'active', TRUE,
        '{"ingress": "deny-all", "egress": "allow-local"}', '默认网络隔离策略')
ON CONFLICT (name) DO NOTHING;

-- ==============================================================================
-- 种子数据插入完成
-- 数据清单：
--   租户: 1 个（Default Tenant）
--   用户: 1 个（admin，密码为占位符）
--   角色: 3 个（admin, viewer, user）
--   权限: 18 个（覆盖 cluster/job/resource/user/tenant/system）
--   角色权限关联: admin(全部), viewer(只读), user(作业/资源读写)
--   加速套件: 1 个（CUDA-11.7）
--   安全策略: 1 个（network-isolation）
--
-- 部署后必做：
--   1. 修改 admin 用户密码（当前为 CHANGE_ME_ON_DEPLOY）
--   2. 根据实际需求调整租户配额
--   3. 验证 RBAC 权限分配是否符合预期
-- ==============================================================================
