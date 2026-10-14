# PR / 发布说明：前端导航统一 + 后端 PostgreSQL HA 翻转 + K8s/CI 强化

> 本说明汇总已合入 `main` 的 7 个提交（`19709be`…`174d967`，区间 2026-10-09 ~ 2026-10-14）。
> 可作为 GitHub PR 描述或发布变更说明直接粘贴使用。

## 一、概述（TL;DR）

本批合并三大块工作：

1. **前端**：统一导航单一来源（`src/nav.ts`），优化菜单子模块，抽离公共 composable / 工具，删除废弃子目录 CI。
2. **后端 / K8s**：后端从 SQLite 翻转至 **PostgreSQL 多副本 HA**（replicas 3 + HPA + PDB），并统一镜像名消除 `ImagePullBackOff`。
3. **运行期前置 + CI + 文档**：运行期前置（PostgreSQL 数据面 + secret 注入）测试数据化落地；CI 把 `verify-k8s-rules.js` 与部署脚本纳入静态把关并统一规则源；新增生产切换 runbook。

## 二、变更范围（按领域）

### 前端（`metaclouds-frontend-vue/`）
- **统一导航源**：新增 `src/nav.ts` 作为菜单 / 路由 / 面包屑的**单一来源**，`Sidebar.vue`、`Topbar.vue`、`router/index.ts` 对齐（commit `19709be` + 文案修正 `ad92282`）。
- **抽公共逻辑**：`src/composables/useAutoRefresh.ts`（自动刷新，原 6 页约 40 行重复逻辑）、`src/utils/status.ts`（状态映射）、`src/__tests__/nav.test.ts`（导航单测）。
- **清理**：删除废弃的 `metaclouds-frontend/.github/workflows/ci-rust.yml`（164 行）。

### 后端 / K8s（`metaclouds-backend-rust/k8s/`）
- **PostgreSQL 多副本 HA 翻转**（`5614e3b`）：`01-configmap.yaml`、`05-deployment.yaml`（replicas 3 + RollingUpdate）、`07-hpa.yaml`（2..10）、`08-pdb.yaml`（minAvailable 2）、`13-kustomization.yaml`（镜像 `newTag` 由占位改为 `main`）、**删除** `19-backend-data-pvc.yaml`；新增 `scripts/deploy-k8s.sh`（`GIT_SHA` 钉不可变 tag）。
- **运行期前置测试数据化**（`9f1d7af`）：新增 `scripts/provision-test-secret.sh`、`scripts/deploy-cluster.sh`、`docs/k8s-provision-with-test-data-2026-10-12.md`；修正 `17-postgresql.yaml` 陈旧注释（仅注释，resources/data 未动）。

### CI（`.github/workflows/ci-cd.yml`）
- **静态把关前移**（`6fc914c`）：`validate-k8s` job 现运行仓库内 `verify-k8s-rules.js`，并安装 kubectl 客户端对两脚本做 `bash -n` / `--check-keys` / `--dry-run`（客户端，无需集群）。
- **规则源统一**（`e2e4b78`）：内联 Python 仅保留 YAML 结构 / namespace 校验（rule 1），镜像层级命名 + kustomize 引用（rules 2&3）统一交由 `verify-k8s-rules.js`，消除双源漂移。

### 文档
- **新增 `docs/k8s-production-switch-runbook.md`**（`174d967`）：生产切换可执行步骤，覆盖 kustomize 占位覆盖 + env 不热更新两大坑、路径 A/B 决策、真实凭据生成、钉版本、验证、回滚、PG HA、安全合规清单；并在测试剧本第六节加指针统一单源。

## 三、关键决策与亮点

- **镜像名统一**：`ghcr.io/<owner>/<repo>/backend-rust|frontend`，kustomization `images.newTag` 由 `placeholder` → `main`，根除 `ImagePullBackOff`。
- **运行期前置 P0 修复**：`deploy-cluster.sh` 在回填测试 secret 后增加 ④.7 `rollout restart deployment/metaclouds-backend`，解决 env 形式 `secretKeyRef` 注入**不热更新**导致的 `/health` 验证失败。
- **CI 把关前移**：运行期前置脚本在流水线以 `--dry-run=client` 静态校验，避免带病 `apply`。
- **规则源单一化**：镜像 / kustomize 引用校验只由 `verify-k8s-rules.js` 一处负责，避免"改了一处另一处仍放过"。

## 四、验证

- **前端**：`nav.test.ts` 单测通过；`type-check` / `build` 通过（CI 阻断项）。
- **后端**：`cargo fmt --check` / `clippy -D warnings` / `test --all-features`；PostgreSQL 双驱动（`--ignored`）冒烟 + 集成测试（JSONB / TIMESTAMPTZ / 软删除 / UPSERT）。
- **K8s / CI**：`verify-k8s-rules.js` → PASSED（rules 2&3）；`validate-k8s` 内联 Python 实跑 → `K8s manifest validation PASSED: 23 resources in 19 files`；`bash -n` 两脚本 OK；`--check-keys` 6 key 全覆盖；YAML 解析通过。
- **安全**：gitleaks / checkov 扫描；CI "Guard - reject placeholder secrets" 拒绝 `CHANGEME`；真实凭据不落 Git（`02-secret.yaml` 占位保留为生产基线）。

## 五、风险与回滚

- **PG 单副本**：`17-postgresql.yaml` 当前为单副本 StatefulSet，无流复制 / HA。**生产必须**换 CloudNativePG / Zalando Operator / 云托管（runbook 第 8 节）。
- **部署层翻转**（A1）已在 `5614e3b` 落地：`USE_SQLITE=false` + replicas 3 + HPA 2..10 + PDB minAvailable 2。
- **回滚**：任一提交可 `git revert`；K8s 回滚用 `GIT_SHA=<prev-sha> ./scripts/deploy-k8s.sh` 钉上一已知良好镜像；或 `kubectl delete -k`（PG PVC `data-postgresql-0` 默认保留，需手动删）。

## 六、待办 / 后续

- **生产切换**：按 runbook 路径 A 把 `13-kustomization.yaml` 中的 `- 02-secret.yaml` 移除，使 Secret 脱离 kustomize 独立管控（根除占位覆盖）。
- **PG HA 落地**（生产）。
- **真实凭据替换**：CI Guard 已就绪，按 runbook 第 2 节注入真实值。

## 七、提交清单

| Commit | 类型 | 范围 | 摘要 |
| --- | --- | --- | --- |
| `19709be` | feat | frontend | 统一导航源 `nav.ts` + 菜单子模块优化 + 工程化收尾（删废弃 CI、加单测） |
| `5614e3b` | feat | k8s | 后端切 PostgreSQL 多副本 HA，统一镜像名消除 ImagePullBackOff |
| `ad92282` | fix | frontend | `/tenant` 面包屑文案对齐单一来源 `nav.ts` |
| `9f1d7af` | feat | k8s | 测试数据部署脚本 + 执行剧本 + 修正 `17-postgresql` 注释 |
| `6fc914c` | ci | k8s | `validate-k8s` 纳入 `verify-k8s-rules.js` 与两脚本 static gate |
| `e2e4b78` | ci | k8s | 统一规则源，内联 Python 仅保留结构校验 |
| `174d967` | docs | k8s | 新增生产切换 runbook，指向测试剧本 checklist |

> 注：本文件为 PR / 发布说明草稿，不属于项目源码，未纳入版本控制（如需归档可单独提交）。
