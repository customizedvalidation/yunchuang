-- WP-P2-B6：治理域（Alert / SecurityPolicy）。
--
-- 方言：SQLite。JSON 字段统一以 TEXT 存储，由 orm::Json<T> 负责编解码。
-- 所有表含 id / created_at / updated_at / deleted_at 软删除三件套 + tenant_id FK。
--
-- 注意：`type` 是 SQL 关键字，应用层 Rust 字段用 `kind` + #[sqlx(rename = "type")]。

-- 1) alerts 告警表
CREATE TABLE IF NOT EXISTS alerts (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at        TEXT    NOT NULL,
    updated_at        TEXT    NOT NULL,
    deleted_at        TEXT    NULL,
    name              TEXT    NOT NULL DEFAULT '',
    description       TEXT    NOT NULL DEFAULT '',
    severity          TEXT    NOT NULL DEFAULT 'warning',   -- info / warning / critical
    type              TEXT    NOT NULL DEFAULT 'system',    -- resource / job / gpu / system / cluster
    status            TEXT    NOT NULL DEFAULT 'active',    -- active / acknowledged / resolved
    source            TEXT    NOT NULL DEFAULT '',
    message           TEXT    NOT NULL DEFAULT '',
    cluster_id        INTEGER NULL,
    job_id            INTEGER NULL,
    resource_id       INTEGER NULL,
    tenant_id         INTEGER NOT NULL DEFAULT 1,
    triggered_at      TEXT    NOT NULL,
    acknowledged_at   TEXT    NULL,
    resolved_at       TEXT    NULL,
    acknowledged_by   INTEGER NULL,
    metadata          TEXT    NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_alerts_deleted_at ON alerts (deleted_at);
CREATE INDEX IF NOT EXISTS idx_alerts_status_severity ON alerts (status, severity);
CREATE INDEX IF NOT EXISTS idx_alerts_type_cluster ON alerts (type, cluster_id);
CREATE INDEX IF NOT EXISTS idx_alerts_tenant ON alerts (tenant_id);

-- 2) security_policies 安全策略表
CREATE TABLE IF NOT EXISTS security_policies (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL,
    deleted_at  TEXT    NULL,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    policy_type TEXT    NOT NULL DEFAULT 'rbac',     -- rbac / network / quota / audit
    effect      TEXT    NOT NULL DEFAULT 'allow',    -- allow / deny
    resources   TEXT    NOT NULL DEFAULT '[]',
    actions     TEXT    NOT NULL DEFAULT '[]',
    conditions  TEXT    NOT NULL DEFAULT '{}',
    priority    INTEGER NOT NULL DEFAULT 100,
    enabled     INTEGER NOT NULL DEFAULT 1,
    tenant_id   INTEGER NOT NULL DEFAULT 1,
    created_by  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_security_policies_deleted_at ON security_policies (deleted_at);
CREATE INDEX IF NOT EXISTS idx_security_policies_type_enabled ON security_policies (policy_type, enabled);
CREATE INDEX IF NOT EXISTS idx_security_policies_tenant ON security_policies (tenant_id);
