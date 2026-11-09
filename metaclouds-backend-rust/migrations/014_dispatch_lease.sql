-- P0-2 多副本调度租约（2026-11-09）。
--
-- 后端在 K8s 上以 3 副本 + HPA 运行，但 dispatch::start_dispatch_loop 每个副本都会
-- 在启动时执行 reconcile。此前没有 leader 语义：3 个副本可同时对同一批 running 作业
-- 挂接完成/回退，互踩释放配额与 GPU，并在宽限期内把正常作业误判为孤儿。
--
-- 本表提供数据库层的原子租约（不依赖进程内状态、不依赖 K8s Lease API）：
--   - lease_key = 'reconcile' 为启动期崩溃恢复的 leader 租约；
--   - holder 记录 pid/启动时刻，便于排障；
--   - expires_at 用 RFC3339 UTC 文本存储，字典序即时间序，SQLite/Postgres 同语义，
--     过期租约可被重新抢占，避免 holder 进程崩溃后永久锁死。
--
-- 方言：SQLite（Phase 2 默认驱动）；Postgres 版本见 migrations/postgres/014_dispatch_lease.sql。
-- 全表可空、无外键、无破坏性变更。

CREATE TABLE IF NOT EXISTS dispatch_lease (
    lease_key  TEXT PRIMARY KEY,
    holder     TEXT NOT NULL,
    expires_at TEXT NULL
);
