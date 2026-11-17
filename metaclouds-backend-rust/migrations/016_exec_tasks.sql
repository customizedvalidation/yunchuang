-- 016 真实执行器接缝：执行任务租约表 + jobs 执行元数据（SQLite 方言）。
--
-- 对应 docs/real-executor-contract.md 的灰度①：先上 exec_tasks 表与
-- SimulatedExecutor 走完整 lease 链路，不改对外行为；真实 KubeExecutor
-- 接入前所有 handle 均为本地模拟标识，usage_source 仍写 'simulated'。
--
-- 纯增量：新增表与可空/带默认值列，无破坏性变更，与 Postgres 版 016 同结构。
-- 注意：本迁移只提供存储契约，不意味着真实执行器已接入或已实测。

ALTER TABLE jobs ADD COLUMN exec_attempt INTEGER NOT NULL DEFAULT 0;
ALTER TABLE jobs ADD COLUMN executor_backend TEXT NULL;

CREATE TABLE IF NOT EXISTS exec_tasks (
    job_id         INTEGER NOT NULL,
    attempt        INTEGER NOT NULL DEFAULT 1,
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
