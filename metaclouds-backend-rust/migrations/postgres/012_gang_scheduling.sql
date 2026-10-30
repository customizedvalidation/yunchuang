-- P2-1 Gang 调度（对齐 Volcano PodGroup minMember / Borg gang 语义）。
-- PostgreSQL variant，语义等价于 ../012_gang_scheduling.sql。
--
-- 方言映射：
--   gang_replicas      -> BIGINT NOT NULL DEFAULT 1
--   gang_min_available -> BIGINT NOT NULL DEFAULT 1
--   gang_fulfilled     -> BIGINT NOT NULL DEFAULT 0

ALTER TABLE jobs ADD COLUMN gang_replicas BIGINT NOT NULL DEFAULT 1;
ALTER TABLE jobs ADD COLUMN gang_min_available BIGINT NOT NULL DEFAULT 1;
ALTER TABLE jobs ADD COLUMN gang_fulfilled BIGINT NOT NULL DEFAULT 0;
