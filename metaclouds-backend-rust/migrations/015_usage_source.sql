-- P2-4 实耗数据来源标记（2026-11-09）。
--
-- 迁移 013 增加的 jobs.*_actual 列当前由 dispatch::sample_job_usage 写入
-- 「申请量 × simulated_utilization」的合成值，真实 DCGM 采集尚未接入。
-- 若计费/showback 直接读 *_actual，就会对从未发生过的算力用量出账。
--
-- 增加 usage_source 让合成数据与真实采集在数据层可判定：
--   'simulated' = 演示采样（禁止计费）；
--   NULL        = 尚未采样；
--   'dcgm' 等   = 未来的真实采集源（由采集器写入）。
-- 计费/对账查询必须显式过滤 usage_source，不能只看 usage_sampled_at IS NOT NULL。
--
-- 方言：SQLite（Phase 2 默认驱动）。Postgres 版本见 migrations/postgres/015_usage_source.sql。

ALTER TABLE jobs ADD COLUMN usage_source TEXT NULL;
