# Metaclouds 数据库迁移管理文档

## 概述

本目录包含 Metaclouds 后端的所有数据库迁移文件，使用 [golang-migrate](https://github.com/golang-migrate/migrate) 工具进行管理。

迁移文件采用版本化命名规范，每个迁移包含 `up`（升级）和 `down`（回滚）两个文件。

## 迁移文件清单

| 版本 | 名称 | 说明 |
|------|------|------|
| 000001 | init_schema | 初始化数据库 Schema（12 张表，含 RBAC 表） |
| 000002 | add_indexes | 性能优化索引（13 个索引） |
| 000003 | seed_initial_data | 初始种子数据（租户、管理员、RBAC、系统配置） |

## 工具安装

### 安装 golang-migrate（PostgreSQL 支持）

```bash
go install -tags 'postgres' github.com/golang-migrate/migrate/v4/cmd/migrate@latest
```

安装后确保 `$GOPATH/bin` 在 PATH 中：

```bash
export PATH=$PATH:$(go env GOPATH)/bin
```

验证安装：

```bash
migrate -version
```

### 其他安装方式

**macOS（Homebrew）：**
```bash
brew install golang-migrate
```

**Linux（直接下载二进制）：**
```bash
curl -L https://github.com/golang-migrate/migrate/releases/download/v4.17.0/migrate.linux-amd64.tar.gz | tar xvz
sudo mv migrate /usr/local/bin/
```

## 常用命令

### 设置数据库连接

```bash
export DATABASE_URL="postgres://user:password@host:5432/metaclouds?sslmode=require"
```

或使用脚本封装（推荐）：

```bash
./scripts/run_migrations.sh up
```

### 执行所有迁移（升级到最新版本）

```bash
migrate -path ./migrations -database "$DATABASE_URL" up
```

### 回滚一个版本

```bash
migrate -path ./migrations -database "$DATABASE_URL" down 1
```

### 回滚到指定版本

```bash
migrate -path ./migrations -database "$DATABASE_URL" goto 000001
```

### 查看当前迁移版本

```bash
migrate -path ./migrations -database "$DATABASE_URL" version
```

### 强制修复版本（迁移失败后手动修复）

当迁移中途失败导致版本不一致时，先手动修复数据库状态，然后强制设置版本：

```bash
# 强制将版本设置为 000002（跳过失败的迁移）
migrate -path ./migrations -database "$DATABASE_URL" force 000002
```

**警告：** `force` 命令不会执行任何 SQL，仅修改 schema_migrations 表中的版本记录。使用前必须确保数据库实际状态与目标版本一致。

### 查看迁移历史（脏状态检测）

```bash
# 连接数据库查询
psql "$DATABASE_URL" -c "SELECT * FROM schema_migrations;"
```

## 新建迁移

### 使用 migrate create 命令

```bash
migrate create -ext sql -dir ./migrations -seq <migration_name>
```

示例：

```bash
migrate create -ext sql -dir ./migrations -seq add_user_preferences
```

这会生成两个文件：
- `000004_add_user_preferences.up.sql`
- `000004_add_user_preferences.down.sql`

### 命名规范

- 使用小写字母和下划线（snake_case）
- 名称应描述迁移的目的，如 `add_column_to_users`、`create_audit_log_table`
- 版本号由工具自动生成（6 位数字，零填充）

### 编写要求

1. **up 文件**：包含升级所需的所有 SQL 语句
2. **down 文件**：包含完整的回滚 SQL，必须能正确撤销 up 的所有变更
3. **幂等性**：使用 `IF NOT EXISTS`、`ON CONFLICT DO NOTHING` 等语法，确保迁移可重复执行
4. **中文注释**：所有 SQL 注释使用中文
5. **事务安全**：golang-migrate 默认在事务中执行迁移，避免使用不能在事务中运行的语句

## 注意事项

### CONCURRENTLY 索引

`CREATE INDEX CONCURRENTLY` 不能在事务块中运行，而 golang-migrate 默认将每个迁移文件包裹在事务中。如需创建并发索引：

**方案一：使用普通 CREATE INDEX（推荐，本项目采用）**
```sql
CREATE INDEX IF NOT EXISTS idx_name ON table_name (column);
```

**方案二：使用 --no-transaction 选项**
```bash
migrate -path ./migrations -database "$DATABASE_URL" --no-transaction up
```
注意：`--no-transaction` 会影响所有迁移，建议仅在包含 CONCURRENTLY 索引的迁移上单独使用。

**方案三：应用层手动执行**
对于零停机要求的生产环境，可在部署窗口外由 DBA 手动执行 CONCURRENTLY 索引创建。

### 零停机迁移原则

1. **扩展优先**：先添加新列/新表，再迁移数据，最后删除旧列
2. **向后兼容**：每个迁移版本都应保证旧版本应用仍能正常运行
3. **小步迭代**：避免大事务，将大规模数据迁移拆分为多个小迁移
4. **先加后删**：添加列用 `DEFAULT NULL`，删除列前先确保应用不再使用

### 生产环境部署流程

1. 在 staging 环境执行迁移并验证
2. 备份生产数据库
3. 在维护窗口执行迁移（或使用蓝绿部署）
4. 验证应用功能正常
5. 监控数据库性能（重点关注新索引的使用情况）

### 常见问题

**Q: 迁移失败后如何处理？**
A: 查看错误信息，手动修复数据库状态，然后使用 `migrate force <version>` 重置版本记录，再重新执行迁移。

**Q: 如何跳过某个有问题的迁移？**
A: 使用 `migrate force <目标版本>` 跳过，但必须确保数据库实际状态已手动调整到目标版本。

**Q: 迁移文件可以修改吗？**
A: **已执行的迁移文件绝对不能修改！** 如果需要变更，应创建新的迁移文件。仅未执行的迁移可以修改。

**Q: 如何处理开发环境和生产环境的差异？**
A: 迁移文件应保持环境无关。环境特定的配置（如密码、域名）应通过环境变量或应用配置管理，不应硬编码在迁移中。

## 相关文件

- 迁移脚本：`scripts/run_migrations.sh`（封装常用操作，含生产环境保护）
- 初始化脚本参考：`../init.sql`（原始初始化脚本，仅作参考，不再直接使用）
- 旧索引迁移：`../deploy/migrations/`（历史迁移文件，已标准化到本目录）
- CI 集成：`../../.github/workflows/ci-cd.yml`（backend-test job 中包含迁移验证步骤）
