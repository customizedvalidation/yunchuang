# Metaclouds（yunchuang）生产环境部署与上线指南

> **版本**：v1.0　**日期**：2026-09-20　**基线 commit**：`e86adb7`
> **适用范围**：目标生产/预发环境部署人员与运维
> **性质**：本指南汇总全部"待目标环境项"的落地操作，本机无法完成项均在此给出可执行步骤、命令与验收标准。

---

## 0. 总览：五项待办与执行顺序

| 代号 | 待办项 | 状态 | 前置依赖 | 预计耗时 |
|---|---|---|---|---|
| **A** | 部署 Secrets 配置（三环境 SSH 自动部署） | 🔧 待目标环境 | 目标服务器可达 | 30 分钟 |
| **B** | Docker 镜像发布（ghcr push） | 🔧 待目标环境 | GitHub 仓库 packages 写权限 | 15 分钟 |
| **C** | Go 后端退役（影子观察 → 灰度切流 → 退役） | 🔧 待目标环境 | A、B 完成后 | 2 周 |
| **D** | K8s 真实部署（14 清单） | 🔧 待目标环境 | 集群 + Ingress + Prometheus Operator | 半天 |
| **E** | 生产 PostgreSQL 实跑 | 🔧 待目标环境 | PG 实例 | 2 小时 |

**推荐执行路径**：

```text
E（先有数据库）→ D（K8s 部署）→ A（SSH 部署激活）→ B（镜像发布）
   → C1 前置验收 → C2 影子观察(≥1 周) → C3 灰度切流(10%→50%→100%)
   → C4 稳定观察(1 周) → C5 退役清单 → C6 退役后观察
```

> 也可先做 A（SSH 裸机部署）快速打通业务，再逐步补齐 D/E/B，最后执行 C 的完整退役流程。
> **铁律**：C 的每一档放量必须观察达标后再推进；任一档触发回滚条件立即按 Runbook 回滚，不强行推进。

---

## A. 部署 Secrets 配置（配置后三环境 SSH 部署自动启用）

### A.1 Secrets 清单

在 GitHub 仓库 → **Settings → Secrets and variables → Actions** 配置以下 secrets：

| Secret | 用于 | 说明 |
|---|---|---|
| `DEV_SSH_KEY` | Deploy to Development | 开发服务器 SSH 私钥（ed25519） |
| `DEV_HOST` | Deploy to Development | 开发服务器地址（IP 或域名） |
| `DEV_USER` | Deploy to Development | 登录用户（需部署目录写权限） |
| `STAGING_SSH_KEY` | Deploy to Staging | 预发服务器 SSH 私钥 |
| `STAGING_HOST` | Deploy to Staging | 预发服务器地址 |
| `STAGING_USER` | Deploy to Staging | 登录用户 |
| `PROD_SSH_KEY` | Deploy to Production | 生产服务器 SSH 私钥 |
| `PROD_HOST` | Deploy to Production | 生产服务器地址 |
| `PROD_USER` | Deploy to Production | 登录用户 |
| `DEPLOY_DIR` | 全部部署 job（可选） | 服务器仓库目录，**默认 `/opt/metaclouds`** |

### A.2 服务器前置条件（每个环境执行一次）

```bash
# 1. 安装 Go（CI 部署命令会在服务器上执行 ./deploy.sh 本机构建）
#    https://go.dev/dl/ 安装 1.24+，确认 go version 可用

# 2. 克隆仓库到部署目录（默认 /opt/metaclouds）
sudo mkdir -p /opt/metaclouds && sudo chown $USER /opt/metaclouds
cd /opt/metaclouds && git clone https://github.com/customizedvalidation/yunchuang.git .

# 3. 准备后端环境配置文件（deploy.sh 启动前必读）
cd /opt/metaclouds/metaclouds-backend
cp .env.example .env.staging       # 按环境改为 .env.development / .env.production
#    必改：JWT_SECRET（≥32 字符）、DATABASE_URL、ALLOWED_ORIGINS

# 4. 生成部署密钥对，公钥加入 authorized_keys
ssh-keygen -t ed25519 -f ~/.ssh/metaclouds_deploy
cat ~/.ssh/metaclouds_deploy.pub >> ~/.ssh/authorized_keys
#    ↓ 私钥完整内容（含 -----BEGIN/END----- 两行）粘贴到对应 *_SSH_KEY secret
cat ~/.ssh/metaclouds_deploy
```

### A.3 触发方式

| 环境 | 自动触发 | 手动触发 |
|---|---|---|
| Development | push 到 `develop` 分支 | workflow_dispatch + environment=development |
| Staging | push 到 `main` 分支 | workflow_dispatch + environment=staging |
| Production | 不自动触发（安全） | workflow_dispatch + environment=production（经 approval gate） |

### A.4 部署执行逻辑（CI 内）

CI 中每个部署 job 的 SSH 步骤仅在对应 `*_SSH_KEY` 与 `*_HOST` **均已配置**时执行：

```bash
echo "${{ secrets.XXX_SSH_KEY }}" > /tmp/deploy_key
chmod 600 /tmp/deploy_key
ssh -i /tmp/deploy_key -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
  "${{ secrets.XXX_USER }}@${{ secrets.XXX_HOST }}" \
  "cd $DEPLOY_DIR && git pull --ff-only && cd metaclouds-backend && ./deploy.sh -e <env> -l <LEVEL> -k"
rm -f /tmp/deploy_key
```

未配置 secret 时步骤自动跳过，job 保持绿色，不影响主 CI 链路。

### A.5 验证启用

```text
1. 配置完 secrets 后，手动触发：Actions → Metaclouds CI/CD Pipeline → Run workflow
   → environment = staging
2. 观察 "Deploy to Staging (SSH)" 步骤：
   - 成功：日志显示 ssh 连接、git pull、deploy.sh 输出（构建 + 启动）
   - 跳过：日志显示 "Skipping"（说明 secrets 未生效，检查 secret 名拼写）
3. 登录验证：curl http://<staging-host>:8000/health 返回 200
```

### A.6 生产环境保护（强烈建议）

GitHub → **Settings → Environments → `production`**：
1. 添加 **Required reviewers**（如 2 人审批）
2. 勾选 **Wait timer**（如 5 分钟冷却期）

> 配置后 workflow_dispatch production 会在执行 `Deploy to Production (SSH)` 前等待人工审批。

---

## B. Docker 镜像发布（ghcr push）

### B.1 现状

- CI `docker-build-backend` / `docker-build-frontend` 两个 job：**构建 + Trivy 扫描（HIGH/CRITICAL）+ SARIF 上传**，但 `push: false`（`.github/workflows/ci-cd.yml` 第 424、485 行）。
- 镜像仅 CI 内构建验证，**无发布通道**；部署侧走 SSH `git pull` + 本机构建，不依赖镜像仓库。

### B.2 前置：ghcr 推送权限

GitHub 仓库 **Settings → Actions → General → Workflow permissions**：
- 勾选 **Read and write permissions**（允许 GITHUB_TOKEN 写 packages）；
- 或改用 PAT：GitHub → Settings → Developer settings → Personal access tokens → 勾选 `write:packages` → 存入 `GHCR_PAT` secret，并改 CI 登录用户。

### B.3 启用 push

修改 `.github/workflows/ci-cd.yml` 两处（后端/前端各一）：

```yaml
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: metaclouds-backend          # 后端；前端为 metaclouds-frontend-vue
          file: metaclouds-backend/Dockerfile
          push: true                           # ← false 改为 true
          tags: ${{ steps.meta-backend.outputs.tags }}
          ...
```

同时将 docker/login-action 的 username/password 改用有权限的凭据：

```yaml
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GHCR_PAT || secrets.GITHUB_TOKEN }}
```

### B.4 验证发布

```bash
# push 成功后（新 run 绿）：
docker pull ghcr.io/customizedvalidation/yunchuang/backend-rust:<git-sha>
docker pull ghcr.io/customizedvalidation/yunchuang/frontend:<git-sha>
docker run --rm ghcr.io/customizedvalidation/yunchuang/backend-rust:<sha> --version
```

> **镜像规范**：K8s 清单要求镜像 tag 用 `<git-sha>`，**禁止 latest**（见 D.5）。

---

## C. Go 后端退役（影子观察 → 灰度切流 → 退役）

> 本部分依据 `metaclouds-backend-rust/docs/cutover-plan.md`（切流方案）与 `go-retirement-checklist.md`（退役清单）整理。标注「🔧 待目标环境执行」的步骤均在目标环境操作。

### C.1 切流前准备（D-7 前全部就绪，任一项不达标不得进入灰度）

| 项 | 验收标准 | 依据 |
|---|---|---|
| Golden 回归 | P0/P1 diff = 0，P2 文案差异 < 10 条并有裁决 | P4-01 |
| 影子双轨 | 连续 **5 个工作日**观察无 must-fix diff；P99 不劣于 Go | P4-02（= 本指南 C.2） |
| 性能基准 | Rust P99 ≤ Go 版 120%，吞吐达预期，报告归档 | P4-03 |
| 测试映射 | 映射表 100% 填充（242 条） | P4-04 |
| Runbook 演练 | 完成一次部署演练 + 一次回滚演练，运维可独立操作 | P4-05 |

**数据迁移**（🔧 待目标环境）：
- Rust 迁移 `001_initial` ~ `008_b6_governance` 在生产 PostgreSQL 全部 up 成功（sqlx 启动期自动执行）。
- 种子数据 `003_b1_seed.sql`（default 租户 + admin）已落库。
- 共用同一 PG 库则直接切连接；独立新库需 pg_dump/psql 导入历史数据并逐表对账（users/tenants/clusters/jobs）。
- 密码哈希：Go 老用户 bcrypt → Rust 登录时先 bcrypt 校验、成功自动重哈希为 argon2；切流后观察登录成功率。

**配置核对**（依据 `src/config.rs::validate` 启动期 fail-fast，任一缺失进程无法启动）：
- `JWT_SECRET` ≥ 32 字符，**与 Go 版同一密钥**（否则全量用户被迫重新登录）
- `DATABASE_URL` 正确指向 PG，生产 `DATABASE_SSL_MODE=require`（禁 disable）
- `REDIS_ENABLED=true`、`REDIS_URL` 可达
- `ALLOWED_ORIGINS` 精确列出前端域名，**不得为 `*`**
- `SERVER_ENV=production`（禁用 SQLite/内存模式/公开注册）
- `TRUSTED_PROXIES` 填 Nginx/Ingress 网段

**监控就绪**：Prometheus 抓取 `/metrics`（ServiceMonitor 15s）、Grafana 面板导入（13 业务 + 3 HTTP 指标，前缀 `metaclouds_` 与 Go 一致可套用）、16 告警规则生效、JSON 日志含 `trace_id/span_id/request_id`。

**回滚预案**：按 Runbook v2 完成至少一次回滚演练；回滚时间：应用层 < 1 分钟（Nginx 改权重）、K8s 层 < 5 分钟（Ingress 改后端）。

### C.2 影子观察 ≥ 1 周（P4-02）

**脚本**：`metaclouds-backend-rust/scripts/shadow-observe.ps1`

```powershell
# 目标环境完整观察：5 个工作日（7200 分钟），每 30s 一轮，只读请求集（长跑不写数据）
powershell -ExecutionPolicy Bypass -File metaclouds-backend-rust/scripts/shadow-observe.ps1 `
  -DurationMinutes 7200 -IntervalSeconds 30 -RequestSet read-only `
  -GoPort 8000 -RustPort 8001 -AdminPass '<admin密码>'

# 本机短验证（脚本逻辑）：5 分钟
powershell -ExecutionPolicy Bypass -File metaclouds-backend-rust/scripts/shadow-observe.ps1 `
  -DurationMinutes 5 -RequestSet read-only
```

**脚本行为**：定时向 Go(:8000) 与 Rust(:8001) 双发同一混合请求集，逐请求对比状态码 / 响应体 SHA256 / 信封键集 / data 形状 / 耗时；diff 分级 P0（状态码不一致）/ P1（信封或形状）/ P2（延迟劣化）/ P3（可接受差异）；每 2 小时自动重登刷新 token；输出 `shadow-observe-yyyyMMdd-HHmm.log` + 结束汇总 `shadow-observe-summary.txt`（默认输出目录 `scripts/p4work/`）。

**观察结论（验收）**：连续 5 个工作日 **无 P0/P1 diff**、P99 延迟不劣于 Go → 进入 C.3。若出 must-fix diff，修复 Rust 版后重置观察期。

### C.3 灰度切流三档（P4-06）

**三阶段灰度**（每档达标再进下一档，Go 全程热备）：

| 阶段 | 流量比例 | 分流方式 | 观察时长 | 达标门槛 |
|---|---|---|---|---|
| 阶段 1 | 10% | Nginx `split_clients` / Ingress 权重 / Istio weight | 24 小时 | 错误率 < 0.1%、P99 < Go 120%、无 P0 业务异常 |
| 阶段 2 | 50% | 权重提到 50% | 24 小时 | 同上 + DB 连接池 / Redis 无瓶颈 |
| 阶段 3 | 100% | 全量切到 Rust，Go 保留运行 1 周热备（不接流量） | 1 周稳定期 | 见 C.4 |

**Nginx 灰度示例**（裸机/同机部署，Go :8000 ↔ Rust :8001）：

```nginx
upstream backend_go  { server localhost:8000; }
upstream backend_rust { server localhost:8001; }

# 按客户端 IP 哈希分流
split_clients "${remote_addr}AAA" $backend {
    10%     backend_rust;   # 阶段 1 → 50%（阶段 2）→ *（阶段 3）
    *       backend_go;
}

server {
    listen 443 ssl;
    server_name app.metaclouds.example.com;
    location /api/ {
        proxy_pass http://$backend;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
        client_max_body_size 100m;
    }
}
```

**K8s 灰度**：Ingress 后端 Service 权重切换（方式 A）；推荐 Istio VirtualService / Argo Rollouts Canary 按 weight 切流（方式 B）。

**自动化切流验证脚本** `metaclouds-backend-rust/scripts/cutover-verify.ps1`（模拟切流，按 rustPct 随机分发请求，统计错误率/P99，检查回滚条件）：

```powershell
# 完整三档（10→50→100，各 60s）+ 回滚演练（100% 切回 Go）
powershell -ExecutionPolicy Bypass -File metaclouds-backend-rust/scripts/cutover-verify.ps1 -IncludeRollbackDrill

# 只跑 10% 档，30s 采样（预发快速验证）
powershell -ExecutionPolicy Bypass -File metaclouds-backend-rust/scripts/cutover-verify.ps1 -Stage 10 -SampleSeconds 30
```

**每档放量后立即验证**（🔧 待目标环境）：
1. `POST /api/v1/auth/login` → 200，JWT Cookie 下发
2. `GET /api/v1/auth/csrf` 取 token，写操作带 `X-CSRF-Token` 正常
3. 核心 CRUD：`/users` `/tenants` `/clusters` `/resources` `/jobs` `/gpus` 带 token → 200
4. `GET /api/v1/monitoring/dashboard` 数据正常
5. `GET /metrics` → 200，含 13 业务指标 + 3 HTTP 指标
6. `GET /swagger-ui`、`GET /api-docs/openapi.json` 可访问
7. 未鉴权 `GET /api/v1/users` → 401 信封正确
8. 响应头含 `X-Trace-Id`，日志可按 trace_id 串联

**回滚触发条件（任一满足立即回滚）**：
- 错误率 > 1%（持续 5 分钟）
- P99 > Go 版 200%（持续 5 分钟）
- 核心业务功能不可用（登录失败、作业无法提交、Dashboard 空数据）
- 数据不一致（核心表行数/关键字段与 Go 分叉）
- 16 告警中 critical 级触发（GPUUnavailable / StorageLow / HighErrorRate / DatabaseConnectionPoolExhausted / RedisConnectionDown）

### C.4 稳定观察期（D2 ~ D9）

每日检查：错误率 / P99 / CPU / Memory / 16 告警；DB 连接池水位（`metaclouds_db_connections_in_use / max < 90%`）；Redis `up == 1`；核心业务流程（作业提交/取消、GPU 分配/释放、告警确认/解决）连续正常；核心表行数对账无漂移。Go 版保持进程启动、健康检查通过、流量权重为 0。

### C.5 退役清单执行（D10，按 `go-retirement-checklist.md`）

- **A 组 退役前确认（10 项）**：稳定运行 ≥1 周 / Golden 回归 P0/P1=0 / 影子双轨 5 工作日 / 性能达标 / 遗留项已评估 / 前端全量切指 / 监控切换 / 日志收集 / 数据库备份验证 / 回滚预案最终确认
- **B 组 退役操作（8 项）**：
  - B1 停止 Go 服务：`kubectl scale deployment/<go-deployment> --replicas=0 -n metaclouds`（K8s）或停止进程（裸机）
  - B2 **保留 Go 代码仓库不删**，打 tag 归档（`frozen-pre-rust`）
  - B3 保留 Go 数据库 ≥ 30 天
  - B4 更新文档统一指向 Rust 版（utoipa 导出 openapi.json）
  - B5 移除 Go 构建/测试流水线 CI job
  - B6 清理/标注 Go 专属监控面板
  - B7 通知团队
  - B8 记录归档
- **C 组 退役后观察**：24h 无异常 / 7 天无回滚需求 / 30 天归档 Go 数据库（dump 冷存储）/ 90 天可归档 Go 代码（只读归档）
- **D 组 回滚触发（退役后仍可回滚）**：Rust 严重故障 30 分钟无法修复 / 需从 Go 库恢复数据 / 业务方紧急要求

### C.6 切流/退役时间线

| 时间点 | 事件 |
|---|---|
| D-7 | 切流前准备完成（C.1 全部绿灯） |
| D0 | 10% 灰度，观察 24h |
| D1 | 50% 灰度，观察 24h |
| D2 | 100% 全量切流，Go 保留热备 |
| D2~D9 | 1 周稳定观察期 |
| D10 | 退役清单执行（C.5） |
| D10+ | Go 停服，退役后观察（24h / 7d / 30d / 90d） |

---

## D. K8s 真实部署

### D.1 清单文件（14 个，已交付，63/63 YAML 校验通过）

目录：`metaclouds-backend-rust/k8s/`

| # | 文件 | 内容 |
|---|---|---|
| 1 | `00-namespace.yaml` | `metaclouds` 命名空间 |
| 2 | `01-configmap.yaml` | 非敏感环境变量（57 项） |
| 3 | `02-secret.yaml` | JWT_SECRET / DATABASE_PASSWORD / REDIS_PASSWORD（占位，需替换） |
| 4 | `03-serviceaccount.yaml` | 服务账号（最小权限） |
| 5 | `04-role.yaml` | RBAC Role + RoleBinding |
| 6 | `05-deployment.yaml` | 3 副本、三探针（/health）、非 root UID 10001、readOnlyRootFilesystem、topologySpreadConstraints、滚动更新 maxSurge=1/maxUnavailable=0 |
| 7 | `06-service.yaml` | ClusterIP :8000 |
| 8 | `07-hpa.yaml` | CPU 50% / 内存 70% 扩缩（3-10 副本） |
| 9 | `08-pdb.yaml` | minAvailable: 1 |
| 10 | `09-networkpolicy.yaml` | 默认拒绝入站，仅允许前端/监控网段 |
| 11 | `10-ingress.yaml` | HTTPS + `app.metaclouds.example.com` → `/api` → backend:8000 |
| 12 | `11-servicemonitor.yaml` | Prometheus 15s 抓取 `/metrics` |
| 13 | `12-prometheusrule.yaml` | 16 条告警规则（`metaclouds-alerts`） |
| 14 | `13-kustomization.yaml` | Kustomize 统一引用（13 个资源 + 公共 labels + 镜像 tag 注入） |

> **探针说明**：Rust 版已提供根级 `GET /health`（无 JWT，返回 200），三探针（startup/liveness/readiness）统一使用 `/health`；`/metrics` 仅供 Prometheus 抓取。

### D.2 前置条件

- K8s 集群（1.25+），`kubectl` 可用且有 `cluster-admin`（或命名空间级 admin）权限
- Ingress Controller（nginx-ingress）已安装
- Prometheus Operator + ServiceMonitor CRD 已安装（抓取 `/metrics`、加载 16 告警规则）
- （可选）Cert-Manager 或已备 TLS 证书

### D.3 准备 Secrets

```bash
# 1. 生成/准备生产密钥（JWT_SECRET ≥ 32 字符）
openssl rand -base64 48

# 2. 替换 02-secret.yaml 中的占位值（base64 编码）
#    JWT_SECRET、DATABASE_PASSWORD、REDIS_PASSWORD 三项
kubectl create ns metaclouds
# 或直接用 kubectl 创建 secret 覆盖占位（更安全，不落盘）
kubectl -n metaclouds create secret generic metaclouds-secret \
  --from-literal=JWT_SECRET='<≥32字符密钥>' \
  --from-literal=DATABASE_PASSWORD='<pg密码>' \
  --from-literal=REDIS_PASSWORD='<redis密码>'
```

### D.4 部署

```bash
# 方式一：kustomize 一键部署（推荐）
kubectl apply -k metaclouds-backend-rust/k8s/

# 方式二：逐文件 apply
kubectl apply -f metaclouds-backend-rust/k8s/00-namespace.yaml
kubectl apply -f metaclouds-backend-rust/k8s/01-configmap.yaml
kubectl apply -f metaclouds-backend-rust/k8s/02-secret.yaml
kubectl apply -f metaclouds-backend-rust/k8s/03-serviceaccount.yaml
kubectl apply -f metaclouds-backend-rust/k8s/04-role.yaml
kubectl apply -f metaclouds-backend-rust/k8s/05-deployment.yaml
kubectl apply -f metaclouds-backend-rust/k8s/06-service.yaml
kubectl apply -f metaclouds-backend-rust/k8s/07-hpa.yaml
kubectl apply -f metaclouds-backend-rust/k8s/08-pdb.yaml
kubectl apply -f metaclouds-backend-rust/k8s/09-networkpolicy.yaml
kubectl apply -f metaclouds-backend-rust/k8s/10-ingress.yaml
kubectl apply -f metaclouds-backend-rust/k8s/11-servicemonitor.yaml
kubectl apply -f metaclouds-backend-rust/k8s/12-prometheusrule.yaml
```

### D.5 镜像 tag 注入

```bash
# 生产镜像已发布后（见 B），用 git sha 覆盖占位 tag：
kubectl -n metaclouds set image deployment/metaclouds-backend \
  metaclouds-backend=ghcr.io/customizedvalidation/yunchuang/backend-rust:${GIT_SHA}

# 或修改 13-kustomization.yaml 的 images 段后重新 apply -k
```

### D.6 部署验证

```bash
# 1. Pod 就绪（三探针命中 /health）
kubectl -n metaclouds get pods -l app=metaclouds
kubectl -n metaclouds rollout status deployment/metaclouds-backend

# 2. 健康检查（直接访问）
kubectl -n metaclouds exec deploy/metaclouds-backend -- wget -qO- http://localhost:8000/health
#    → {"status":"ok"}（或等价 200 响应）

# 3. 指标端点
kubectl -n metaclouds exec deploy/metaclouds-backend -- wget -qO- http://localhost:8000/metrics | head -20
#    → 含 metaclouds_ 前缀业务指标 + HTTP 指标

# 4. Ingress 访问
curl -k https://app.metaclouds.example.com/api/v1/health
#    未鉴权核心端点应返回 401 信封（说明路由 + 鉴权均正常）

# 5. 扩展性资源生效
kubectl -n metaclouds get hpa,networkpolicy,pdb
```

### D.7 升级与回滚

- **滚动更新**：Deployment 策略 `maxSurge=1 / maxUnavailable=0` → `kubectl set image` 即零宕机滚动。
- **回滚上一版本**：`kubectl -n metaclouds rollout undo deployment/metaclouds-backend`
- **回滚到 Go 版（极端场景）**：恢复 Go Deployment（保留仓库），Ingress 后端 Service 改回 `go-backend:8000`，< 5 分钟。

---

## E. 生产 PostgreSQL 实跑

### E.1 前置

- PostgreSQL 实例（14+/16 均可；CI 用 postgres:16 验证过），创建库与用户：

```sql
CREATE USER metaclouds WITH PASSWORD '<密码>';
CREATE DATABASE metaclouds OWNER metaclouds;
GRANT ALL PRIVILEGES ON DATABASE metaclouds TO metaclouds;
```

### E.2 连接配置

后端启动时注入（K8s 走 ConfigMap/Secret；裸机走环境变量）：

```text
DATABASE_URL=postgres://metaclouds:<密码>@<pg-host>:5432/metaclouds
USE_SQLITE=false
DATABASE_SSL_MODE=require          # 生产强制；本机测试可 disable
```

### E.3 迁移执行

迁移通过 sqlx 内嵌宏 `sqlx::migrate!("./migrations")` **进程启动时自动执行**（8 个迁移 `001_initial` ~ `008_b6_governance`），也可手动确认：

```bash
cd metaclouds-backend-rust
DATABASE_URL='postgres://metaclouds:<密码>@<pg-host>:5432/metaclouds' \
  sqlx migrate run            # 需要 sqlx-cli；或直接启动服务观察迁移日志
```

迁移完成后校验：

```sql
-- 20 张表 + 种子数据
SELECT count(*) FROM information_schema.tables
WHERE table_schema='public' AND table_type='BASE TABLE';  -- 期望 20
SELECT id, username FROM users;                            -- 期望 admin（argon2 哈希）
SELECT count(*) FROM tenants;                              -- 期望 default 租户
```

> **Postgres 方言关键路径**（CI `rust-test-postgres` job 已用 postgres:16 service 容器验证）：JSONB codec（OID 3802 委托 serde_json）、TIMESTAMPTZ、SERIAL 自增、软删除、UPSERT。生产实跑建议再跑一次：

```bash
# 在 CI 复跑 Postgres 双驱动测试（或本地连真实 PG）
cargo test --test postgres_smoke_test -- --ignored --nocapture
cargo test --test postgres_integration_test -- --ignored --nocapture
```

### E.4 连接池调优（sqlx PgPool）

按 `docs/performance-benchmark.md` 与 Runbook §8.3 调优：

| 参数 | 建议值 | 说明 |
|---|---|---|
| `DATABASE_MAX_CONNECTIONS` | 10~20 | 单实例连接池上限 |
| `DATABASE_MAX_IDLE` | 5~10 | 空闲连接保留 |
| `DATABASE_ACQUIRE_TIMEOUT` | 5~10s | 获取连接超时 |
| 监控 | `metaclouds_db_connections_in_use / max < 90%` | 连接池水位告警 |

### E.5 生产验证清单

- [ ] 20 张表全部创建，索引 53/53 与设计对齐
- [ ] 种子数据（default 租户 + admin）存在，登录成功
- [ ] 业务 CRUD（作业/GPU/分区/配额）写入后重启服务数据不丢
- [ ] `DATABASE_SSL_MODE=require` 下连接成功（无 TLS 报错）
- [ ] 连接池水位正常，无 `DatabaseConnectionPoolExhausted` 告警

---

## F. 全局验收清单（五项联动）

| # | 验收项 | 通过标准 |
|---|---|---|
| 1 | SSH 部署激活 | workflow_dispatch staging 后 `Deploy to Staging (SSH)` 真执行，curl /health = 200 |
| 2 | 镜像发布 | ghcr 可见 `<repo>/backend-rust:<sha>` 与 frontend 镜像，kubectl set image 成功 |
| 3 | K8s 部署 | 3 Pod Ready，三探针 /health 通过，Ingress 可访问，HPA/PDB/NP 生效 |
| 4 | PG 实跑 | 迁移 up、种子数据存在、业务 CRUD 持久化、连接池稳定 |
| 5 | 影子/切流 | 5 工作日无 P0/P1 diff；10%→50%→100% 每档 24h 达标 |
| 6 | Go 退役 | A~E 组清单全勾选，Go 0 流量，归档记录完整 |
| 7 | CI 全绿 | 最新 run 14 job 全绿（含 rust-test-postgres） |

---

## G. 附录

### G.1 关键路径索引

| 材料 | 路径 |
|---|---|
| 本部署指南 | `docs/production-deployment-guide.md` |
| Secrets 配置 | `docs/ci-deployment-secrets.md` |
| 切流方案 | `metaclouds-backend-rust/docs/cutover-plan.md` |
| Go 退役清单 | `metaclouds-backend-rust/docs/go-retirement-checklist.md` |
| 运维 Runbook v2 | `metaclouds-backend-rust/docs/runbook-v2-rust.md` |
| 回滚演练 | `metaclouds-backend-rust/docs/deployment-rollback-drill.md` |
| 影子双轨方案 | `metaclouds-backend-rust/docs/shadow-dual-track.md` |
| 性能基准 | `metaclouds-backend-rust/docs/performance-benchmark.md` |
| API 参考（Rust） | `metaclouds-backend-rust/docs/api-reference-rust.md` |
| 复盘报告 | `metaclouds-backend-rust/docs/comprehensive-review-2026-09-20.md` |
| 测试映射 | `metaclouds-backend-rust/docs/test-mapping-phase4.md` |

### G.2 脚本

| 脚本 | 用途 |
|---|---|
| `metaclouds-backend-rust/scripts/shadow-observe.ps1` | P4-02 影子观察（默认 24h，5 工作日 7200min） |
| `metaclouds-backend-rust/scripts/cutover-verify.ps1` | P4-06 三档切流验证（10/50/100 + 回滚演练） |
| `metaclouds-backend-rust/scripts/shadow-compare.ps1` | 影子对比（短时） |
| `metaclouds-backend-rust/scripts/benchmark.ps1` | 性能基准 |
| `metaclouds-backend-rust/scripts/validate-k8s-yaml.ps1` | K8s YAML 校验（本机已 63/63 通过） |

### G.3 关键命令速查

```bash
# Secrets 就绪后手动触发 staging 部署（GitHub UI 或 gh）
gh workflow run ci-cd.yml -f environment=staging

# K8s 部署
kubectl apply -k metaclouds-backend-rust/k8s/

# 影子观察（目标环境 5 工作日）
powershell -ExecutionPolicy Bypass -File scripts/shadow-observe.ps1 -DurationMinutes 7200 -RequestSet read-only

# 切流验证（三档 + 回滚演练）
powershell -ExecutionPolicy Bypass -File scripts/cutover-verify.ps1 -IncludeRollbackDrill

# Go 停服（退役 B1）
kubectl scale deployment/<go-deployment> --replicas=0 -n metaclouds
```

---

> **最后提示**：本指南所有命令均需在目标环境由具备相应权限的运维执行；执行前确认已备份数据库与配置。任何一步失败，回滚路径见 Runbook v2（应用层 < 1 分钟 / K8s < 5 分钟）。
