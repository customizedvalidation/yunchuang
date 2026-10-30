-- P2-4 申请/实耗双账本（GPU 时间片 + 超卖回收的实耗采样基础）。
-- PostgreSQL variant，语义等价于 ../013_dual_ledger.sql。
--
-- 方言映射：
--   gpu_fraction_actual -> DOUBLE PRECISION NULL
--   cpu_actual          -> DOUBLE PRECISION NULL
--   memory_gb_actual    -> DOUBLE PRECISION NULL
--   usage_sampled_at    -> TIMESTAMPTZ NULL

ALTER TABLE jobs ADD COLUMN gpu_fraction_actual DOUBLE PRECISION NULL;
ALTER TABLE jobs ADD COLUMN cpu_actual DOUBLE PRECISION NULL;
ALTER TABLE jobs ADD COLUMN memory_gb_actual DOUBLE PRECISION NULL;
ALTER TABLE jobs ADD COLUMN usage_sampled_at TIMESTAMPTZ NULL;
