-- Initial schema for metaclouds-backend-rust (PostgreSQL variant).
-- Semantically equivalent to ../001_initial.sql (SQLite).
-- Aligned with the Go v1 contract: users table with username/email UNIQUE.
--
-- 方言映射：
--   INTEGER PRIMARY KEY AUTOINCREMENT -> BIGSERIAL PRIMARY KEY（对齐 Go GORM uint64 ID）
--   TEXT（时间戳）                    -> TIMESTAMPTZ（Rust 侧为 chrono::DateTime<Utc>）
--   TEXT（字符串）                    -> TEXT 保持
--   INTEGER（普通整数）                -> BIGINT（Rust 侧 i64）

CREATE TABLE IF NOT EXISTS users (
    id            BIGSERIAL    PRIMARY KEY,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    username      TEXT         NOT NULL UNIQUE,
    email         TEXT         NOT NULL UNIQUE,
    password_hash TEXT         NOT NULL,
    role          TEXT         NOT NULL DEFAULT 'user',
    tenant_id     BIGINT       NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_users_username ON users (username);
CREATE INDEX IF NOT EXISTS idx_users_email    ON users (email);
