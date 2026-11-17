-- 017 抢占契约（SQLite 方言，2026-11-16）。
--
-- ⚠️ 边界诚实声明：本迁移只提供抢占所需的可空元数据列与候选集索引，
-- 不代表抢占已在生产启用。抢占开关 DISPATCH_PREEMPTION_ENABLED 默认关闭；
-- 在模拟执行体上做抢占回归价值有限，真实启用须与真实执行器一并验证。
--
-- 设计要点（对应 docs/p2-3-preemption-backlog.md 的 M1/M2）：
--   - M1：候选 victim 必须在 SQL 层按 cluster_id / partition_id / tenant_id 收窄，
--     索引覆盖 (status, cluster_id, partition_id, tenant_id, priority)。
--   - M2：不再引入第二套 run_token；抢占走 complete/fail/reconcile 共用的
--     revert_to_pending + release_resources_for_job 幂等终态路径。
--
-- 纯增量：两列均可空，无破坏性变更；与 Postgres 版 017 同结构。

ALTER TABLE jobs ADD COLUMN preempt_exempt_until TEXT NULL;
ALTER TABLE jobs ADD COLUMN preempted_by INTEGER NULL;

CREATE INDEX IF NOT EXISTS idx_jobs_preemption_victim
    ON jobs (status, cluster_id, partition_id, tenant_id, priority);
