-- P1-3 弹性配额借用（对齐 Kueue Cohort nominalQuota + borrow/lend）。
--
-- 在 `resource_quotas`（007_b4_scheduler.sql 建表）上追加弹性借用控制列：
--   - borrow_enabled            是否允许借用集群空闲容量（其他租户未用额度）。
--   - borrow_limit_gpu/cpu/memory_gb/storage_gb 各类资源最多可借用的上限。
--
-- 方言：SQLite（Phase 2 默认驱动）。ALTER TABLE ADD COLUMN + DEFAULT 0 安全幂等。

ALTER TABLE resource_quotas ADD COLUMN borrow_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_gpu INTEGER NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_cpu REAL NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_memory_gb REAL NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_storage_gb REAL NOT NULL DEFAULT 0;
