-- WP-P2-B1：基础域 Auth/Tenant 补齐。
--
-- 1) users 表新增 last_login_at 列（B1 要求记录最近成功登录时间；
--    Go 版未持久化该列，此处为 Rust 侧增强，可空，登录成功时由服务层写入）。
-- 2) 默认租户 / 管理员账户的播种为运行时幂等逻辑（见 src/db.rs 的
--    seed_admin_if_empty），不在迁移文件里写死 argon2 哈希（哈希含随机盐，
--    不便以 SQL 字面量固化）。
--
-- 方言：SQLite。ALTER TABLE ADD COLUMN 在全新库上按 001→002→003 顺序执行。

ALTER TABLE users ADD COLUMN last_login_at TEXT NULL;
CREATE INDEX IF NOT EXISTS idx_users_last_login_at ON users (last_login_at);
