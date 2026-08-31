# 外部系统凭据轮换 Runbook（git 清理 ≠ 撤销已部署暴露）

> **核心结论**：`git filter-branch` 只是把密钥从历史里抹掉。凡在此前**已经部署、运行、复制**的凭据，git 清理无能为力——旧口令在那些系统里仍然有效。下面每一步都**必须手动执行**，但本回合已把它们下沉为**可执行脚本**（`D:/YCYD/scripts/`），把"手工步骤"变成"跑一个脚本"。

---

## 0. 已自动完成的部分（前序回合）

| 项 | 状态 |
| --- | --- |
| 代码去硬编码（`Admin@123!`/`User@123!` 回退） | ✅ `bootstrapPassword()` 生产环境缺变量即拒绝启动 |
| 14 个凭据全量轮换（密码学强随机） | ✅ `D:/YCYD/.env` |
| compose 明文 → `${VAR:?required}` 强制注入 | ✅ 两个 `docker-compose.yml` |
| git 历史清除 `.env*`/二进制/`frontend/dist` | ✅ `git grep --all` 对泄露值零命中 |
| `init.sql` DB admin 哈希对齐到新口令 | ✅ `init.sql:145`（与 `DEFAULT_ADMIN_PASSWORD` 一致；新部署走此值） |
| frontend 登录页去掉泄露口令提示 + dist 脱敏 | ✅ |
| 删除磁盘临时 JWT（`token.tmp`/`backend_token.tmp`） | ✅ 已被 JWT_SECRET 轮换作废 |

---

## 1. 一键轮换（推荐路径）

脚本在 `D:/YCYD/scripts/`，与 `.env` / `docker-compose.yml` 同目录。需本地有 `docker compose` 且**整套服务已启动**。

```bash
cd D:/YCYD
# 非生产环境，交互确认后全自动：
./scripts/rotate_all.sh

# 生产环境需显式允许（脚本默认拒绝 production）：
ALLOW_PROD=1 ./scripts/rotate_all.sh
```

`rotate_all.sh` 内部依次：① PostgreSQL 角色 + 应用 admin 轮换 → ② `docker compose up -d --force-recreate redis grafana backend` 加载新 `.env` → ③ `verify_rotation.sh` 验证。

> 单步脚本（按需）：
> - `./scripts/rotate_db_credentials.sh` —— Postgres（见 §2）
> - `./scripts/rotate_grafana.sh` —— Grafana 零重启重置（见 §4）
> - `./scripts/rotate_redis.sh` —— Redis（见 §3）
> - `./scripts/verify_rotation.sh` —— 轮换后验证（见 §7）

---

## 2. PostgreSQL（关键陷阱）

**陷阱**：`POSTGRES_PASSWORD` 仅在建库**首启**（空卷）时由 Postgres 镜像应用。卷已存在时，改 `.env` **不会**改掉已有角色口令——必须 `ALTER USER`。

两套口令要分清：
- `POSTGRES_PASSWORD` → Postgres 超级用户 `metaclouds`（**应用连接 DB 用的**）
- `DEFAULT_ADMIN_PASSWORD` → 应用层 `admin` 账号（**人登录 Web 用的**）

`rotate_db_credentials.sh` 用 Postgres 自带 `pgcrypto.crypt()` 生成标准 bcrypt（`$2a$10$`，与 Go `bcrypt.DefaultCost` 兼容），**无需任何外部工具（Go/Python）**：

```sql
CREATE EXTENSION IF NOT EXISTS pgcrypto;
ALTER USER metaclouds WITH PASSWORD :'pw';                       -- POSTGRES_PASSWORD
UPDATE users SET password = crypt(:'pw', gen_salt('bf', 10))      -- DEFAULT_ADMIN_PASSWORD
  WHERE username = 'admin';
```
脚本已加 `SET log_statement='none'` 避免口令进 DB 日志，且幂等、不破坏数据。

---

## 3. Redis

compose 以 `--requirepass ${REDIS_PASSWORD}` 启动，`force-recreate` 即应用新值（**无需知道旧口令**）：

```bash
docker compose up -d --force-recreate redis        # 读新 .env
# 或运行实例就地轮换（需 OLD_REDIS_PASSWORD）：
OLD_REDIS_PASSWORD=旧值 ./scripts/rotate_redis.sh
```

---

## 4. Grafana

零重启（grafana-cli 直连 DB，不需旧口令）：

```bash
./scripts/rotate_grafana.sh
# 等价于：
docker compose exec grafana grafana-cli admin reset-admin-password '<新 GRAFANA_ADMIN_PASSWORD>'
```

---

## 5. JWT_SECRET

已在 `.env` 轮换。**`force-recreate backend` 后所有旧 JWT 立即失效**（用户需重新登录，属预期安全收益）。
检查是否还有**其它服务**（网关/反代/独立校验器）用旧密钥验令牌，一并改用新密钥。

---

## 6. CI / CD Secret 与运行容器

- 把 `D:/YCYD/.env` 全部值更新进 **GitHub Actions / GitLab CI / 其它流水线** Secret（`.env` 本身勿提交）。
- 旧容器环境变量/内存仍持旧口令直到重建：`docker compose up -d --force-recreate` 已覆盖。
- 全局清理旧值可能被复制的位置：运维笔记、聊天、其它仓库、备份脚本、监控配置、第三方集成。

---

## 7. 验证（轮换后必做，`verify_rotation.sh` 自动跑）

- backend health OK；
- redis 用**新**口令 `PONG`；
- 用**新** `DEFAULT_ADMIN_PASSWORD` 登录 → HTTP 200；
- 用**旧** `Admin@123!` 登录 → HTTP 401（证明旧口令已死）；
- 后端连 Redis/Postgres 无 auth 报错（`docker compose logs backend | grep -iE 'auth|password|connection'`）。

---

## 8. 回滚与备份目录（重要）

- 轮换**不破坏数据**，可逆：用**旧值**重跑对应脚本即可（旧值见 `D:/YCYD/.credential-rotation-backup-20260831-035733/env-originals/`）。
- 该备份目录含**明文新/旧凭据**，本身敏感。确认新凭据已在 Secret Manager 落地且验证通过后，**删除或加密迁移**该目录（回滚由 `repo-full.bundle` / `.git` 副本覆盖，无需保留明文凭据文件）。

---

## 9. 仍需注意的残留（非 git 仓库范围）

- `metaclouds-frontend` 此工作区是**非 git 纯检出副本**，`Login.tsx` 修复需在其真实仓库提交并重新 `docker compose build`。
- 若 `dist/` 被直接部署（不经 Docker 构建），已做字面量脱敏，但仍建议走一次 `docker compose build` 从修复后源码重建。
