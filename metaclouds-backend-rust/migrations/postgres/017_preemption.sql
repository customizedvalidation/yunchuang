-- 017 抢占契约（PostgreSQL 方言，2026-11-16）。
-- 结构与 SQLite 版 017 完全一致；preempt_exempt_until 用 RFC3339 UTC 文本，
-- 字典序即时间序，与 dispatch_lease / exec_tasks 的租约比较语义保持一致。
-- 本迁移只提供存储契约，抢占开关默认关闭，真实启用须与真实执行器一并验证。

ALTER TABLE jobs ADD COLUMN IF NOT EXISTS preempt_exempt_until TEXT NULL;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS preempted_by BIGINT NULL;

CREATE INDEX IF NOT EXISTS idx_jobs_preemption_victim
    ON jobs (status, cluster_id, partition_id, tenant_id, priority);
