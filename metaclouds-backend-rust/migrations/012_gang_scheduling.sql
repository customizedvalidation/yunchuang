-- P2-1 Gang 调度（对齐 Volcano PodGroup minMember / Borg gang 语义）。
--
-- 在 `jobs`（006_b3_jobs.sql 建表）上追加 Gang 准入三列：
--   - gang_replicas        作业声明的副本总数（1 = 非 Gang）。
--   - gang_min_available   最小可运行副本数；调度时按该数量的资源做 all-or-nothing 原子预留。
--   - gang_fulfilled       已按 Gang 准入落实的最小副本数（成功准入后 = gang_min_available）。
--
-- 方言：SQLite（Phase 2 默认驱动）。ALTER TABLE ADD COLUMN + 带默认值，零破坏、安全幂等。

ALTER TABLE jobs ADD COLUMN gang_replicas INTEGER NOT NULL DEFAULT 1;
ALTER TABLE jobs ADD COLUMN gang_min_available INTEGER NOT NULL DEFAULT 1;
ALTER TABLE jobs ADD COLUMN gang_fulfilled INTEGER NOT NULL DEFAULT 0;
