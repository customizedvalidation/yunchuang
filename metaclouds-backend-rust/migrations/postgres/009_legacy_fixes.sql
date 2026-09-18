-- 009_legacy_fixes.sql — A 类遗留字段补齐。PostgreSQL variant，
-- 语义等价于 ../009_legacy_fixes.sql。
--
-- 背景：Phase 2 建 partitions 表时未包含 Go models/partition.go 中的
-- `Priority int` 与 `MaxRuntimeMinutes int` 两列；Go 版
-- PUT /partitions/:id/priority | /max-runtime 路由需要持久化这两个字段。
--
-- 方言：Postgres。ALTER TABLE ADD COLUMN IF NOT EXISTS 幂等，带 DEFAULT 0，
-- 存量行自动回填 0；0 语义对齐 Go：priority=0 默认，max_runtime_minutes=0 表示不限时。
-- 两列在 Rust 侧为 i64，故用 BIGINT。

ALTER TABLE partitions ADD COLUMN IF NOT EXISTS priority BIGINT NOT NULL DEFAULT 0;
ALTER TABLE partitions ADD COLUMN IF NOT EXISTS max_runtime_minutes BIGINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_partitions_priority ON partitions (priority);
