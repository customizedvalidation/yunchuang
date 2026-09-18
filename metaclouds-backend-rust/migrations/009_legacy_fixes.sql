-- 009_legacy_fixes.sql — A 类遗留字段补齐。
--
-- 背景：Phase 2 建 partitions 表时未包含 Go `models/partition.go` 中的
-- `Priority int` 与 `MaxRuntimeMinutes int` 两列；而 Go 版
-- `PUT /partitions/:id/priority` / `PUT /partitions/:id/max-runtime`
-- 路由（partition_controller.go）需要持久化这两个字段。
--
-- 方言：SQLite。ALTER TABLE ADD COLUMN 带 DEFAULT，存量行自动回填 0；
-- 0 语义对齐 Go：priority=0 默认，max_runtime_minutes=0 表示不限时。
--
-- 幂等：SQLite 不支持 ADD COLUMN IF NOT EXISTS（< 3.35），通过独立迁移
-- 文件 + sqlx 版本表保证只执行一次，故不再加条件判断。

ALTER TABLE partitions ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
ALTER TABLE partitions ADD COLUMN max_runtime_minutes INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_partitions_priority ON partitions (priority);
