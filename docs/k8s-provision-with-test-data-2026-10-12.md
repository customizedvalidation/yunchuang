# Kubernetes 运行期前置落地剧本（测试数据版）

- **日期**：2026-10-12
- **适用范围**：测试集群（⚠️ 仅测试，生产请走"生产切换 checklist"）
- **背景**：后端已从 SQLite 单副本翻转至 PostgreSQL 多副本（commit `5614e3b`）。但 `kubectl apply` 整包前，有两件**运行期前置**（即 05-deployment.yaml 文件头"前置"段）：
  1. 先在集群内部署 `metaclouds-backend-rust/k8s/17-postgresql.yaml`（PostgreSQL StatefulSet + Service）；
  2. 先把 `02-secret.yaml` 里的 `CHANGEME` 占位替换为**真实**敏感值，并通过 `metaclouds-secrets` Secret 注入（CI 门禁拒绝 `CHANGEME`；后端在 PG 模式下若缺真实 `database-url` 会 fail-closed 退出）。
- 本剧本用**测试数据**在目标集群执行这两项前置，所有命令可直接复制运行（目标集群在用户侧，已配置 `kubectl`）。

> ⚠️ **安全声明**：本剧本注入的凭据均为明显标注的测试值（统一带 `-change-me` / `-test-` 后缀），仅用于测试集群。生产部署必须替换为真实凭据，切勿把真实凭据写进任何 git 跟踪的 YAML（`02-secret.yaml` 的 `CHANGEME` 占位已保留为生产基线，未改动）。

---

## 一、前置条件

1. 本地/跳板机已安装 `kubectl`，且 `kubectl config current-context` 指向**目标测试集群**。
2. 已克隆本仓库，且当前目录为仓库根（`D:/YCYD` 或等价路径）。
3. 目标集群具备创建 `StatefulSet` / `PVC` 的权限（默认 `storageClass` 支持 `ReadWriteOnce`）。
   - 命名空间 `metaclouds` 若不存在，脚本会自动 `apply 00-namespace.yaml` 创建（见步骤 0）。
4. 不修改 `02-secret.yaml` 的 `CHANGEME` 占位；测试凭据只通过脚本注入。

## 二、测试数据说明（secret key 集合）

`scripts/provision-test-secret.sh` 注入 `metaclouds-secrets`，覆盖 `02-secret.yaml` 声明的全部 6 个 key：

| Key | 测试值（节选） | 说明 / 对齐来源 |
| --- | --- | --- |
| `jwt-secret` | `test-jwt-secret-change-me-...` | JWT 签名密钥（≥32 字节） |
| `database-url` | `postgresql://metaclouds_user:test-postgres-password-change-me@postgresql.metaclouds.svc.cluster.local:5432/metaclouds?sslmode=require` | 与 17-postgresql 的 Service/库名/端口精确对齐 |
| `redis-password` | `test-redis-password-change-me` | 18-redis 的 `REDIS_PASSWORD` |
| `default-admin-password` | `test-default-admin-password-change-me` | 首次播种管理员口令 |
| `postgres-password` | `test-postgres-password-change-me` | 17-postgresql 的 `POSTGRES_PASSWORD`（与 database-url 口令一致） |
| `protected-endpoints-token` | `test-protected-endpoints-token-change-me` | 生产访问 `/metrics`、`/swagger-ui` 的 Bearer |

`database-url` 各段来源（**务必精确对齐**）：
- 用户 `metaclouds_user` ← `01-configmap.yaml` 的 `DATABASE_USER` / `17-postgresql.yaml` 的 `POSTGRES_USER`（`pg_isready -U metaclouds_user`）
- 主机 `postgresql.metaclouds.svc.cluster.local` ← `17-postgresql.yaml` Service 名 `postgresql` + 命名空间 `metaclouds`（与 `01-configmap` 的 `DATABASE_HOST` 一致）
- 端口 `5432`、库名 `metaclouds`、 `sslmode=require` ← `01-configmap` 的 `DATABASE_PORT/NAME/SSL_MODE`
- 口令复用 `postgres-password`，保证 PostgreSQL 实际口令 == `database-url` 中的口令

## 三、逐步命令（在目标集群执行）

> 以下步骤 ①~⑤ 已被编排进 `scripts/deploy-cluster.sh`，可直接跑脚本；下面也给出手动等价命令。

### 方式 A：一键编排（推荐）

```bash
# 预览（不真正变更，kubectl --dry-run=client 校验语法）
./scripts/deploy-cluster.sh --dry-run

# 真正执行（须 kubectl 已配置目标集群）
./scripts/deploy-cluster.sh
```

### 方式 B：手动逐步

```bash
# 0) 命名空间
kubectl apply -f metaclouds-backend-rust/k8s/00-namespace.yaml

# 1) 运行期前置之一：先部署 PostgreSQL
kubectl apply -f metaclouds-backend-rust/k8s/17-postgresql.yaml

# 2) 等 PostgreSQL Pod Ready（否则后端连不上库会 fail-closed）
kubectl rollout status statefulset/postgresql -n metaclouds --timeout=300s
kubectl wait -n metaclouds pod -l app=postgresql --for=condition=Ready --timeout=300s

# 3) 运行期前置之二：注入测试 secret（幂等 upsert）
./scripts/provision-test-secret.sh --apply

# 4) 整包部署（后端 3 副本 + Redis + 前端；可用 GIT_SHA 钉版本）
./scripts/deploy-k8s.sh
# 或：kubectl apply -k metaclouds-backend-rust/k8s

# 4.5) ⚠️ 回填测试 secret：步骤 4 的 apply -k 会按 02-secret.yaml 的 CHANGEME 占位
#      把 metaclouds-secrets 覆盖回占位值（集群侧无 CI 门禁，apply 会成功），
#      必须再次 upsert 测试值，确保后端最终拿到真实 database-url。
#      回填后需重启后端 Deployment（kubectl -n metaclouds rollout restart
#      deployment/metaclouds-backend）才能使 env 注入的 DATABASE_URL 生效，
#      否则后端仍持占位值、/health 验证失败（deploy-cluster.sh 已内含该重启）。
./scripts/provision-test-secret.sh --apply

# 5) 验证
kubectl -n metaclouds get pods -l app=metaclouds
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s
# 后端健康探测（临时端口转发）
kubectl -n metaclouds port-forward svc/metaclouds-backend 18080:8000 &
curl -fsS http://127.0.0.1:18080/health ; kill %1
```

## 四、预期结果

- `kubectl -n metaclouds get pods -l app=metaclouds` 中：
  - `postgresql-0` 为 `Running` 且 `READY 1/1`（startupProbe 用 `pg_isready`）
  - `redis-0` 为 `Running`
  - `metaclouds-backend` 3 副本 `Running`/`READY`，无 `CrashLoopBackOff`（缺少真实 `database-url` 才会 fail-closed）
- `metaclouds-backend` 的 `/health` 返回 `200`。
- `kubectl -n metaclouds get secret metaclouds-secrets -o jsonpath='{.data}'` 含全部 6 个 key。

## 五、回滚步骤

```bash
# 1) 删除整包工作负载（含后端/前端/PG/Redis）
kubectl delete -k metaclouds-backend-rust/k8s

# 2) 删除测试 secret（便于下次重新注入）
./scripts/provision-test-secret.sh --delete
# 或：kubectl -n metaclouds delete secret metaclouds-secrets --ignore-not-found
```

> 注意：`delete -k` 会按 kustomization 删除所有资源；`17-postgresql` 的 PVC（`volumeClaimTemplates` 生成的 `data-postgresql-0`）默认保留，需手动 `kubectl -n metaclouds delete pvc data-postgresql-0` 才能彻底清掉数据。

## 六、生产切换 checklist（切勿直接套用测试凭据）

> 详细、可逐步执行的命令见 **`docs/k8s-production-switch-runbook.md`**（含 kustomize 占位覆盖 + env 不热更新两大坑、路径 A/B 决策、PG HA）。本节保留为速查。

1. **替换真实凭据**：不要使用 `--apply` 的测试值。生产推荐以下任一方式，且**绝不**把真实值提交到 Git：
   - External Secrets Operator / Sealed Secrets（Git 仅存加密引用）；
   - 或按 `02-secret.yaml` 注释用 `openssl rand -base64` 生成随机值后手动 `kubectl create secret generic metaclouds-secrets --from-literal=...`；
   - `database-url` 中的口令必须与 `postgres-password` 一致，且主机/库名/端口与 `17-postgresql.yaml` 对齐。
2. **PG 高可用**：当前 `17-postgresql.yaml` 为单副本 StatefulSet，未做流复制/HA。生产请换 **CloudNativePG** 或 **Zalando Postgres Operator**，或云厂商托管实例，并同步更新 `DATABASE_URL` 的连接串（含 HA 端点/多个 host）。
3. **Redis 口令**：`redis-password` 生产须随机且保密（`18-redis.yaml` 用 `--requirepass` 注入）。
4. **CI 门禁**：`.github/workflows/ci-cd.yml` 的 "Guard - reject placeholder secrets" 会在 apply 前解析 `02-secret.yaml`，任何含 `CHANGEME` 的值都会让部署失败——请确保使用真实值而非占位。
5. **镜像钉版本**：生产用 `GIT_SHA=<commit-sha> ./scripts/deploy-k8s.sh` 钉不可变 tag，勿用默认 `main` 漂移 tag。
6. **命名空间环境标签**：`00-namespace.yaml` 的 `environment: production` 标签保留；测试集群可临时改 `test`（仅标签，不影响功能）。

## 七、交付文件

| 文件 | 说明 |
| --- | --- |
| `scripts/provision-test-secret.sh` | 用测试值注入 `metaclouds-secrets`（覆盖 `02-secret.yaml` 全部 6 个 key），支持 `--dry-run/--apply/--delete/--check-keys` |
| `scripts/deploy-cluster.sh` | 编排"PG 先部署 → 等 Ready → 注入测试 secret → 整包 apply → 验证"的正确顺序，支持 `--dry-run` |
| `docs/k8s-provision-with-test-data-2026-10-12.md` | 本执行剧本 |

> 未修改：`02-secret.yaml`（CHANGEME 占位保留）、`deploy-k8s.sh`（仅被调用，未改）、Rust 代码、docker-compose、CI 实质配置。
