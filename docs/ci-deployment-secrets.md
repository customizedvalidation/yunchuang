# CI 部署 Secrets 配置指南

配置以下 GitHub Actions secrets 后，对应部署环节会在下次 push / workflow_dispatch 时**自动启用**（无需再改 workflow 代码）。

## Secrets 清单

| Secret | 用于 | 说明 |
|---|---|---|
| `DEV_SSH_KEY` | Deploy to Development | SSH 私钥（目标开发服务器，`ssh-keygen -t ed25519` 生成，公钥加入服务器 `~/.ssh/authorized_keys`） |
| `DEV_HOST` | Deploy to Development | 开发服务器地址（IP 或域名） |
| `DEV_USER` | Deploy to Development | 登录用户（需有仓库 clone 目录写权限） |
| `STAGING_SSH_KEY` | Deploy to Staging | SSH 私钥（同上） |
| `STAGING_HOST` | Deploy to Staging | 预发服务器地址 |
| `STAGING_USER` | Deploy to Staging | 登录用户 |
| `PROD_SSH_KEY` | Deploy to Production | SSH 私钥（同上） |
| `PROD_HOST` | Deploy to Production | 生产服务器地址 |
| `PROD_USER` | Deploy to Production | 登录用户 |
| `DEPLOY_DIR` | 全部部署 job（可选） | 服务器上仓库目录，默认 `/opt/metaclouds` |

## 触发方式

| 环境 | 自动触发 | 手动触发 |
|---|---|---|
| Development | push 到 `develop` 分支 | workflow_dispatch + environment=development |
| Staging | push 到 `main` 分支 | workflow_dispatch + environment=staging |
| Production | 不自动触发（安全） | workflow_dispatch + environment=production（经 approval gate） |

## 服务器前置条件

目标服务器需满足（SSH 命令会自动执行）：

```bash
# 1. 安装 Go（deploy.sh 需要本机构建）
# 2. 克隆仓库到部署目录（默认 /opt/metaclouds）
sudo mkdir -p /opt/metaclouds && sudo chown $USER /opt/metaclouds
cd /opt/metaclouds && git clone https://github.com/customizedvalidation/yunchuang.git .

# 3. 准备环境配置文件（deploy.sh 要求）
cd /opt/metaclouds/metaclouds-backend
cp .env.example .env.staging      # 按需改为 .env.development / .env.production
# 4. 服务器上生成部署密钥对，公钥加入 authorized_keys
ssh-keygen -t ed25519 -f ~/.ssh/metaclouds_deploy
cat ~/.ssh/metaclouds_deploy.pub >> ~/.ssh/authorized_keys
# 私钥内容（~/.ssh/metaclouds_deploy）粘贴到对应 *_SSH_KEY secret
```

## 部署执行逻辑

CI 中每个部署 job 的 SSH 步骤仅在对应 `*_SSH_KEY` 与 `*_HOST` 均已配置时执行：

```bash
echo "${{ secrets.XXX_SSH_KEY }}" > /tmp/deploy_key
ssh -i /tmp/deploy_key user@host \
  "cd $DEPLOY_DIR && git pull --ff-only && cd metaclouds-backend && ./deploy.sh -e <env> -l <LEVEL> -k"
```

未配置 secret 时步骤自动跳过，job 保持绿色，不影响主 CI 链路。

## 生产环境保护（建议）

在 GitHub → Settings → Environments → `production` 中：
1. 添加 **Required reviewers**（如 2 人审批）
2. 勾选 **Wait timer**（如 5 分钟冷却期）

这样 workflow_dispatch production 会在真正执行 `Deploy to Production (SSH)` 前等待人工审批，避免误部署。
