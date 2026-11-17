# PostgreSQL 备份与恢复演练手册

> 状态：**流程文档，尚未在真实环境演练**。离线开发沙箱无 `pg_dump` / `psql` / kubectl，
> 因此 `scripts/backup-postgres.sh`、`scripts/restore-postgres.sh` 与
> `k8s/19-postgresql-backup.yaml` 均**未实测**。首次演练必须在预发环境完成并记录结果，
> 在拿到通过记录前不得宣称"备份可恢复"。

---

## 1. 目标与边界

| 项 | 说明 |
| --- | --- |
| 目标 | 验证备份产物**可恢复**、恢复后业务数据一致、RTO/RPO 满足约定 |
| 备份能力 | `k8s/19-postgresql-backup.yaml`（每日 02:00 UTC CronJob）+ 两个本地脚本 |
| **不是 HA** | 当前 PG 仍是单副本 StatefulSet（`17-postgresql.yaml`）；备份不消除单点故障 |
| **不是已验证** | 所有命令需在真实 PG 环境执行；本文件只是可执行步骤与验收标准 |

建议目标（需与业务方确认后写入 SLA）：RPO ≤ 24h（当前每日全量），RTO ≤ 60min（单库恢复）。

---

## 2. 前置条件

- [ ] 目标环境已 apply `k8s/19-postgresql-backup.yaml`，PVC `postgresql-backups` 已绑定
- [ ] 已安装 `postgresql-client`（含 `pg_dump` / `pg_restore` / `psql`）
- [ ] 连接凭据通过 `DATABASE_URL` 或 `~/.pgpass` 提供，**不写入命令行**
- [ ] 已准备隔离的预发库（**严禁在生产的唯一库上直接演练恢复**）
- [ ] 已记录演练窗口与回滚负责人

---

## 3. 场景 A：验证自动备份产物

```bash
# 1) 触发一次即时备份（CronJob 的 Job 模板，或直接用脚本）。
DATABASE_URL='postgres://user@host:5432/metaclouds' \
BACKUP_DIR=/var/backups/metaclouds \
bash scripts/backup-postgres.sh

# 2) 校验归档可读（脚本已自动执行，此处独立复核）。
ls -lh /var/backups/metaclouds/metaclouds-*.dump | tail -3
pg_restore -l /var/backups/metaclouds/metaclouds-<timestamp>.dump | head -20

# 3) 从 K8s 侧确认 CronJob 已产生成功 Job。
kubectl -n metaclouds get cronjob postgresql-backup
kubectl -n metaclouds get jobs -l job-name=postgresql-backup --sort-by=.metadata.creationTimestamp
```

**通过标准**：脚本输出 `verified archive is readable`；`pg_restore -l` 能列出对象；CronJob 最近一次 Job `COMPLETED`。

---

## 4. 场景 B：恢复到隔离预发库

```bash
# 1) dry-run：只展示归档内容与目标库，不写入。
DATABASE_URL='postgres://user@staging-host:5432/metaclouds_drill' \
bash scripts/restore-postgres.sh \
  --file /var/backups/metaclouds/metaclouds-<timestamp>.dump

# 2) 核对输出中的目标库确实是隔离预发库，再加 --confirm 真正恢复。
DATABASE_URL='postgres://user@staging-host:5432/metaclouds_drill' \
bash scripts/restore-postgres.sh \
  --file /var/backups/metaclouds/metaclouds-<timestamp>.dump --confirm
```

脚本行为：恢复前自动对目标库做一次快照（`metaclouds-pre-restore-*.dump`），再执行
`pg_restore --clean --if-exists --no-owner --no-privileges`。

**通过标准**：命令 exit 0；预发库表数量与源库一致；抽样业务查询返回预期行数。

---

## 5. 场景 C：应用侧回归（恢复后）

在**恢复后的隔离库**上启动后端（指向该库），执行最小回归：

```bash
cd metaclouds-backend-rust
# 用一个隔离端口与指向演练库的连接串，避免误连生产。
SERVER_PORT=18001 DATABASE_URL='postgres://user@staging-host:5432/metaclouds_drill' \
USE_SQLITE=false cargo run --release
```

验证点：
- `GET /health` 返回 200；
- 使用演练库已有账号登录成功（口令来自该环境的密钥管理，不写入文档）；
- 作业列表可查询，且 `usage_source='simulated'` 的行未出现在计费口径中。

---

## 6. 验收记录模板（演练后填写）

| 项 | 记录 |
| --- | --- |
| 演练日期 / 执行人 | ________ |
| 源库 / 目标库 | ________ |
| 备份归档文件与大小 | ________ |
| `pg_restore -l` 是否通过 | ☐ 是 ☐ 否 |
| 恢复耗时 / RTO 实测 | ________ |
| 数据一致性抽查结果 | ________ |
| 应用回归是否通过 | ☐ 是 ☐ 否 |
| 遗留问题与整改项 | ________ |

---

## 7. 频率与失败处理

- **频率**：首次上线前必须演练一次；此后每季度一次，或数据库大版本升级后。
- **失败处理**：任一验收项失败即视为"备份不可用"，需修复后重演；期间对外口径保持
  "备份能力已部署、恢复未验证"，不得宣称具备灾备能力。
- **不要做的事**：不要在生产唯一库上直接恢复；不要删除 pre-restore 快照直到验收通过；
  不要把凭据写进命令行历史或文档。
