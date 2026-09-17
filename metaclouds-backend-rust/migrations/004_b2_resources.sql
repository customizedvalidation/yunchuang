-- WP-P2-B2 资源域迁移：NodeTopology 节点拓扑表。
--
-- 说明：
--   - resources / clusters 表已在 migrations/002_tenants_clusters_resources.sql 建好，
--     且字段已与 Go models/resource.go / cluster.go 全部对齐（含多 GPU 厂商与显存字段），
--     本迁移不再重复创建或 ALTER（避免对已执行库做破坏性变更）。
--   - 本迁移仅新增 topology_nodes（节点拓扑感知）表与所需索引。
--
-- 方言：SQLite（Phase 2 仅在 SQLite 实际执行）。
--   - JSON 字段 labels 以 TEXT 存储，由 crate::orm::Json<T> 负责编解码。
--   - 软删除时间戳为 TEXT（ISO8601），由 sqlx chrono 编解码层读写。

-- 1) topology_nodes 节点拓扑表
CREATE TABLE IF NOT EXISTS topology_nodes (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at    TEXT    NOT NULL,
    updated_at    TEXT    NOT NULL,
    deleted_at    TEXT    NULL,
    cluster_id    INTEGER NOT NULL DEFAULT 0,
    hostname      TEXT    NOT NULL DEFAULT '',
    ip            TEXT    NOT NULL DEFAULT '',
    role          TEXT    NOT NULL DEFAULT 'worker',   -- worker | master
    cpu_cores     INTEGER NOT NULL DEFAULT 0,
    memory_gb     INTEGER NOT NULL DEFAULT 0,
    gpu_count     INTEGER NOT NULL DEFAULT 0,
    gpu_model     TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'unknown',
    labels        TEXT    NOT NULL DEFAULT '{}'        -- JSON object (K8s node labels)
);

-- 2) 索引：软删除过滤 + (cluster_id, role) 联合过滤
CREATE INDEX IF NOT EXISTS idx_topology_nodes_deleted_at ON topology_nodes (deleted_at);
CREATE INDEX IF NOT EXISTS idx_topology_nodes_cluster_role ON topology_nodes (cluster_id, role);
CREATE INDEX IF NOT EXISTS idx_topology_nodes_hostname ON topology_nodes (hostname);

-- resources(cluster_id) 索引已在 002 建立（idx_resources_cluster_id），此处不重复。
