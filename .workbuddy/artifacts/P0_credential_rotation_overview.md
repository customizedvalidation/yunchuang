# P0 凭据轮换 + 清理 git 历史 — 完成报告

> 仓库：`D:\YCYD\metaclouds-backend`（无远程，2 个提交，本地历史泄露）
> 处理时间：2026-08-30 / 31

## 一、做了什么

| 步骤 | 动作 | 结果 |
|------|------|------|
| 0. 备份（可逆） | 复制整个 `.git`、导出 `repo-full.bundle`、备份 5 个 env 原件、生成 `NEW_CREDENTIALS.md` | 见 `D:\YCYD\.credential-rotation-backup-20260831-035733/` |
| 1. 代码去硬编码 | 新增 `models/bootstrap_credentials.go`：`bootstrapPassword()` 在生产环境缺失变量时**拒绝启动**，不再回退到 `Admin@123!`/`User@123!` | `db.go`/`init_data.go` 已接入 |
| 2. 凭据轮换 | `scripts/rotate_credentials.py` 用 `secrets` 生成密码学强值，覆盖 `.env`/`.env.development`/`.env.staging`/`.env.production`/`docker/.env.production` | 全部 ≥24 字符，且与原始值不同 |
| 3. compose 强制注入 | 根与 `metaclouds-backend/docker-compose.yml` 明文口令改为 `${VAR:?required}` | 杜绝弱口令兜底 |
| 4. 清理脚本/文档/测试 | 测试与脚本改为注入已知测试口令，移除对 `Admin@123!` 的依赖；`.example` 模板用占位符 | 无泄露明文残留 |
| 5. 加固 .gitignore | 忽略真实 `.env*`（保留 `.example`）、`metaclouds-backend` 二进制、`frontend/dist` | 取消跟踪上述密钥/构建产物 |
| 6. 重写历史 | `git filter-branch --index-filter` 清除全部提交中的 `.env*`、`metaclouds-backend.exe`、`*.exe~`、`frontend/dist` | 随后 `refs/original` 删除 + `reflog expire` + `gc --prune=now` |
| 7. 收尾验证 | `go build`/`go vet`/`go test ./...` 全绿；修复 `TestClusterService_GetClusters` 脆弱顺序断言 | 通过 |
| 8. pre-commit 钩子 | 新增「阶段 0 密钥扫描」：阻断 `.env*`（非 `.example`）/私钥/二进制，扫描 JWT/AWS/GitHub/Slack/私钥格式 | 已实测可拦截 `.env` 提交 |

## 二、关键验证（均已通过）

- `git ls-files` 仅剩 `.env.development.example`、`.env.production.example`（模板），**无任何真实密钥文件**。
- `git log --all -- <secret paths>` —— **空**，历史中无任何密钥路径。
- `git grep --all 'MetacloudsSecure2026|User@123!'` —— **零命中**，泄露凭据不可从任何可达 blob 恢复。
- 仓库体积 **27 MiB → 279 KiB**（32MB 二进制与 `frontend/dist` 已剔除）。
- `go vet ./...` 与 `go test ./...` 全部 `ok`。

## 三、备份与回滚

备份目录：`D:\YCYD\.credential-rotation-backup-20260831-035733/`
- `git-dir-backup/`：完整 `.git` 副本
- `repo-full.bundle`：可 `git clone` 还原
- `env-originals/`：5 个 env 文件原始内容
- `NEW_CREDENTIALS.md`：已轮换明文凭据
- `HOW_TO_ROLLBACK.md`：三种回滚方式

> 历史重写是不可逆操作；如回滚，优先用 bundle 或 `.git` 副本。

## 四、仍须用户处理（git 清理不能替代）

1. **外部系统真正轮换并失效旧凭据**：PostgreSQL、Redis、Grafana、JWT 消费方、已部署实例、CI Secret。旧凭据一旦部署过即已暴露，仅清理 git 不撤销暴露。
2. **`metaclouds-frontend` 仓库源码**仍把 `admin / Admin@123!` 作为登录页默认账号提示（独立仓库，本次未触及）。
3. 删除磁盘临时令牌 `token.tmp` / `backend_token.tmp`。
4. CPU/内存指标为硬编码模拟值（内存仿真后端设计使然），生产建议接入 node_exporter。

## 五、最终状态

- 本地 git 历史已无明文凭据，仅保留 `.example` 模板。
- 全部 P0 代码修复 + 凭据处理已提交（HEAD `ecaaede`）。
- 后续提交由 pre-commit 钩子自动拦截密钥，防止再次泄露。
