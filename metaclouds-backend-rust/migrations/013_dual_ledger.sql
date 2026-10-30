-- P2-4 申请/实耗双账本（GPU 时间片 + 超卖回收的实耗采样基础）。
--
-- 在 `jobs`（006_b3_jobs.sql 建表）上追加 4 个可空实耗采样列：
--   - gpu_fraction_actual  采样时点的实际 GPU 份额占用（申请 fraction × 利用率）。
--   - cpu_actual           采样时点的实际 CPU 占用（核）。
--   - memory_gb_actual     采样时点的实际内存占用（GB）。
--   - usage_sampled_at     实耗采样时间戳（NULL = 尚未采样）。
--
-- 回收准入（quota::allocate_resources_with_borrow）按 Σ max(0, 申请 − 实耗) 计算租户
-- 可回收量，NULL（未采样）作业不参与（保守：视为占满申请量）。
--
-- 方言：SQLite（Phase 2 默认驱动）。全部可空，零破坏。

ALTER TABLE jobs ADD COLUMN gpu_fraction_actual REAL NULL;
ALTER TABLE jobs ADD COLUMN cpu_actual REAL NULL;
ALTER TABLE jobs ADD COLUMN memory_gb_actual REAL NULL;
ALTER TABLE jobs ADD COLUMN usage_sampled_at TEXT NULL;
