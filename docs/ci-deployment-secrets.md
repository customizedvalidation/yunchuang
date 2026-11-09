# CI 部署 Secrets 配置指南（索引）

- 更新：2026-11-09（第三轮）
- **当前真源**：[`docs/deployment-prerequisites.md`](deployment-prerequisites.md)
- **变量清单与校验规则的可执行版本**：`bash scripts/check-deploy-prereqs.sh --list`

---

## ⚠️ 本文档旧版已废止

旧版（2026-09-28 起）描述的是**已退役的部署路径**，请勿按它操作：

| 旧版说法 | 现状 |
|---|---|
| 目标机执行 `cargo build --release && sudo systemctl restart metaclouds-backend-rust` | 已改为 SSH 取编排文件后 `docker compose pull/up` 拉同一 `sha-<commit>` 的 GHCR 镜像，目标机不再编译（2026-11-09 第二轮收敛） |
| 缺 `*_SSH_KEY` 时步骤自动跳过，job 保持绿色 | 已废止：这是「没部署却报成功」的假绿。现在由 `deploy-prereqs` 总闸统一 gate，未启用时部署 job 显示 **skipped**，既不报绿也不报红 |
| 只需 3 个环境 × 3 个 secret + `DEPLOY_DIR` | 还需 `*_USER`、`*_BASE_URL`、`KUBE_CONFIG`、6 个 K8s 真实凭据；且做格式校验（占位域名 / 假 PEM key / 假 kubeconfig 会被拒绝） |
| `K8S_BASE_URL` 是必需前置 | 降为可选：K8s 验收走 `kubectl port-forward`，该地址仅用于 GitHub Environment URL 展示 |

---

## 30 秒上手

```bash
# 1) 看当前还差什么（不联网、不改动）
bash scripts/check-deploy-prereqs.sh --report

# 2) 按 runbook 配好之后做严格自检
bash scripts/check-deploy-prereqs.sh --env-file .env.deploy --target staging --strict

# 3) 打开总闸
gh variable set DEPLOY_ENABLED --body 'true'
gh variable set DEPLOY_TARGETS --body 'staging'
```

推送代码**不要求**先完成运维配置：未启用时部署段明确 skipped，主构建链路照常运行。配置齐全后再打开 `DEPLOY_ENABLED`，此时 `deploy-prereqs` 才会强校验，缺一项即按设计变红并一次报全。

---

## 分组速查

| 组 | GitHub Variables | GitHub Secrets |
|---|---|---|
| 总闸 | `DEPLOY_ENABLED`、`DEPLOY_TARGETS`、`REQUIRE_PG_HA`（可选） | — |
| development | `DEV_BASE_URL` | `DEV_SSH_KEY`、`DEV_HOST`、`DEV_USER` |
| staging | `STAGING_BASE_URL` | `STAGING_SSH_KEY`、`STAGING_HOST`、`STAGING_USER` |
| production | `PROD_BASE_URL` | `PROD_SSH_KEY`、`PROD_HOST`、`PROD_USER` |
| 共用 | — | `DEPLOY_DIR`（可选，默认 `/opt/metaclouds`） |
| k8s | `K8S_BASE_URL`（可选） | `KUBE_CONFIG`、`K8S_JWT_SECRET`、`K8S_DATABASE_URL`、`K8S_REDIS_PASSWORD`、`K8S_DEFAULT_ADMIN_PASSWORD`、`K8S_POSTGRES_PASSWORD`、`K8S_PROTECTED_ENDPOINTS_TOKEN` |

生成 K8s 凭据：`scripts/generate-deploy-secrets.sh`（强制 `K8S_POSTGRES_PASSWORD` 与 `K8S_DATABASE_URL` 口令同源）。

---

## 触发方式（未改动）

| 环境 | 自动触发 | 手动触发 |
|---|---|---|
| development | push 到 `develop` | `workflow_dispatch` + `environment=development` |
| staging | push 到 `main` | `workflow_dispatch` + `environment=staging` |
| production | 不自动触发 | `workflow_dispatch` + `environment=production`，经 `production-approval` 审批门（需 staging 成功） |
| k8s | push 到 `main` | `workflow_dispatch` |

以上触发条件**还要叠加** `deploy-prereqs` 输出的对应 target 开关；未在 `DEPLOY_TARGETS` 中列出的目标一律 skipped。

## 生产环境保护（建议）

在 GitHub → Settings → Environments → `production` 中添加 **Required reviewers**（建议 2 人）并勾选 **Wait timer**（如 5 分钟冷却期）。

## 延伸阅读

- `docs/deployment-prerequisites.md` — 完整 runbook（生成命令、目标机预置、故障排查、上线复核）
- `docs/system-fixes-2026-11-09.md` — 第二轮修复与剩余阻塞项台账
- `scripts/verify-deployment.sh` — 部署后真实断言（`/health/ready` JSON 信封 + `/api/v1/system/mode`）
