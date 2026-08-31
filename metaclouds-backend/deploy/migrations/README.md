# 数据库索引迁移指南

## 概述

本目录包含用于优化数据库查询性能的索引迁移脚本。这些索引与内存存储中的索引结构相对应，确保生产环境的数据库查询性能与内存存储保持一致。

## 索引清单

### Resource 表索引

| 索引名称 | 字段 | 用途 |
|---------|------|------|
| `idx_resources_cluster_id` | cluster_id | 按集群查询资源 |
| `idx_resources_type` | type | 按类型查询资源 |
| `idx_resources_status` | status | 按状态查询资源 |
| `idx_resources_cluster_status` | cluster_id, status | 按集群和状态查询（常用） |
| `idx_resources_type_status` | type, status | 按类型和状态查询 |

### Job 表索引

| 索引名称 | 字段 | 用途 |
|---------|------|------|
| `idx_jobs_status_priority` | status, priority | 作业调度和优先级排序（关键） |
| `idx_jobs_tenant_status` | tenant_id, status | 多租户场景查询 |
| `idx_jobs_cluster_status` | cluster_id, status | 按集群查询作业状态 |

### User 表索引

| 索引名称 | 字段 | 用途 |
|---------|------|------|
| `idx_users_email` | email | 按邮箱查询用户 |

## 迁移文件说明

| 文件 | 说明 |
|------|------|
| `20260528_add_indexes.sql` | PostgreSQL 索引创建脚本 |
| `20260528_add_indexes_sqlite.sql` | SQLite 索引创建脚本 |
| `run_migration.sh` | PostgreSQL 迁移执行脚本 |
| `migration-job.yaml` | Kubernetes Job 配置 |
| `configmap.yaml` | Kubernetes ConfigMap 配置 |

## 部署方式

### 方式一：使用 Shell 脚本（适用于 Linux/VPS 部署）

```bash
# 赋予执行权限
chmod +x run_migration.sh

# 运行迁移
./run_migration.sh <database_host> <database_port> <database_name> <database_user> <database_password>

# 示例
./run_migration.sh db.example.com 5432 metaclouds_prod admin password123
```

### 方式二：直接执行 SQL（适用于手动操作）

```bash
# 使用 psql 客户端
psql -h <host> -p <port> -d <database> -U <user> -f 20260528_add_indexes.sql

# SQLite
sqlite3 metaclouds.db < 20260528_add_indexes_sqlite.sql
```

### 方式三：Kubernetes 部署

#### 1. 创建数据库 Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: metaclouds-db-secret
type: Opaque
data:
  host: <base64-encoded-host>
  port: <base64-encoded-port>
  database: <base64-encoded-database>
  username: <base64-encoded-username>
  password: <base64-encoded-password>
```

创建命令：

```bash
kubectl create secret generic metaclouds-db-secret \
  --from-literal=host=your-db-host \
  --from-literal=port=5432 \
  --from-literal=database=metaclouds \
  --from-literal=username=admin \
  --from-literal=password=your-password
```

#### 2. 创建 ConfigMap

```bash
kubectl apply -f configmap.yaml
```

#### 3. 执行迁移 Job

```bash
kubectl apply -f migration-job.yaml
```

#### 4. 检查迁移状态

```bash
# 查看 Job 状态
kubectl get jobs metaclouds-db-migration

# 查看 Pod 日志
kubectl logs -l job-name=metaclouds-db-migration
```

## 验证迁移

### PostgreSQL

```sql
-- 查看已创建的索引
SELECT indexrelname, relname AS table_name 
FROM pg_indexes 
WHERE schemaname = 'public' 
AND relname IN ('resources', 'jobs', 'users');

-- 检查索引使用情况（迁移后运行一段时间）
SELECT 
  idx.relname AS index_name,
  tbl.relname AS table_name,
  idx_scan AS index_scans,
  idx_tup_read AS tuples_read,
  idx_tup_fetch AS tuples_fetched
FROM pg_stat_user_indexes idx
JOIN pg_class tbl ON idx.schemaname = tbl.schemaname AND idx.relname = tbl.relname
WHERE tbl.relname IN ('resources', 'jobs', 'users');
```

### SQLite

```sql
-- 查看已创建的索引
SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%';
```

## 注意事项

1. **迁移时机**：建议在低峰期执行迁移，索引创建可能会影响数据库性能
2. **幂等性**：所有 `CREATE INDEX` 语句都使用 `IF NOT EXISTS`，可安全重复执行
3. **索引维护**：索引会增加写操作的开销，需要在查询性能和写入性能之间权衡
4. **备份**：建议在执行迁移前备份数据库
5. **测试环境验证**：建议先在测试环境验证迁移脚本

## 索引与代码对应关系

| 代码索引 | 数据库索引 |
|---------|-----------|
| `ResourcesByStatus` | `idx_resources_status` |
| `ResourcesByCluster` | `idx_resources_cluster_id` |
| `ResourcesByType` | `idx_resources_type` |
| `JobsByStatus` | `jobs_status_idx` (已存在) |
| `JobsByPriority` | `jobs_priority_idx` (已存在) |
| `JobsByTenant` | `jobs_tenant_id_idx` (已存在) |
| `UsersByEmail` | `idx_users_email` |
| - | `idx_jobs_status_priority` (新增复合索引) |
| - | `idx_resources_cluster_status` (新增复合索引) |
