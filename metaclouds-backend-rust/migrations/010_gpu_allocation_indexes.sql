-- 010_gpu_allocation_indexes.sql — GPU 分配表按 (user/tenant, status) 复合索引。
--
-- 背景：GPU 分配列表与配额/利用率汇总常按 user_id / tenant_id 过滤 active 分配；
-- 现有索引仅覆盖 (deleted_at) / (job_id, status) / (device_id)，按用户/租户
-- 查 active allocation 会退化为全表扫。
--
-- 方言：SQLite 与 Postgres 通用（CREATE INDEX IF NOT EXISTS 两方言均支持）。

CREATE INDEX IF NOT EXISTS idx_gpu_allocations_user_status ON gpu_allocations (user_id, status);
CREATE INDEX IF NOT EXISTS idx_gpu_allocations_tenant_status ON gpu_allocations (tenant_id, status);
