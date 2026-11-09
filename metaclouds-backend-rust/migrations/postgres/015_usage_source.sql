-- P2-4 实耗数据来源标记（2026-11-09），PostgreSQL variant。
-- 语义与 ../015_usage_source.sql 完全一致（TEXT 可空，零破坏性变更）。

ALTER TABLE jobs ADD COLUMN usage_source TEXT NULL;
