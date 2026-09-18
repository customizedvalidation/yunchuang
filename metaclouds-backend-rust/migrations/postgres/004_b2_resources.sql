-- WP-P2-B2 资源域迁移：NodeTopology 节点拓扑表。
-- PostgreSQL variant，语义等价于 ../004_b2_resources.sql。
--
-- 说明：
--   - resources / clusters 表已在 002 建好且字段与 Go models 全部对齐，
--     本迁移不再重复创建或 ALTER。
--   - 本迁移仅新增 topology_nodes（节点拓扑感知）表与所需索引。
--
-- 方言映射：
--   JSON 列 labels（Rust orm::Json<Value>）-> JSONB DEFAULT '{}'::jsonb
--   软删除时间戳                            -> TIMESTAMPTZ NULL

-- 1) topology_nodes 节点拓扑表
CREATE TABLE IF NOT EXISTS topology_nodes (
    id            BIGSERIAL    PRIMARY KEY,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ  NULL,
    cluster_id    BIGINT       NOT NULL DEFAULT 0,
    hostname      TEXT         NOT NULL DEFAULT '',
    ip            TEXT         NOT NULL DEFAULT '',
    role          TEXT         NOT NULL DEFAULT 'worker',   -- worker | master
    cpu_cores     BIGINT       NOT NULL DEFAULT 0,
    memory_gb     BIGINT       NOT NULL DEFAULT 0,
    gpu_count     BIGINT       NOT NULL DEFAULT 0,
    gpu_model     TEXT         NOT NULL DEFAULT '',
    status        TEXT         NOT NULL DEFAULT 'unknown',
    labels        JSONB        NOT NULL DEFAULT '{}'::jsonb  -- K8s node labels
);

-- 2) 索引：软删除过滤 + (cluster_id, role) 联合过滤
CREATE INDEX IF NOT EXISTS idx_topology_nodes_deleted_at ON topology_nodes (deleted_at);
CREATE INDEX IF NOT EXISTS idx_topology_nodes_cluster_role ON topology_nodes (cluster_id, role);
CREATE INDEX IF NOT EXISTS idx_topology_nodes_hostname ON topology_nodes (hostname);

-- resources(cluster_id) 索引已在 002 建立（idx_resources_cluster_id），此处不重复。
