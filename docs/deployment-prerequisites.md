# 部署前置条件 Runbook（推送前运维配置）

- 日期：2026-11-09（第三轮）
- 适用：`.github/workflows/ci-cd.yml` 的 `deploy-development` / `deploy-staging` / `deploy-production` / `deploy-k8s`
- 唯一真源：`scripts/check-deploy-prereqs.sh`（变量清单与校验规则都在脚本里，`--list` 可打印）
- 相关：`docs/system-fixes-2026-11-09.md`（第二轮修复）、`docs/k8s-production-switch-runbook.md`（生产切换）

---

## 一、结论：推送不再被运维配置阻塞

第二轮修复把「缺配置就跳过并报绿」改成了「缺配置就 `exit 1`」，方向正确，但带来两个副作用：

1. **推送前必须先一次性配齐三套环境**，否则 `main` 一推就整条流水线变红 —— 「运维还没就绪」被误报成「代码有问题」。
2. 前置条件只存在于文档表格，CI 侧是 4 个 job 各自手写的 3 行 Guard：漏检 `*_USER` / `DEPLOY_DIR` / `KUBE_CONFIG` / K8s 真实凭据，**只查非空、不查格式**（`https://dev.example.com` 这种占位域名、随便一段文本的假 SSH key、非 kubeconfig 的 base64 都能通过）。

第三轮引入 `deploy-prereqs` 总闸，把状态改成显式三态：

| 状态 | 条件 | CI 表现 | 含义 |
|---|---|---|---|
| 未启用 | `DEPLOY_ENABLED` ≠ `true` | 4 个部署 job **skipped**，主构建链路照常绿 | 既不是成功部署，也不是失败；日志给 `::notice::` 并打印还差哪些变量 |
| 声明启用但没配齐 | `DEPLOY_ENABLED=true` 且 `DEPLOY_TARGETS` 中任一目标前置条件不全 | `deploy-prereqs` **exit 1，流水线变红** | 一次报全部缺失项；不允许「半个部署」 |
| 已配齐 | `DEPLOY_ENABLED=true` 且目标校验通过 | 仅 `DEPLOY_TARGETS` 列出的目标 job 真正部署 | 部署后仍走 `verify-deployment.sh` 真实断言 |

> 因此：**代码推送不需要任何运维配置**（部署段明确 skipped）。运维准备好后再打开开关，才进入强校验。这既保留了 fail-closed，又不让「还没部署」伪装成绿或误报成红。

---

## 二、变量总表

完整版随时可用 `bash scripts/check-deploy-prereqs.sh --list` 打印，下表是分组摘要。

### 2.1 总闸（GitHub → Settings → Secrets and variables → Actions → **Variables**）

| Variable | 默认 | 说明 |
|---|---|---|
| `DEPLOY_ENABLED` | 空（= 不启用） | 设为 `true` 才启用部署链路 |
| `DEPLOY_TARGETS` | `staging` | 逗号分隔：`development,staging,production,k8s`。**含 `production` 必须同时含 `staging`**（审批门 `production-approval` 依赖 staging 成功） |
| `REQUIRE_PG_HA` | 空 | `true` 时把内置 PG 单点从 warning 升级为硬门禁 |

### 2.2 compose 部署（三个环境结构相同）

| 类别 | 名称 | 校验规则 |
|---|---|---|
| Variable | `{DEV,STAGING,PROD}_BASE_URL` | `http(s)://` 绝对地址；拒绝 `example.com/org/net`、`CHANGEME`、`placeholder` |
| Secret | `{DEV,STAGING,PROD}_SSH_KEY` | 必须含 `-----BEGIN ... PRIVATE KEY-----` |
| Secret | `{DEV,STAGING,PROD}_HOST` | 非空、无空白、非 `example.*` / 占位 |
| Secret | `{DEV,STAGING,PROD}_USER` | 裸用户名，不含空白 / `@` / 占位 |
| Secret | `DEPLOY_DIR` | 可选，默认 `/opt/metaclouds`；给了就必须是绝对路径 |

> `K8S_BASE_URL` 在第二轮文档里被列为「必需」，实际只用于 GitHub Environment 的 URL 展示 —— K8s 验收走 `kubectl port-forward`，不依赖该地址。**已降为可选**，preflight 在缺失时只打印 `[INFO]`，不再阻塞。

### 2.3 K8s 部署（全部为 Secrets，除 `K8S_BASE_URL`）

| Secret | 校验规则 |
|---|---|
| `KUBE_CONFIG` | 合法 base64，解码后含 `apiVersion:` / `clusters:`；**解码内容绝不打印** |
| `K8S_JWT_SECRET` | ≥ 32 字符，非占位 |
| `K8S_DATABASE_URL` | `postgresql://user:password@host/db` 且非占位 |
| `K8S_REDIS_PASSWORD` | ≥ 16 字符 |
| `K8S_DEFAULT_ADMIN_PASSWORD` | ≥ 12 字符 |
| `K8S_POSTGRES_PASSWORD` | ≥ 16 字符，**必须与 `K8S_DATABASE_URL` 中的口令相同** |
| `K8S_PROTECTED_ENDPOINTS_TOKEN` | ≥ 16 字符 |
| `K8S_BASE_URL`（Variable） | 可选，见上 |

> ⚠️ **两个必须由运维修掉的死结（本轮已修）**
> 1. 仓库内 `k8s/02-secret.yaml` 是恒含 `CHANGEME` 的占位模板。旧 CI Guard 直接解析它 → 只要启用 K8s 部署就 100% 失败；且旧 Guard 还依赖未安装的 PyYAML、错误分支含无效 Python 引号、把解码后的凭据明文打进公开日志。
> 2. 即使绕过 Guard，`kustomize build \| kubectl apply -f -` 会把集群里预置的真实 Secret **覆盖回占位值**（`scripts/deploy-cluster.sh` 的 ④.5 步骤就是在打这个补丁）。
>
> 现在 `deploy-k8s` 在 apply 前调用 `scripts/render-k8s-secret.sh`，从上述 6 个 CI secrets 渲染真实 Secret 的**工作副本**（只存在于 runner checkout，不提交），Guard 改为校验渲染结果（纯标准库、只报 key 名），`kustomize build` 拿到的就是真实值。

> ⚠️ **口令同源**：`17-postgresql.yaml` 用 `postgres-password` 初始化 `POSTGRES_PASSWORD`，后端用 `database-url` 里的口令连接。两者不一致时清单全部 apply 成功、后端却在鉴权阶段起不来，且失败发生在部署之后极难归因。请用 `scripts/generate-deploy-secrets.sh`，它强制两者同源。

### 2.4 目标机预置（不是 GitHub 配置，SSH 部署前必须人工完成）

CI 只通过 SSH 执行 `git pull --ff-only` 取编排文件，然后 `docker compose pull/up` 拉同一 `sha-<commit>` 的 GHCR 镜像；**不在目标机编译**。目标机需要：

1. Docker + Compose 插件；`docker login ghcr.io`（能拉 `backend-rust` / `frontend` 镜像的只读凭据）。
2. 仓库克隆在 `DEPLOY_DIR`，且 `git pull --ff-only` 可无交互执行。
3. 同目录 `.env`（被 `.gitignore` 忽略）至少包含：
   `POSTGRES_USER` / `POSTGRES_PASSWORD` / `POSTGRES_DB` / `REDIS_PASSWORD` / `JWT_SECRET` / `DEFAULT_ADMIN_PASSWORD` / `DATABASE_URL`（`postgresql://`）/ `USE_SQLITE=false` / `IMAGE_TAG=sha-<commit>` / `ALLOW_SIMULATED_EXECUTION`（演示集群才设 `true`）。
4. 新版前端 nginx 镜像（含 `location /health/` 反代）+ 新版 Ingress（`/health` → backend）。只更新后端镜像会让 `verify-deployment.sh` 拿到 SPA 回退的 `index.html`，验收按设计失败。
5. 迁移 014（`dispatch_lease`）/ 015（`usage_source`）为纯增量，双驱动各一份，apply 前确认方言目录。

### 2.5 全量注入与注入后核验（2026-11-11 补）

> ⚠️ **两个曾导致「照着做仍然红」的口径缺口，本节已用工具补齐**：
> 1. `generate-deploy-secrets.sh --gh-commands` **只输出 6 个 K8s Secret**，而 `--strict --target all` 实测的 13 项 FAIL 全在它覆盖范围之外（三环境 12 项 + `KUBE_CONFIG`）。以为「注入同一套凭据就配齐了」而直接打开开关，preflight 会立刻 `exit 1`。
> 2. `--strict` 读的是本地 `--env-file`，**不是 GitHub 上的值**；`gh secret` 也不能回读明文。本地 strict 绿只能证明「这一套值本身齐全」，证明不了「已注入 GitHub」。

正确顺序（四步，前两步都不能省）：

```bash
# ① 本地预检：这一套值本身齐不齐（读 .env.deploy，不碰 GitHub）
bash scripts/check-deploy-prereqs.sh --env-file .env.deploy --strict --target all   # 必须 exit 0

# ② 全量注入：生成覆盖全部配置类变量的命令，先审阅再执行
bash scripts/generate-deploy-secrets.sh --from-env .env.deploy --all-gh-commands > /tmp/inject.sh
less /tmp/inject.sh        # 明文只在这个本地文件里；不要提交、不要贴进日志
bash /tmp/inject.sh

# ③ 注入后核验：只列名称，绝不读取任何 Secret 的值
bash scripts/check-deploy-prereqs.sh --verify-gh --target all                       # 必须 exit 0
#    带 --env-file 时还会比对非敏感 Variables 的值是否与本地一致

# ④ 人工打开开关（脚本刻意不代开，避免误开把全绿 CI 变红）
gh variable set DEPLOY_TARGETS --body 'development,staging,production,k8s'
gh variable set DEPLOY_ENABLED --body true
#    最终验收是 CI 的 deploy-prereqs job（--ci 模式，runner 注入 secrets 后校验）
```

工具行为约定：

| 模式 | 覆盖范围 | 缺失值处理 | 是否触碰总闸 |
|---|---|---|---|
| `--gh-commands` | 6 个 K8s Secret | 缺 1 个即 `exit 2` | 否 |
| `--all-gh-commands` | 3 个 `*_BASE_URL` + 可选 `K8S_BASE_URL` + 12 项环境 Secret + `DEPLOY_DIR` + `KUBE_CONFIG` + 6 个 K8s Secret | 显式 `# SKIP` + `::warning::`，**绝不用新随机值偷偷补齐** | 否 |
| `--verify-gh` | 按 `--target` 核对期望键集；打印 `DEPLOY_ENABLED` / `DEPLOY_TARGETS` 实际值 | 缺失即 `exit 1`，一次报全 | 只读 |

`--all-gh-commands` 会强制校验 `K8S_DATABASE_URL` 与 `K8S_POSTGRES_PASSWORD` 同源，多行值（SSH 私钥 PEM）自动改用 quoted heredoc 走 stdin，单行值做单引号转义。`--verify-gh` 在 `gh` 未安装 / 未认证时打印期望键集后 `exit 1`，不会误报通过。

---

## 三、最小可用配置：只启用 staging

```bash
# 0) 本地模板（可提交的是 .env.deploy.example；真实值放 .env.deploy，已被忽略）
cp .env.deploy.example .env.deploy

# 1) 生成部署密钥对，公钥放到目标机 authorized_keys
ssh-keygen -t ed25519 -f ~/.ssh/metaclouds_deploy -N ''
ssh-copy-id -i ~/.ssh/metaclouds_deploy.pub deploy@<staging-host>

# 2) GitHub Variables
gh variable set DEPLOY_ENABLED --body 'true'
gh variable set DEPLOY_TARGETS --body 'staging'
gh variable set STAGING_BASE_URL --body 'https://staging.<real-domain>'

# 3) GitHub Secrets
gh secret set STAGING_SSH_KEY < ~/.ssh/metaclouds_deploy
gh secret set STAGING_HOST --body '<staging-host>'
gh secret set STAGING_USER --body 'deploy'
# DEPLOY_DIR 可省略（默认 /opt/metaclouds）

# 4) 本地自检（不联网、不改动任何东西）
bash scripts/check-deploy-prereqs.sh --env-file .env.deploy --target staging --strict
```

`--strict` 通过后才推 `main`。此时 `deploy-development` / `deploy-production` / `deploy-k8s` 仍为 skipped，不需要它们的任何配置。

---

## 四、启用 K8s 部署

```bash
# 1) 生成 6 个真实凭据（K8S_POSTGRES_PASSWORD 与 DATABASE_URL 口令同源）
bash scripts/generate-deploy-secrets.sh --out .env.deploy.k8s
#   可选：--pg-host / --pg-port / --pg-db / --pg-user / --sslmode

# 2) 用同一份值注入 GitHub（不要单独再跑一次生成，否则两套口令不一致）
bash scripts/generate-deploy-secrets.sh --from-env .env.deploy.k8s --gh-commands | bash

# 3) kubeconfig（建议使用只读/受限的部署账号，不要用 cluster-admin）
base64 -w0 ~/.kube/config | gh secret set KUBE_CONFIG      # Linux
# base64 < ~/.kube/config | tr -d '\n' | gh secret set KUBE_CONFIG   # macOS

# 4) 打开 k8s 目标
gh variable set DEPLOY_TARGETS --body 'staging,k8s'

# 5) 本地校验格式（KUBE_CONFIG 需单独放进 env 或合并到 .env.deploy.k8s）
KUBE_CONFIG="$(base64 -w0 ~/.kube/config)" \
  bash scripts/check-deploy-prereqs.sh --env-file .env.deploy.k8s --target k8s --strict
```

> 生产更优做法：用 External Secrets Operator / Sealed Secrets 托管凭据，Git 只存加密引用；`render-k8s-secret.sh` 是当前的过渡实现。

---

## 五、CI 行为与故障排查

| 现象 | 原因 | 处理 |
|---|---|---|
| 部署 job 全部 `skipped`，日志有 `DEPLOY_ENABLED is not 'true'` | 总闸未打开（**正常状态**） | 配置完再 `gh variable set DEPLOY_ENABLED --body true` |
| `Deployment Preflight` 红，一次列出多项 `[FAIL]` | 声明启用但配置不全/格式不对 | 按日志里的变量名与 `--list` 提示补齐；用 `--strict` 本地复现 |
| `DEPLOY_TARGETS includes production but not staging` | 审批门依赖 staging 成功 | 改成 `staging,production` |
| `Guard - deployment prerequisites` 红 | 有人手动触发绕过了 preflight 的输出 | 检查该 job 的 env 映射是否漏了变量 |
| `render-k8s-secret.sh` 红 | 6 个 K8s secrets 有缺失/太短/含 CHANGEME | 用生成脚本重发，注意 `--from-env` 保持同源 |
| `verify-deployment.sh` 红 | `/health/ready` 拿到 SPA HTML 或非 `success:true` 信封 | 确认新版 nginx 镜像 + Ingress `/health` 已生效；确认 `SERVER_ENV=production` 下 `ALLOW_SIMULATED_EXECUTION` 已按设计设置 |
| 后端启动即退出 | `SERVER_ENV=production` 未显式承认演示模式，或生产 `.env` 缺 `DATABASE_URL` / `DEFAULT_ADMIN_PASSWORD` | 见 `docs/system-fixes-2026-11-09.md` 第一节（这是 fail-closed 的设计行为） |

---

## 六、上线前必须复核的四件事

1. 生产 `01-configmap.yaml` 当前以**演示集群**口径显式承认模拟（`ALLOW_SIMULATED_EXECUTION=true`、`DISPATCH_ENABLED=true`、`K8S_SIMULATION_MODE=true`）。作为真实算力平台交付前必须改为 `false` 并把 `DISPATCH_ENABLED` 设为 `false`（或接入真实执行器后移除模拟路径）。
2. `/api/v1/system/mode` 应返回 `simulated=false`；前端常驻 `ModeBanner` 不再显示演示提示。
3. `jobs.usage_source='simulated'` 的合成实耗**禁止**进入计费 / showback 查询。
4. `kubectl get deploy` 的实际镜像 tag 必须是 `sha-<commit>`；`verify-deployment.sh` 通过且返回的是后端 JSON 信封而非 HTML。

---

## 七、文档关系

- 本文：部署前置条件的**当前真源**（变量、状态机、生成与验收）。
- `docs/ci-deployment-secrets.md`：已重写为索引，旧版描述的「目标机 `cargo build --release` + `systemctl restart`」「缺 secret 自动跳过报绿」均为 2026-10 之前的历史口径，已废止。
- `docs/system-fixes-2026-11-09.md` 第四节：第二轮遗留清单，本轮已按其落地，口径以本文为准。
