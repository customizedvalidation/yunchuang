# Kubernetes 生产切换 Runbook（测试凭据 → 真实凭据 + HA）

- **日期**：2026-10-14
- **适用范围**：将测试集群的 TEST-ONLY 部署切换为生产运行，或在新生产集群首次落地。
- **配套文档**：测试数据落地剧本见 `docs/k8s-provision-with-test-data-2026-10-12.md`；本 runbook 取代该文档第六节"生产切换 checklist"的速查角色，给出可逐步执行的命令。
- **安全红线**：任何真实凭据**绝不**提交到 Git。仓库内 `02-secret.yaml` 的 `CHANGEME` 占位是生产基线，保持不变；真实值只进集群 Secret 或外部密钥系统。

---

## 0. 核心矛盾（必读，决定走哪条路径）

生产切换有两个已确认的坑，必须理解后再动手：

1. **kustomize 占位覆盖**：`13-kustomization.yaml` 的 `resources` 引用了 `02-secret.yaml`。**只要 `02-secret.yaml` 仍在 kustomization 内**，`kubectl apply -k` / `scripts/deploy-k8s.sh` 就会用 `CHANGEME` 占位把集群里的 `metaclouds-secrets` 覆盖回占位值（集群侧无门禁，apply 会"成功"）。
2. **env 不热更新**：后端通过 env 形式 `secretKeyRef` 注入 `DATABASE_URL` 等（非 volume 挂载）。Secret 变更后 K8s **不会**自动重启 Pod，后端仍持旧值，会因 `database-url` 是占位而 **fail-closed 退出**。必须在改完 Secret 后 `kubectl rollout restart deployment/metaclouds-backend`。

> CI 的 "Guard - reject placeholder secrets" 仅在 `apply` **之前**解析 `02-secret.yaml`，拒绝 `CHANGEME`；它**不能**保护"apply -k 把已注入的真实 Secret 覆盖回占位"这条运行期路径——那一步发生在集群侧，绕过了 CI。

由此引出两条落地路径：

| 路径 | 做法 | 是否让 kustomize 管理 Secret | 覆盖风险 | 推荐度 |
|------|------|------------------------------|----------|--------|
| **A（推荐）** | 把 `02-secret.yaml` 从 `13-kustomization.yaml` 的 `resources` 移除，真实 Secret 由外部系统/手动独立管控 | 否 | 彻底消除覆盖 | ⭐ 生产首选 |
| **B（最小改动）** | 保留 `02-secret.yaml` 在 kustomization；apply -k 后用真实值 upsert Secret，再重启后端 | 是 | 存在（已用回填兜底） | 临时/过渡 |

---

## 1. 前置条件

1. `kubectl` 已配置且 `kubectl config current-context` 指向**生产集群**。
2. 已克隆仓库，当前目录为仓库根。
3. 已决定走路径 A 或 B（见第 0 节）。
4. 若是**新生产集群首次落地**：先部署 PostgreSQL 数据面并等 Ready，再注入 Secret，最后整包 apply —— 顺序与测试剧本一致（见第 3/4 节步骤 0）。
5. 确认镜像已推送 GHCR（CI 在 push 到 `main` 时推送分支 tag；或更稳妥地用 `GIT_SHA` 钉不可变 tag）。

---

## 2. 生成真实凭据（不落 Git）

在跳板机/CI 上生成随机值，**仅用于本次 `kubectl create`**，不要写入任何文件提交：

```bash
# 一次性生成并直接创建 Secret（推荐，避免中间落盘）
# ⚠️ database-url 中的口令必须与 postgres-password 完全一致——
#    17-postgresql 的 POSTGRES_PASSWORD 取同一 secretKeyRef(postgres-password)，
#    后端 DATABASE_URL 取 database-url；两者口令不一致会导致后端连不上库。
kubectl -n metaclouds create secret generic metaclouds-secrets \
  --from-literal=jwt-secret="$(openssl rand -base64 48)" \
  --from-literal=database-url="postgresql://metaclouds_user:$(openssl rand -base64 24 | tr -d '/+=')@postgresql.metaclouds.svc.cluster.local:5432/metaclouds?sslmode=require" \
  --from-literal=redis-password="$(openssl rand -base64 24)" \
  --from-literal=default-admin-password="$(openssl rand -base64 18 | tr -d '/+=')" \
  --from-literal=postgres-password="$(openssl rand -base64 24 | tr -d '/+=')" \
  --from-literal=protected-endpoints-token="$(openssl rand -base64 32 | tr -d '/+=')" \
  --dry-run=client -o yaml | kubectl apply -f -
```

若希望先把值看一眼再创建（仍不落 Git，用环境变量暂存）：

```bash
export REAL_PG_PW="$(openssl rand -base64 24 | tr -d '/+=')"
export REAL_JWT="$(openssl rand -base64 48)"
export REAL_REDIS="$(openssl rand -base64 24)"
export REAL_ADMIN="$(openssl rand -base64 18 | tr -d '/+=')"
export REAL_TOKEN="$(openssl rand -base64 32 | tr -d '/+=')"
export REAL_DBURL="postgresql://metaclouds_user:${REAL_PG_PW}@postgresql.metaclouds.svc.cluster.local:5432/metaclouds?sslmode=require"

kubectl -n metaclouds create secret generic metaclouds-secrets \
  --from-literal=jwt-secret="${REAL_JWT}" \
  --from-literal=database-url="${REAL_DBURL}" \
  --from-literal=redis-password="${REAL_REDIS}" \
  --from-literal=default-admin-password="${REAL_ADMIN}" \
  --from-literal=postgres-password="${REAL_PG_PW}" \
  --from-literal=protected-endpoints-token="${REAL_TOKEN}" \
  --dry-run=client -o yaml | kubectl apply -f -

# 用完即清，避免凭据滞留在 shell 历史/环境
unset REAL_PG_PW REAL_JWT REAL_REDIS REAL_ADMIN REAL_TOKEN REAL_DBURL
```

> `database-url` 口令必须 == `postgres-password`。`base64 | tr -d '/+='` 去掉了 `/ + =`，剩余字符在 Postgres 连接串的密码段安全（不含 `@ : ?`）。

---

## 3. 路径 A（推荐）：kustomize 不再管理 Secret

### 步骤 0 — 部署 PostgreSQL 数据面并等 Ready（新集群首次落地才需要）

```bash
kubectl apply -f metaclouds-backend-rust/k8s/00-namespace.yaml
kubectl apply -f metaclouds-backend-rust/k8s/17-postgresql.yaml
kubectl rollout status statefulset/postgresql -n metaclouds --timeout=300s
kubectl wait -n metaclouds pod -l app=postgresql --for=condition=Ready --timeout=300s
```

### 步骤 1 — 把 `02-secret.yaml` 移出 kustomization（一次性配置变更）

编辑 `metaclouds-backend-rust/k8s/13-kustomization.yaml`，删除 `resources` 列表中的 `- 02-secret.yaml` 这一行。
这样 `kubectl apply -k` 永远不会触碰 `metaclouds-secrets`，彻底消除占位覆盖。

> 此改动应随代码提交（它不含任何凭据，只移除引用）。建议作为独立 commit。

### 步骤 2 — 创建真实 Secret（见第 2 节命令，已在集群侧）

### 步骤 3 — 钉版本部署整包（不含 Secret）

```bash
# 生产务必钉 GIT_SHA（不可变 sha tag），勿用默认 main 漂移 tag
GIT_SHA="$(git rev-parse HEAD)" ./scripts/deploy-k8s.sh
```

### 步骤 4 — 重启后端使 env 注入生效（幂等，建议每次切换后都跑）

```bash
kubectl -n metaclouds rollout restart deployment/metaclouds-backend
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s
```

### 外部密钥系统（生产推荐替代手动 create）

若用 External Secrets Operator / Sealed Secrets，把第 2 节的手动 `create` 替换为：在集群内部署对应的 `ExternalSecret` / `SealedSecret` CR（Git 仅存加密引用或外部引用），**同样把 `02-secret.yaml` 移出 kustomization**，避免两边打架。

---

## 4. 路径 B（最小改动）：保留占位但回填真实值

> 仅在过渡/临时场景使用；长期生产应迁移到路径 A。

```bash
# 0) 若是新集群，先部署 PG 并等 Ready（同第 3 节步骤 0）

# 1) 钉版本整包 apply —— 这一步会用 02-secret.yaml 的 CHANGEME 把 Secret 覆盖回占位（预期内）
GIT_SHA="$(git rev-parse HEAD)" ./scripts/deploy-k8s.sh

# 2) 立刻用真实值 upsert Secret（抵消上面的覆盖）
#    （命令同第 2 节；此处不再重复，粘贴第 2 节任一生成块即可）

# 3) 重启后端使 env 注入拾取真实值
kubectl -n metaclouds rollout restart deployment/metaclouds-backend
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s
```

> 路径 B 的"apply -k 覆盖 → 回填真实值 → 重启"三步顺序与测试剧本 `deploy-cluster.sh` 的 ④→④.5→④.7 完全一致，只是值换成真实的。

---

## 5. 镜像钉版本（生产强制）

- 永远用 `GIT_SHA=<commit-sha> ./scripts/deploy-k8s.sh` 钉不可变 tag。
- `scripts/deploy-k8s.sh` 内部对 `kubectl kustomize` 的输出做 `sed "s#:main#:${GIT_SHA}#g"`，把后端/前端两处镜像都从 `main` 钉到 sha。
- 禁止 `latest`；默认 `main` 仅用于非生产快速验证。

---

## 6. 验证（切换后必做）

```bash
# 工作负载健康
kubectl -n metaclouds get pods -l app=metaclouds
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s

# 后端 /health 返回 200（临时端口转发）
kubectl -n metaclouds port-forward svc/metaclouds-backend 18080:8000 &
curl -fsS http://127.0.0.1:18080/health ; kill %1

# Secret 确为真实值、无 CHANGEME（逐 key base64 解码核对）
for k in jwt-secret database-url redis-password default-admin-password postgres-password protected-endpoints-token; do
  v=$(kubectl -n metaclouds get secret metaclouds-secrets -o jsonpath="{.data.$k}" | base64 -d)
  echo "$k = $v"
  case "$v" in *CHANGEME*) echo "  !! 仍含占位 CHANGEME，切换失败";; esac
done

# 后端日志不应出现 fail-closed（缺真实 database-url 才会）
kubectl -n metaclouds logs deployment/metaclouds-backend --tail=50 | grep -i "fail-closed\|database-url" || echo "no fail-closed signals"
```

预期：`metaclouds-backend` 3 副本 `Running`/`READY`，`/health` 200，6 个 key 全部为真实值且无 `CHANGEME`。

---

## 7. 回滚

```bash
# 回滚镜像到上一已知良好 sha（最快止血）
GIT_SHA="<prev-good-sha>" ./scripts/deploy-k8s.sh

# 回滚整包工作负载（路径 A 下 Secret 不在 kustomization，不会被删；路径 B 下会被删，见下）
kubectl delete -k metaclouds-backend-rust/k8s
```

- **路径 A**：Secret 独立管控，`delete -k` 不会删它；如需重置凭据再跑第 2 节创建命令即可。
- **路径 B**：`delete -k` 会连 Secret 一起删；回滚后需重新创建真实 Secret 并重启后端。
- PostgreSQL 的 PVC（`volumeClaimTemplates` 生成的 `data-postgresql-0`）`delete -k` 默认保留；要彻底清数据需 `kubectl -n metaclouds delete pvc data-postgresql-0`。

---

## 8. PostgreSQL 高可用（生产必做）

- **现状**：`17-postgresql.yaml` 为**单副本** StatefulSet，无流复制/HA。单点故障即全站数据面不可用。
- **推荐方案**（任选其一）：
  1. **CloudNativePG（cnpg）** —— 社区主流，原生支持副本/自动故障转移/备份；
  2. **Zalando Postgres Operator（Spilo）** —— 老牌稳定；
  3. **云厂商托管**（RDS / CloudSQL / Aurora）—— 运维成本最低。
- 切换后需同步更新：
  - `database-url` 连接串改为 HA 端点（或 `host1,host2` 多主机、含 `target_session_attrs=primary` 等）；
  - 若改用 Operator，通常**不再**用 `17-postgresql.yaml` 原生 StatefulSet，改为 Operator 的 CR（记得把旧的 `17-postgresql.yaml` 从 kustomization 移除）；
  - 重新钉版本部署并重启后端。

---

## 9. 安全与合规检查清单

- [ ] 仓库内 `02-secret.yaml` 仍全是 `CHANGEME` 占位（`git grep CHANGEME metaclouds-backend-rust/k8s/02-secret.yaml` 有命中即正确）。
- [ ] 真实 Secret 未落 Git（任何 `base64 -d` 后含真实口令的文件都不在版本控制）。
- [ ] 本次部署通过了 CI 的 "Guard - reject placeholder secrets"（若 CI 跑过即说明 `02-secret.yaml` 无 `CHANGEME`，但运行时覆盖需靠本 runbook 的路径 A 根除）。
- [ ] `metaclouds-secrets` 6 个 key 齐全且为真实值（第 6 节逐 key 核对）。
- [ ] `database-url` 口令 == `postgres-password`（否则后端连不上库）。
- [ ] 镜像为 `GIT_SHA` 不可变 tag，非 `main`/`latest`。
- [ ] 凭据轮换策略：`02-secret.yaml` 标注 `rotation-policy: 90d`，记录生产凭据的轮换步骤（重新生成 → upsert → 重启后端）。

---

## 10. 交付文件

| 文件 | 说明 |
| --- | --- |
| `docs/k8s-production-switch-runbook.md` | 本 runbook（生产切换可执行步骤） |
| `metaclouds-backend-rust/k8s/13-kustomization.yaml` | 路径 A 需删除其中 `- 02-secret.yaml` 一行（生产基线配置变更，无凭据） |
| `scripts/deploy-k8s.sh` | 钉版本部署入口（`GIT_SHA=...`） |
| `docs/k8s-provision-with-test-data-2026-10-12.md` | 测试数据落地剧本（本 runbook 取代其第六节速查角色） |
