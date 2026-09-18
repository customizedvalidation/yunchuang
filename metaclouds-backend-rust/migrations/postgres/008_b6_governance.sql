-- WP-P2-B6：治理域（Alert / SecurityPolicy）。
-- PostgreSQL variant，语义等价于 ../008_b6_governance.sql。
--
-- 方言映射：
--   id                                  -> BIGSERIAL PRIMARY KEY
--   时间戳（triggered_at/acknowledged_at/resolved_at/软删除）-> TIMESTAMPTZ
--   JSON（metadata / resources / actions / conditions，Rust orm::Json<T>）-> JSONB
--   priority（Rust i32）                -> INTEGER
--   enabled（Rust bool）                 -> BOOLEAN DEFAULT TRUE
--
-- 注意：`type` 列名保留（SQL 非保留字），应用层 Rust 字段用 kind + rename。

-- 1) alerts 告警表
CREATE TABLE IF NOT EXISTS alerts (
    id                BIGSERIAL    PRIMARY KEY,
    created_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ  NULL,
    name              TEXT         NOT NULL DEFAULT '',
    description       TEXT         NOT NULL DEFAULT '',
    severity          TEXT         NOT NULL DEFAULT 'warning',   -- info / warning / critical
    type              TEXT         NOT NULL DEFAULT 'system',    -- resource / job / gpu / system / cluster
    status            TEXT         NOT NULL DEFAULT 'active',    -- active / acknowledged / resolved
    source            TEXT         NOT NULL DEFAULT '',
    message           TEXT         NOT NULL DEFAULT '',
    cluster_id        BIGINT       NULL,
    job_id            BIGINT       NULL,
    resource_id       BIGINT       NULL,
    tenant_id         BIGINT       NOT NULL DEFAULT 1,
    triggered_at      TIMESTAMPTZ  NOT NULL,
    acknowledged_at   TIMESTAMPTZ  NULL,
    resolved_at       TIMESTAMPTZ  NULL,
    acknowledged_by   BIGINT       NULL,
    metadata          JSONB        NOT NULL DEFAULT '{}'::jsonb
);

CREATE INDEX IF NOT EXISTS idx_alerts_deleted_at ON alerts (deleted_at);
CREATE INDEX IF NOT EXISTS idx_alerts_status_severity ON alerts (status, severity);
CREATE INDEX IF NOT EXISTS idx_alerts_type_cluster ON alerts (type, cluster_id);
CREATE INDEX IF NOT EXISTS idx_alerts_tenant ON alerts (tenant_id);

-- 2) security_policies 安全策略表
CREATE TABLE IF NOT EXISTS security_policies (
    id          BIGSERIAL    PRIMARY KEY,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    deleted_at  TIMESTAMPTZ  NULL,
    name        TEXT         NOT NULL,
    description TEXT         NOT NULL DEFAULT '',
    policy_type TEXT         NOT NULL DEFAULT 'rbac',     -- rbac / network / quota / audit
    effect      TEXT         NOT NULL DEFAULT 'allow',    -- allow / deny
    resources   JSONB        NOT NULL DEFAULT '[]'::jsonb,
    actions     JSONB        NOT NULL DEFAULT '[]'::jsonb,
    conditions  JSONB        NOT NULL DEFAULT '{}'::jsonb,
    priority    INTEGER      NOT NULL DEFAULT 100,
    enabled     BOOLEAN      NOT NULL DEFAULT TRUE,
    tenant_id   BIGINT       NOT NULL DEFAULT 1,
    created_by  BIGINT       NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_security_policies_deleted_at ON security_policies (deleted_at);
CREATE INDEX IF NOT EXISTS idx_security_policies_type_enabled ON security_policies (policy_type, enabled);
CREATE INDEX IF NOT EXISTS idx_security_policies_tenant ON security_policies (tenant_id);
