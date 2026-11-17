-- 016 真实执行器接缝：执行任务租约表 + jobs 执行元数据（PostgreSQL 方言）。
-- 结构与 SQLite 版 016 完全一致；lease_until 用 RFC3339 UTC 文本，
-- 字典序即时间序，保证两套方言的租约比较语义相同。
-- 本迁移只提供存储契约，不意味着真实执行器已接入或已实测。

ALTER TABLE jobs ADD COLUMN IF NOT EXISTS exec_attempt BIGINT NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN IF NOT EXISTS executor_backend TEXT NULL;

CREATE TABLE IF NOT EXISTS exec_tasks (
    job_id         BIGINT NOT NULL,
    attempt        BIGINT NOT NULL DEFAULT 1,
    backend        TEXT NOT NULL,
    handle         TEXT NULL,
    owner_instance TEXT NULL,
    lease_until    TEXT NULL,
    submitted_at   TEXT NULL,
    finished_at    TEXT NULL,
    last_error     TEXT NULL,
    PRIMARY KEY (job_id, attempt)
);

CREATE INDEX IF NOT EXISTS idx_exec_tasks_lease ON exec_tasks (lease_until);
