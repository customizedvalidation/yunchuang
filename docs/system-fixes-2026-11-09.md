# 2026-11-09 第二轮修复实施报告（含第三轮回标）

- 日期：2026-11-09（下午出具；同日第三轮换新第四/五节并新增第七节）
- 对应复盘：`docs/system-review-and-optimization-2026-11-09.md`（同日 12:45 出具，第五节/第六节为**修复前**口径，状态以本文为准）
- 基线：`2be6ebe`（复盘时工作区干净）；第二轮改动已提交为 `e3eacda` / `93356e9` / `2bcff5b` / `2b2360e`，第三轮改动见第七节
- 范围：Rust 后端、Vue3 前端、compose / 19 份 K8s 清单 / nginx / Ingress、CI 流水线、文档
- 定位：**S0 止血全部完成 + S1 的多副本语义部分完成 + S2 的 CI 真实性修复落地；第三轮补齐运维前置条件与阻塞项台账**
- 当前部署口径真源：`docs/deployment-prerequisites.md`（第四节的旧「推送前必须配齐」要求已被显式开关取代）

---

## 一、结论先行

本轮把复盘里「名实不符」的三个 P0 从「只有读源码才知道的隐含事实」变成「配置层不可隐式启用、代码层有守卫、接口层可查询、界面层有提示、数据层可区分」的显式状态。

**但根本事实没有改变**：作业执行体仍是 `tokio::time::sleep` 后直接置 `completed`，实耗仍是 `申请 × 0.75` 的合成值，K8s 页面仍是 `MockK8sClient`。本轮修的是「不能把假的当真的用」，不是「把假的换成真的」。接真实执行器仍需 K8s 集群 / GPU / DCGM 与系统定位拍板，未启动。

| 目标 | 结果 |
|---|---|
| 生产误用模拟执行 | ✅ 启动 fail-closed 拒绝；演示集群须显式 `ALLOW_SIMULATED_EXECUTION=true` |
| 用户被模拟数据误导 | ✅ `/api/v1/system/mode` + 常驻演示模式横幅 |
| 多副本互踩 / 卡 running | ⚠️ 部分：启动期 reconcile 唯一 leader、全局并发上限、SIGTERM 排水、完成幂等已修；**执行任务仍未 DB 持久化**，进程崩溃仍会丢在途任务 |
| CI 恒绿 | ✅ 占位域名 / `if: always()` / `curl || echo Warning` 全部替换为真实断言；镜像扫运行时镜像、SHA 钉版本 |
| 配置与文档漂移 | ✅ 双驱动注释、`K8S_*` 语义、撤销名单默认值、版本 / PoC 描述、roadmap 状态回标 |

---

## 二、逐项修复清单

### P0-1 让「模拟」显式且不可误用

| 文件 | 改动 |
|---|---|
| `src/config.rs` | 新增 `allow_simulated_execution`（默认 `false`，env `ALLOW_SIMULATED_EXECUTION`）。`SERVER_ENV=production` 且未显式承认时，只要 `dispatch_enabled=true` **或** `k8s_simulation_mode=true`，`validate()` 直接拒绝启动；错误信息给出「显式承认演示模式」或「关闭模拟路径」两条解除路径 |
| `src/main.rs` | 派发循环启动输出 WARN，写明「模拟执行 / 合成实耗 / 不得当作真实算力交付」，带 3 个配置值 |
| `src/handlers/system.rs`（新） | 公开只读 `GET /api/v1/system/mode`（无 JWT、不含敏感信息），返回 `simulated` / `execution_mode` / `k8s_data_source` / `usage_samples_synthetic` / `notice` |
| `src/routes.rs` | 把 `/system/mode` 注册进 public router，登录页也能判定实例模式 |
| `src/handlers/k8s.rs` | `K8S_SIMULATION_MODE` 从「零业务消费」变为真实语义：新增 `ensure_mock_allowed`，4 个只读 handler 全部前置校验；`true` 明确返回 mock，`false` 在真实客户端接入前 **fail-closed 报错**，不再用 mock 冒充真实集群 |
| `src/components/ModeBanner.vue`（新） | 常驻横幅，`onMounted` 拉 `/api/v1/system/mode`；端点不可用静默降级并留控制台告警 |
| `src/App.vue` | 在 `el-config-provider` 内挂载横幅，覆盖登录页与全部业务页 |
| `src/components/__tests__/ModeBanner.spec.ts`（新） | 回归测试：模拟模式展示文案、非模拟模式不展示、端点失败降级等 |
| `k8s/01-configmap.yaml` | 显式声明 `ALLOW_SIMULATED_EXECUTION=true` / `DISPATCH_ENABLED=true` / `DISPATCH_SIMULATED_RUN_MS=2000` / `DISPATCH_SIMULATED_UTILIZATION=0.75`；`K8S_SIMULATION_MODE` 由虚假的 `false` 改为如实的 `true` |
| `docker-compose.yml` | 补齐同一组执行模式变量（含 `REVOCATION_FAIL_CLOSED`），`K8S_ENABLED` 与 ConfigMap 对齐为 `false`；避免 CI 用 `SERVER_ENV=production` 部署时因未承认演示模式而启动失败 |
| `tests/config_test.rs` | 新增 5 个门禁用例：生产拒绝隐式模拟派发 / 拒绝隐式 mock K8s / 关闭两条模拟路径可正常启动 / 显式承认可通过 / development 不受限 |

> ⚠️ **破坏性变更**：生产环境升级后若未同步下发 `ALLOW_SIMULATED_EXECUTION=true`（演示集群）或 `DISPATCH_ENABLED=false`（关闭派发），后端会**拒绝启动**。这是设计意图，不是故障。

### P0-2 多副本调度语义（部分修复）

| 文件 | 改动 |
|---|---|
| `migrations/014_dispatch_lease.sql` + `migrations/postgres/014_dispatch_lease.sql`（新） | 新增 `dispatch_lease` 表（双驱动同构，RFC3339 文本时间，字典序即时间序） |
| `src/services/dispatch.rs` | ① `acquire_reconcile_lease`：TTL 租约选唯一 leader，3 副本只有一个跑启动期崩溃恢复，过期可重抢；② `try_dispatch_one`：把 `max_concurrent` 条件放进 **同一条原子 UPDATE 的子查询**，消除「先 COUNT 再认领」的 TOCTOU（此前 3 副本实际全局上限 = 3×10）；③ `DispatchLoop` / `spawn_tracked` / `INFLIGHT_TASKS` / `shutdown(grace)`：SIGTERM 先停接新单 → 等在途任务收尾（≤30s）→ 停主循环；④ `complete_running_job` 前置 `status=="running"` + 末尾 `rows_affected` 双重幂等 |
| `src/main.rs` | 关闭路径由 `h.abort()` 改为 `d.shutdown(30s).await` |
| `k8s/05-deployment.yaml` | `terminationGracePeriodSeconds` 30 → 45，给排水留余量 |
| `migrations/015_usage_source.sql` + PG variant（新） | `jobs.usage_source TEXT NULL`，采样写入 `'simulated'`；**计费/showback 必须显式过滤该列**，否则会对从未发生的算力出账 |

> ⚠️ **未修完**：执行任务仍是 `tokio::spawn` 内存态，没有 DB 持久化、worker lease/心跳、reconcile owner 幂等。进程被 SIGKILL 时，在途作业仍会卡在 `running`，只能等 300s 宽限期回退。这需要 S1 的真实执行器一并设计。

### P0-3 CI 门禁可信度

| 文件 | 改动 |
|---|---|
| `scripts/verify-deployment.sh`（新） | 部署后真实断言：`/health/ready`（查库）最多 5 次重试 + **校验响应必须是后端 JSON 信封** + `/api/v1/system/mode` 信封与字段校验；任何一步失败非 0 退出 |
| `metaclouds-frontend-vue/nginx.conf` | 新增 `location /health/` 反代到 `backend:8000`，置于 SPA 回退之前。否则 `/health/ready` 会被 `try_files` 回退成 `index.html` 并返回 200，验收脚本拿到 HTML 却以为后端就绪——这是本轮实测发现并补掉的**第二处假绿** |
| `k8s/10-ingress.yaml` | 新增 `/health` path → backend Service，与 nginx 反代同语义；此前该路径会命中 `/` 落到前端 |
| `.github/workflows/ci-cd.yml` | ① `verify-deployment` 去掉 `if: always()` 与占位域名 echo，改为 `needs: [deploy-staging]` + `result == 'success'`；② 三个 `deploy-*` 加 Guard，缺 BASE_URL / SSH 凭据直接 `exit 1`，不再「跳过也算绿」；③ 部署统一拉 GHCR 镜像 `IMAGE_TAG=sha-<commit>`，删除目标机 `cargo build --release && systemctl restart`；④ Trivy 由 `scan-type: fs` 改为扫描**已推送的运行时镜像**，带 `exit-code: 1`；⑤ 15 个 job 去掉 `needs: security-scan` 单点扇出，安全门禁收敛到镜像构建/部署前；⑥ `type=sha,format=long` + `kustomize edit set image` 钉 SHA + 部署后断言实际镜像 tag；⑦ K8s health 检查由 `curl || echo Warning` 改 5 次重试 + 失败 `exit 1`；⑧ 未配置 `KUBE_CONFIG` 时不再报「skipped success」而是 `exit 1` |
| `docker-compose.yml` | `backend` / `frontend` 补 `image: ...:${IMAGE_TAG:-main}`；`USE_SQLITE` / `DATABASE_URL` 变量化，消除「SSH 源码构建用 SQLite、K8s 镜像用 PG」的漂移 |
| `k8s/17-postgresql.yaml` + CI | StatefulSet 注解 `metaclouds.io/production-ha: "false"` 显式标记单点；CI 增 PG HA guard（默认 warning，`REQUIRE_PG_HA=true` 可升为硬门禁） |

### P1 / P2 残余

| 项 | 改动 |
|---|---|
| P1-4 撤销名单 fail-open | `revocation_fail_closed` 默认 `false` → `true`；新增 `cache::revocation_store_ever_active()` 区分「从未部署 Redis（无缓存降级，不放 503）」与「曾接入后整体失效（返回 503，不放行已撤销 JWT）」，避免翻转默认值把未部署 Redis 的环境打成全站不可用 |
| P1-3 配置矛盾 | ConfigMap `K8S_ENABLED=false` 与 `automountServiceAccountToken: false` 对齐；注释写明接入真实客户端时三者（开关 / token / RBAC）必须一起变更 |
| P1-7 折叠分组（第一轮） | `src/plugins/element-plus.ts` 补 `ElCollapse` / `ElCollapseItem` 深路径导入、样式与注册，修复生产作业表单折叠分组不渲染 |
| P2-1 双调度实现 | 新增 `legacy-scheduler` feature，647 行纯内存 mock 默认**不参与编译**；`priority_scheduler_test.rs` 加 `#![cfg(feature = "legacy-scheduler")]`；`Cargo.toml` version `0.1.0` → `2.0.1`、description 去 PoC，改为生产后端描述 |
| P2-3 backlog 3 项 | ① 核验确认 `reconcile` 回退前已由 `release_quota_for_job` 等量释放 CPU/内存/GPU，**历史描述为摘除分支时的中间态，非现存缺陷**；② `select_topology_node` 在 `fraction<1 && slice_cap>1` 时按设备剩余容量可容纳的份额槽位数计数，新增正反两向回归用例；③ 完成幂等见 P0-2 |
| P2-4 | `usage_source` 标记合成实耗，见上 |
| P2-6 | README 新增「Authorization boundary」章节；`docs/metaclouds-optimization-roadmap.md` 加状态回标横幅并逐项回写 P2 backlog |

---

## 三、实测门禁（本地，2026-11-09）

工具链 `cargo` / `rustc` 1.96.0，与 CI 一致。离线 Windows 沙箱；不依赖网络与 runner 的项目全部实跑。

| 层 | 命令 | 结果 |
|---|---|---|
| 后端 | `cargo fmt --all -- --check` | ✅ FMT_OK |
| 后端 | `cargo clippy --all-targets -- -D warnings` | ✅ 退出 0，零 lint；仅 Windows 沙箱增量锁文件删除失败的 `os error 5` 环境噪声 |
| 后端 | `cargo check --all-features --quiet` | ✅ ALL_FEATURES_PASS |
| 后端 | `cargo test --quiet` | ✅ 全套通过（lib + 集成测试；含 5 个新配置门禁用例） |
| 后端 | `cargo test --features legacy-scheduler --quiet` | ✅ LEGACY_FEATURE_PASS（可选 feature 单独编译路径可用） |
| 前端 | `npm run type-check` | ✅ 0 类型错误 |
| 前端 | `npm run test` | ✅ **12 文件 / 60 用例全绿**（新增 ModeBanner 用例；此前 `JobManagement.spec.ts` 的偶发超时经单独与全套复验均为资源争抢噪声，本次 8.4s 全过） |
| 前端 | `npx vite build --outDir <全新目录>` | ✅ 2,378 modules 成功，element-plus chunk 951.37 kB，无打包回归 |
| 脚本 | `bash -n scripts/verify-deployment.sh` | ✅ 语法通过 |
| 脚本 | 本地 mock 后端双场景实测 | ✅ 真实就绪 JSON → `PASSED` / exit 0；**SPA 回退 HTML + HTTP 200 → 5 次重试后 exit 1 正确拒绝** |
| 静态 | CI YAML 依赖图 / 表达式 / 缩进 | ✅ 15 个 job、无悬空 `needs`、`${{ }}` 85/85 配对、0 tab；4 份 K8s 清单 + compose 0 tab |
| 静态 | `10-ingress.yaml` `/health` 与 `/api` path | ✅ 均存在且指向 backend Service |

> **本地绿 ≠ CI 绿**：`cargo audit`、`postgres:16` 双驱动集成、Checkov、Trivy 镜像扫描、真实 K8s apply 均需 GitHub runner 与网络，本地无法等价覆盖。首个完整 run 仍未观察到。

---

## 四、推送前的运维前置条件（第三轮已闭环，口径以 runbook 为准）

> 本节于 2026-11-09 第三轮重写。原文要求「推送前必须一次性配齐全部变量，否则流水线按设计变红」—— 那是把「运维尚未就绪」误报成「代码有问题」。**当前真源：`docs/deployment-prerequisites.md`**；可执行的变量清单与校验规则在 `scripts/check-deploy-prereqs.sh`。

### 4.1 状态机（已落地）

| 状态 | 条件 | CI 表现 |
|---|---|---|
| 未启用部署 | `DEPLOY_ENABLED != true` | 4 个部署 job **skipped**，主构建链路照常绿；日志打印还差哪些变量（`--report` 诊断） |
| 声明启用但没配齐 | `DEPLOY_ENABLED=true` 且 `DEPLOY_TARGETS` 中任一目标前置条件不全 | `deploy-prereqs` **exit 1 变红**，一次报全缺失项 |
| 已配齐 | 目标校验通过 | 仅 `DEPLOY_TARGETS` 列出的目标真正部署，随后仍走 `verify-deployment.sh` 真实断言 |

**结论：推送代码不再被运维配置阻塞**，同时没有退回「跳过也算绿」。总闸开关与 `DEPLOY_TARGETS` 让「部署未发生」在 UI 上明确显示为 skipped。

### 4.2 本轮修掉的运维侧真实缺陷

| # | 问题 | 处理 |
|---|---|---|
| 1 | 4 个 deploy job 各自手写 3 行 Guard，漏检 `*_USER` / `DEPLOY_DIR` / `KUBE_CONFIG` / K8s 真实凭据；只查非空不查格式，占位域名、假 PEM key、假 kubeconfig 都能通过 | 收敛到 `scripts/check-deploy-prereqs.sh` 单一真源（存在性 + URL/PEM/base64/长度/占位符校验），`--list` 自助打印清单；4 个 job 与 CI 总闸共用 |
| 2 | `k8s/02-secret.yaml` 是恒含 `CHANGEME` 的占位模板：旧 Guard 解析它 → 启用 K8s 部署必 100% 失败；`kustomize build \| kubectl apply` 又会把集群里预置的真实 Secret **覆盖回占位值** | 新增 `scripts/render-k8s-secret.sh`，apply 前从 6 个 CI secrets 渲染真实 Secret 工作副本（仅存在于 runner checkout）；Guard 改为校验渲染结果 |
| 3 | 旧 Guard `import yaml` 依赖 runner 预装 PyYAML（该 job 从未安装）、错误分支的 Python 字符串有未转义嵌套引号（语法错误）、并把解码后的凭据明文打到公开日志 | 改纯标准库逐行解析，增加必需 key 完整性检查，**只报 key 名不回显值** |
| 4 | `02-secret.yaml` 注释让运维分别随机生成 `database-url` 口令与 `postgres-password`；`17-postgresql.yaml` 用后者初始化 PG，后端用前者连接，不一致时清单全绿但后端鉴权起不来 | 新增 `scripts/generate-deploy-secrets.sh` 强制两者同源，并用 `--from-env` 保证本地自检值与 `gh secret set` 注入值一致 |
| 5 | 伪前置条件 `K8S_BASE_URL` 被列为必需 | 降为可选：K8s 验收走 `kubectl port-forward`，该地址仅用于 GitHub Environment URL 展示 |
| 6 | `Configure kubeconfig` 把 secret 直接插值进 shell 源码，引号/换行可能破坏命令 | 改为从 env 读取 `printf '%s' "$KUBE_CONFIG" \| base64 -d` |

### 4.3 仍需运维提供的配置（摘要，非推送阻塞）

| 类别 | 变量 / Secret |
|---|---|
| 总闸 | `DEPLOY_ENABLED`、`DEPLOY_TARGETS`、`REQUIRE_PG_HA`（可选） |
| compose | `{DEV,STAGING,PROD}_BASE_URL`（Variable）、`{DEV,STAGING,PROD}_{SSH_KEY,HOST,USER}`（Secret）、`DEPLOY_DIR`（可选） |
| K8s | `KUBE_CONFIG` + 6 个 `K8S_*` 凭据（Secret）、`K8S_BASE_URL`（可选 Variable） |
| 目标机 | GHCR 只读登录、`.env`（含 `postgresql:// DATABASE_URL`）、`git pull --ff-only` 可用、新版 nginx/Ingress |

生成与逐项命令见 `docs/deployment-prerequisites.md`；本地自检：

```bash
bash scripts/check-deploy-prereqs.sh --env-file .env.deploy --report
bash scripts/check-deploy-prereqs.sh --env-file .env.deploy --target staging --strict
bash scripts/pre-push-check.sh --strict
```

### 4.4 上线前必须复核的四件事（未变）

1. 生产 ConfigMap 当前以**演示集群**口径显式承认模拟（`ALLOW_SIMULATED_EXECUTION=true`）。作为真实算力平台交付前必须改为 `false` 并把 `DISPATCH_ENABLED` 设为 `false`（或接入真实执行器后移除模拟路径）。
2. `/health` 反代依赖**新版前端 nginx 镜像**与新版 Ingress；只更新后端镜像会让验收脚本拿不到后端就绪响应。
3. `usage_source='simulated'` 的合成实耗禁止进入计费/showback 查询。
4. 迁移 014/015 为纯增量（建表 / 加列，可空、无外键），双驱动各一份，apply 前确认对应方言目录。

---

## 五、剩余未决与阻塞项台账（第三轮更新）

图例：🔴 需外部条件 / 🟡 设计已定、待环境实施 / 🟢 本轮闭环 / ⚪ 部分完成

| ID | 项 | 状态 | 本轮动作 | 解除条件 | 验收标准 | 关联 |
|---|---|---|---|---|---|---|
| R1 | 真实执行器（kube-rs Job/CRD）、DCGM 实耗 | 🟡 | 成文 `docs/real-executor-contract.md`：`Executor` trait、状态机、灰度路径、7 组验收用例；**未写实现** | 真实 K8s 集群 + GPU/Device Plugin + DCGM + kubeconfig/RBAC；且**系统定位拍板**（演示平台 vs 生产算力平台） | 契约第六节 7 组用例全绿；`usage_source='dcgm'` 进入账本 | `real-executor-contract.md` |
| R2 | 执行任务 DB 持久化 + worker lease/心跳 | 🟡 | 在契约中定稿 `exec_tasks` 表（双驱动）、lease 续约与 attempt 幂等；**未建表、未改代码** | 与 R1 一并实施（共用生命周期，不可拆） | SIGKILL 后另一副本在 TTL 内接管，不重复提交/释放 | `real-executor-contract.md` §3–4 |
| R3 | reconcile owner 运行期幂等 | ⚪ | 启动期已用 014 租约选主；运行期仍未做 | 同 R2 | 契约用例 1/2/6 | `migrations/014_dispatch_lease.sql` |
| R4 | P2-3 抢占分支回归与合并 | 🟡 | 新增 `docs/p2-3-preemption-backlog.md`：2 处 major、基线漂移清单、rebase 顺序、7 组必补测试、验收标准 | 建议与 R1 一并立项（在内存态模拟作业上验证「模拟互抢」价值有限） | 回归清单第五节全绿，默认关闭开关 | 分支 `park/p2-3-preemption` @ `d3ea543` |
| R5 | PostgreSQL HA | 🔴 | 已标注单点 + CI 可配置硬门禁；本轮在 runbook 指向切换手册 | 托管 PG / CloudNativePG + 更新 `DATABASE_URL`；设 `REQUIRE_PG_HA=true` 升为硬门禁 | PG 故障切换演练通过，后端 3 副本不断服 | `docs/k8s-production-switch-runbook.md`、`17-postgresql.yaml` |
| R6 | Dockerfile 基础镜像钉 digest | 🟡 | **第四轮口径修正**：第三轮的 ARG 参数化被回退（checkov 解析不了 `${ARG}` 报 CKV_DOCKER_7，且 CI 从未消费该机制）；现为字面 tag + `pin-base-images.sh --check/--resolve/--apply`，CI 两个 build job 新增可选 `BASE_IMAGE_DIGESTS` 注入步骤 | 联网环境 `--resolve` 生成 digest 并设为 GitHub Variable `BASE_IMAGE_DIGESTS` | `--check` 通过（FROM 字面 tag）；设 Variable 后构建的镜像 FROM 带 `@sha256:` | 两份 Dockerfile、`pin-base-images.sh`、`.github/workflows/ci-cd.yml` |
| R7 | CI 首个完整 run | 🟡 | **首个完整 run 已出结论：failure（4 个 job 红）**。第四轮逐一定位并修复 3 个 job 的确凿根因（checkov CKV_DOCKER_7 / kustomization 文件名死结 / tinypool critical），rustls 升 0.23.46 + cargo-audit 步骤拆分与 zip 显式豁免（R11）；详见第八节 | 推送本轮修复触发新 run，观察是否全绿；仍红则按新拆分的步骤名精确定位 | 全 job 实跑；主构建链路绿且 deploy job 状态语义正确 | 第八节、`pre-push-check.sh` |
| R8 | 生产 Secret 注入 | 🟡 | 本轮打通路径：`render-k8s-secret.sh` + `generate-deploy-secrets.sh` + 渲染后 Guard；**真实值仍须运维提供** | 运维注入 6 个 K8s secrets 或改用 External Secrets Operator / Sealed Secrets | Guard 校验渲染 Secret 无占位；后端正常连库/签发 JWT | `docs/deployment-prerequisites.md` §2.3/§4 |
| R9 | K8s 客户端仍是 `MockK8sClient` | 🔴 | 第二轮已让 `K8S_SIMULATION_MODE=false` 时读端点 fail-closed 拒绝 mock，不再冒充真实集群 | 与 R1 同（真实集群 + kube-rs 客户端） | `/api/v1/system/mode` 返回 `simulated=false`；节点/Pod 数据来自 API Server | `src/handlers/k8s.rs` |
| R10 | 生产仍在跑模拟执行 | 🔴 | 第二轮已显式化并有 fail-closed；本轮在 runbook 列为上线复核第 1 项 | R1 完成 + 关闭 `ALLOW_SIMULATED_EXECUTION` / `DISPATCH_ENABLED` | 生产 `validate()` 通过且模式接口不再报模拟 | `src/config.rs`、`01-configmap.yaml` |
| R11 | zip 3.0.0（build-dep）命中 4 条 RUSTSEC（1 critical / 2 high / 1 medium） | 🟡 | 第四轮：cargo audit 显式 `--ignore` 4 条并注明理由（zip 仅是 utoipa-swagger-ui 的 build-dep，解压上游受信资源，无运行时暴露面） | utoipa 5→6 + utoipa-swagger-ui 9→10（10.x 要求 `zip ^9`、`utoipa ^6`）连带升级，须在可编译验证环境执行后移除 `--ignore` | `cargo audit` 输出无 zip 相关项且 CI 中无 `--ignore RUSTSEC-2026-03{67,68,69}/0429` | `Cargo.toml`（utoipa=5, swagger-ui=9）、`.github/workflows/ci-cd.yml` |

### 5.1 阻塞依赖链

```
系统定位拍板 ──> 真实集群/GPU/DCGM ──> R1 ──> R2 / R3 / R9 / R10
                                    └──> R4（抢占回归，建议同批）
推送第四轮修复 ──> 新 CI run ──> R7（首个 run 已 failure，根因 4 取 3 已修）──> 全绿后按需打开 DEPLOY_ENABLED
utoipa 6 可编译验证环境 ──> R11（移除 zip 豁免）
运维提供真实凭据 ──> R8 ──> K8s 部署链路可用（R5 仍为单点告警）
联网环境 resolve digest ──> R6（设 BASE_IMAGE_DIGESTS）
```

> 第四轮把「R7 结论待观察」推进为「已观察到 failure 且 4 个失败 job 的根因 3 个已实测修复、1 个（cargo audit）双管齐下处理」。**没有任何一项被伪装成已完成**：执行体依旧是 `sleep(2s)`，实耗依旧是合成值，K8s 页面依旧是 mock。

---

## 六、交付物与改动面

- 第二轮（已提交 `e3eacda` / `93356e9` / `2bcff5b` / `2b2360e`）：约 30 个文件，含 8 个新增文件（4 份迁移、`handlers/system.rs`、`ModeBanner.vue`、`ModeBanner.spec.ts`、`scripts/verify-deployment.sh`）。
- 复盘与报告：`docs/system-review-and-optimization-2026-11-09.md` / `.html`（问题证据链与路线图）、本文件。
- 第三轮（本轮）交付物：
  - 脚本：`check-deploy-prereqs.sh`（单一真源预检）、`render-k8s-secret.sh`（真实 Secret 渲染）、`generate-deploy-secrets.sh`（凭据同源生成）、`pin-base-images.sh`（digest 解析/检查）、`pre-push-check.sh`（推送前自检）。
  - 文档：`docs/deployment-prerequisites.md`（runbook）、`docs/ci-deployment-secrets.md`（重写为索引）、`docs/real-executor-contract.md`（执行器契约）、`docs/p2-3-preemption-backlog.md`（回归清单）、本文件第四/五/七节。
  - 配置：`.env.deploy.example`、`.gitignore`（放行模板 + 说明）、两份 Dockerfile 的 4 个基础镜像 ARG、`docs/metaclouds-optimization-roadmap.md` 状态回标。
  - CI：新增 `deploy-prereqs` 总闸、4 个 deploy job 改用统一 Guard、K8s Secret 渲染与渲染后 Guard、kubeconfig 改从 env 读取。

---

## 七、第三轮实施记录（运维前置条件 + 阻塞项台账化）

### 7.1 目标

把第四节「推送前必须一次性配齐运维变量」从一句文档要求变成**可执行、可诊断、不会误报红**的机制；把第五节「剩余未决」从 8 行表格变成有 ID、状态、解除条件、验收标准与依赖链的台账。

### 7.2 关键决策

| 决策 | 理由 |
|---|---|
| 用 `DEPLOY_ENABLED` / `DEPLOY_TARGETS` 显式开关，未启用时部署 job **skipped** | 既保留 fail-closed，又不把「还没部署」伪装成绿、也不把「运维没就绪」误报成代码失败 |
| 前置条件收敛到 `scripts/check-deploy-prereqs.sh`，本地与 CI 共用 | 消除 4 份手写 Guard 的漂移；`--list` 与文档同源 |
| 校验格式而非只校验非空 | `example.com` 占位域名 / 假 PEM / 非 kubeconfig 的 base64 此前都能通过 |
| CI 渲染真实 K8s Secret 工作副本，而非改仓库模板 | 模板必须保持无凭据；渲染产物只存在于 runner checkout |
| 基础镜像只参数化、不硬写 digest | 离线沙箱无法解析 digest，脚本明确报错而不是伪造 |

### 7.3 实测门禁（本地，2026-11-09）

本轮**未改动 Rust / TypeScript 源码**（仅 Dockerfile、脚本、YAML、文档），故未重跑后端/前端构建门禁；改动面用下列静态与脚本实测覆盖。

| 层 | 场景 | 结果 |
|---|---|---|
| 脚本语法 | 6 个新脚本 `bash -n` | ✅ 6/6 通过 |
| 预检 · 本地 | `--report`（配置全空） | ✅ exit 0，只报告不阻塞 |
| 预检 · 本地 | `--strict --target staging`（全空） | ✅ exit 1，4 项 `[FAIL]` 一次报全 |
| 预检 · 本地 | staging 齐备 | ✅ exit 0（5 checks / 0 failure） |
| 预检 · 格式 | 占位 URL `dev.example.com` / 假 SSH key | ✅ 均 exit 1，分别精确指出字段 |
| 预检 · 本地 | k8s 齐备（含 base64 kubeconfig） | ✅ exit 0（7 checks / 0 failure） |
| 预检 · CI | `DEPLOY_ENABLED` 未设 | ✅ exit 0，`GITHUB_OUTPUT` 仅 `enabled=false`，3 条 notice |
| 预检 · CI | 启用但配置空 | ✅ exit 1，4 项 `[FAIL]` |
| 预检 · CI | 启用 + staging 齐备 | ✅ exit 0，输出 `staging=true` / `enabled=true` |
| 预检 · CI | `DEPLOY_TARGETS=production`（无 staging） | ✅ exit 2，指出审批门依赖 |
| 预检 · CI | 两个目标、k8s 缺配置 | ✅ exit 1，7 项 `[FAIL]` 一次报全 |
| Secret 渲染 | 缺 6 个凭据 / `--dry-run` 齐备 / 真实渲染 | ✅ exit 1 / exit 0 / exit 0（6 keys，无占位） |
| 渲染后 Guard | 对渲染产物 / 对仓库占位模板 | ✅ PASS（6 keys）/ **FAIL（6 个占位 key）**，证明有区分力 |
| 凭据生成 | 6 个值、口令同源、`--from-env --gh-commands` | ✅ 6 key；`database-url` 口令 == `postgres-password`；输出 6 条命令 |
| digest 检查 | `--check` / `--check --strict` / `--resolve`（离线） | ✅ 0/4 钉死并告警 / exit 1 / exit 3（明确不伪造 digest） |
| 推送自检 | `pre-push-check.sh --report` | ✅ exit 0，检出工作区未提交 + ahead 15；`git ls-remote origin` 连通 |
| CI YAML | 静态依赖图 / 表达式 / outputs | ✅ 16 jobs、`${{ }}` 119/119 配对、0 tab、5/5 outputs 声明、所有 `needs.*.outputs` 引用均有对应 `needs` |
| Dockerfile | ARG 声明位置与 FROM 引用 | ✅ 两份共 4 个 ARG，全部声明在首个 `FROM` 之前且被正确引用 |

实测中修掉的 3 个脚本缺陷：① `GITHUB_OUTPUT` 同名 key 重复写入（改为只写最终值）；② 渲染自检对全文 grep `CHANGEME` 被注释误触发、计数正则把 `namespace: metaclouds` 算成第 7 个 data key（改为只解析 `data:` 段并逐个 base64 解码校验）；③ Windows/NTFS 不执行 POSIX `chmod`，生成脚本现在检测并告警而不是声称权限已生效。

### 7.4 受环境限制、本轮未等价覆盖

- GitHub runner 上的首个完整 CI run —— 推送已于本轮完成并触发（run `ee08262` 状态 `in_progress`），**结论待观察**；本地无法等价覆盖 runner 上的工具链与联网步骤。
- `docker build` 对新增 ARG 的实际兼容性 —— 沙箱无 Docker；已做静态检查（ARG 位置/FROM 引用），未做镜像构建。
- 真实集群、GPU、DCGM、托管 PG —— 台账 R1/R2/R5/R9 的解除条件，本轮无变化。

### 7.5 结论

**推送不再被运维配置阻塞**：未启用部署时 CI 明确显示 skipped，主构建链路保持绿；声明启用却缺配置才按设计变红并一次报全。K8s 部署链路的两个死结（占位模板必失败、apply 覆盖真实 Secret）与口令不同源陷阱已闭环。剩余阻塞项 R1–R10 全部有可执行解除条件与验收标准，但**核心事实未变**：执行体仍是 `sleep(2s)`、实耗仍是合成值、K8s 仍是 mock —— 本轮没有把任何「未做」写成「已完成」。

### 7.6 推送与远端对齐（本轮收尾实测）

| 项 | 结果 |
|---|---|
| `git push origin main` | ✅ `2c13f7c..ee08262  main -> main`，一次推送送达此前积压的 16 个提交 |
| 同步核验 | ✅ `git fetch` 后 `origin/main == main == ee08262`，ahead 0 / behind 0，`git status` 干净 |
| GitHub Actions | ✅ 已触发 `Metaclouds CI/CD Pipeline` run（head `ee08262`，event `push`），状态 `queued → in_progress`；**结论尚未产生** |
| 历史对照 | ⚠️ API 显示此前 4 次 run（10-29 / 10-30）全部 `failure`，与「首个完整 run 从未观察到」一致；本次是首次有机会看到完整流水线 |
| 离线兜底 | ✅ `.workbuddy/yunchuang-main-20261109.bundle` 更新至 `ee08262`（2.3 MB，`git bundle verify` OK，记录完整历史） |

> CI 结论、以及 `deploy-prereqs` 在 runner 上的真实行为，需在 GitHub 上观察后回填；本地静态与脚本实测不能替代 runner 结果。


---

## 八、第四轮（2026-11-10）：首个完整 CI run 的失败根因与修复

7.6 中触发的历史 run（`ee08262` / `59f2827`，含 16+1 个积压提交）已于 11-09 晚间出结论：**failure**。通过 check-runs annotations API（匿名可读）拿到失败注解原文，并在本地等价复现，本轮修复其中 **4 个失败 job 中的 3 个**的确定根因。

### 8.1 失败清单与根因（全部实测确凿）

| 失败 job | 失败步骤 | 根因（证据） | 修复 |
|---|---|---|---|
| Security Pre-Scan | checkov | `CKV_DOCKER_7` ×2（annotation 原文）：第三轮把 FROM 改为 `${ARG}`，checkov 静态解析不了变量 → 误判 latest | FROM 回退字面 tag；digest 钉死改为 `pin-base-images.sh --apply` 构建前改写工作副本（CI 两个 build job 新增可选注入步骤） |
| Validate K8s Manifests | deploy-cluster.sh --dry-run | ① `kubectl apply --dry-run=client` 仍需 API server discovery（本地 kubectl 1.31 无 kubeconfig 复现：`failed to download openapi: dial tcp [::1]:8080`）② **更深层死结**：`13-kustomization.yaml` 不是 kustomize 规范文件名，`apply -k <dir>` / `kustomize build` 根本不认——意味着真实 K8s 部署链路从未真正可用 | 文件重命名 `kustomization.yaml`（git mv）+ dry-run 改为 `kubectl kustomize` 全量客户端渲染 + 渲染结果结构断言（kind/apiVersion/metadata.name/禁 latest）；`commonLabels` 弃用警告一并修为 v5 `labels` 语法。**首次**渲染成功：22 个 K8s 对象 |
| Frontend Test & Build | npm audit (critical) | `tinypool@1.1.1` 命中 CVE-2026-104848/104849（critical，RCE gadget；GitHub Advisory API 比对 package-lock 确认是唯一命中） | `package.json` 增加 `overrides: {"tinypool": "^2.2.0"}` 强制脱离 vitest 2.1.9 的 `^1.0.1` 约束；本地实测 60/60 单测 + vue-tsc + vite build 全绿 |
| Rust Lint & Test | cargo audit | 无 annotation 详情（普通 run step 只报 exit 1）。RustSec advisory-db 比对 `Cargo.lock`：rustls 0.23.45 命中 RUSTSEC-2026-0346..0349（patched ≥0.23.46）；zip 3.0.0 命中 RUSTSEC-2026-0367/0368/0369/0429（patched ≥9.0.0）。也可能是 `cargo install cargo-audit` 编译失败——两者本轮都处理 | rustls 升至 0.23.46（Cargo.lock 手改，checksum 取自官方 index，依赖集与 0.23.45 完全一致已核对；本地已 fetch 并编译）；cargo-audit 拆分为 install/audit 两步以便下次精确定位；zip 4 条**显式豁免**（见 8.2） |

### 8.2 zip 豁免的理由与解除条件（R11，禁止新增同类豁免）

zip 3.0.0 是 `utoipa-swagger-ui 9.0.2` 的 **build-dependency**：其 build.rs 解压的是上游受信资源（`utoipa-swagger-ui-vendored` crate 自带的 swagger-ui dist zip），不处理攻击者可控输入，**无运行时暴露面**。升级到 patched（≥9.0.0）需要 utoipa 5→6 + utoipa-swagger-ui 9→10 连带升级（10.0.2 要求 `utoipa ^6.0.0`、`zip ^9.0`，MSRV 1.88），涉及 OpenAPI 宏 API 变化，必须在可编译验证的环境专门执行。豁免以 `cargo audit --ignore RUSTSEC-2026-03{67,68,69} --ignore RUSTSEC-2026-0429` 显式写入 CI（带注释），台账 R11 跟踪解除。

### 8.3 本轮实测（本地等价）

| 项 | 结果 |
|---|---|
| checkov 3.4.3（与 CI 同版本同参数，干净 `git archive` 副本） | ✅ K8s 367 / Dockerfile 147 / github_actions 451 全过，0 失败 |
| kubectl 1.31 无 kubeconfig 复现旧门禁 | ✅ 复现原失败（openapi 下载失败）→ 修复后 `deploy-cluster.sh --dry-run` exit 0 |
| kustomize 渲染 | ✅ 22 docs，labels 26 处注入，无 warning |
| 前端 60 单测 / build | ✅ 全绿（tinypool 2.2.0 + vitest 2.1.9 兼容） |
| rustls 0.23.46 | ✅ 本地 `cargo fetch`（rsproxy 镜像）+ `cargo fmt --check` + `cargo check --all-targets --all-features` + `cargo clippy -D warnings` + `cargo test --all-features` 全部通过（手改 lock 后首次完整本地编译验证） |
| npm critical 全量比对（400 条公告 × lock） | ✅ 唯一命中 tinypool，已 ≥ 2.1.2 |

### 8.4 R6 状态口径修正

第三轮把 R6 记为「ARG 参数化完成」——但该机制 CI 侧**从未被消费**（无 build-arg 引用），且直接导致 CKV_DOCKER_7 失败。本轮回退为字面 tag + `pin-base-images.sh --check/--resolve/--apply`（`--apply` 按 `BASE_IMAGE_DIGESTS` 变量在 CI 构建前改写工作副本，默认未配置则零行为变化）。digest 值仍需联网环境解析（Docker Hub registry 在本沙箱被代理阻断），R6 保持 🟡。
