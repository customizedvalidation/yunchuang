-- P0-2 多副本调度租约（2026-11-09），PostgreSQL variant。
-- 语义与 ../014_dispatch_lease.sql 完全一致；TEXT 列刻意不换成 TIMESTAMPTZ，
-- 以便租约 SQL 在双驱动下共用一条语句（RFC3339 文本的字典序即时间序）。

CREATE TABLE IF NOT EXISTS dispatch_lease (
    lease_key  TEXT PRIMARY KEY,
    holder     TEXT NOT NULL,
    expires_at TEXT NULL
);
