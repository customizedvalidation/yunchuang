-- P1-3 弹性配额借用（对齐 Kueue Cohort nominalQuota + borrow/lend）。
-- PostgreSQL variant，语义等价于 ../011_quota_elastic_borrow.sql。
--
-- 方言映射：
--   borrow_enabled             -> BIGINT NOT NULL DEFAULT 0
--   borrow_limit_gpu           -> BIGINT NOT NULL DEFAULT 0
--   borrow_limit_cpu           -> DOUBLE PRECISION NOT NULL DEFAULT 0
--   borrow_limit_memory_gb     -> DOUBLE PRECISION NOT NULL DEFAULT 0
--   borrow_limit_storage_gb    -> DOUBLE PRECISION NOT NULL DEFAULT 0

ALTER TABLE resource_quotas ADD COLUMN borrow_enabled BIGINT NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_gpu BIGINT NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_cpu DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_memory_gb DOUBLE PRECISION NOT NULL DEFAULT 0;
ALTER TABLE resource_quotas ADD COLUMN borrow_limit_storage_gb DOUBLE PRECISION NOT NULL DEFAULT 0;
