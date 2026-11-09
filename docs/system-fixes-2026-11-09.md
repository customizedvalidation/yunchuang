# 2026-11-09 第二轮修复实施报告

- 日期：2026-11-09（下午）
- 对应复盘：`docs/system-review-and-optimization-2026-11-09.md`（同日 12:45 出具，第五节/第六节为**修复前**口径，状态以本文为准）
- 基线：`2be6ebe`（复盘时工作区干净），本轮全部改动仍在工作区、未提交
- 范围：Rust 后端、Vue3 前端、compose / 19 份 K8s 清单 / nginx / Ingress、CI 流水线、文档
- 定位：**S0 止血全部完成 + S1 的多副本语义部分完成 + S2 的 CI 真实性修复落地**

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

## 四、推送前必须完成的运维配置（新增前置条件）

修复把「没配置就跳过并报绿」改成了「没配置就明确失败」。因此推送到 `main` 后，若下列变量未配置，流水线会**按设计变红**：

| 类别 | 需要的仓库变量 / Secret | 用途 |
|---|---|---|
| 地址 | `DEV_BASE_URL` / `STAGING_BASE_URL` / `PROD_BASE_URL` / `K8S_BASE_URL` | 替换 `*.example.com` 占位域名，供 `verify-deployment.sh` 断言 |
| SSH | `DEV_SSH_KEY` / `DEV_HOST` / `DEV_USER`、`STAGING_*`、`PROD_*`、`DEPLOY_DIR` | compose 镜像部署；目标机需预置 GHCR 登录凭据与 `.env`（含 `postgresql:// DATABASE_URL`） |
| 集群 | `KUBE_CONFIG`（base64 kubeconfig） | `deploy-k8s` 缺它时直接 `exit 1`，不再报「skipped success」 |
| 可选 | `REQUIRE_PG_HA=true` | 把「内置 PG 单副本」从 warning 升级为阻断门禁 |

其他部署注意：

1. 生产 ConfigMap 当前以**演示集群**口径显式承认模拟（`ALLOW_SIMULATED_EXECUTION=true`）。作为真实算力平台交付前，必须改为 `false` 并把 `DISPATCH_ENABLED` 设为 `false`（或接入真实执行器后移除模拟路径）。
2. `/health` 反代依赖**新版前端 nginx 镜像**与新版 Ingress；只更新后端镜像会让验收脚本拿不到后端就绪响应。
3. `usage_source='simulated'` 的合成实耗禁止进入计费/showback 查询。
4. 迁移 014/015 为纯增量（建表 / 加列，可空、无外键），双驱动各一份，apply 前确认对应方言目录。

---

## 五、剩余未决与阻塞

| 项 | 状态 | 阻塞原因 / 解除条件 |
|---|---|---|
| 真实执行器（kube-rs Job/CRD 等） | 未启动 | 无 K8s 集群 / GPU / DCGM / kubeconfig；且系统定位（演示平台 vs 生产算力平台）未拍板 |
| 执行任务 DB 持久化 + worker lease/心跳 | 未做 | 与上同属 S1，需与真实执行器生命周期一并设计 |
| reconcile owner 幂等 | 部分 | 启动期已用租约选主；运行期幂等未完成认领前仍有窗口 |
| P2-3 抢占分支回归 | 未做 | 保全于 `park/p2-3-preemption`，832 行改动且已知 2 处 major（victim 未按 cluster/partition 过滤、run_token 契约） |
| PG HA | 未做 | 已显式标注单点 + CI 可配置硬门禁；实际切换需托管 PG / CloudNativePG 与 `DATABASE_URL` |
| Dockerfile 基础镜像钉 digest | 未做 | 需逐份核对并写入 digest |
| CI 首个完整 run | 未观察 | `main` 领先 `origin/main` 11 个提交未推送；离线沙箱无 GH 凭据，需手动 `git push origin main` |
| 生产 Secret | 未注入 | `02-secret.yaml` 仍为占位值，`database-url` / `jwt-secret` 需运维提供，缺失时后端 fail-closed 退出 |

---

## 六、交付物与改动面

- 本文件：`docs/system-fixes-2026-11-09.md`（修复实施报告，roadmap 已引用）
- 同日复盘：`docs/system-review-and-optimization-2026-11-09.md` / `.html`（问题证据链与路线图，口径已回标）
- 代码改动：约 30 个文件（含 8 个新增文件：4 份迁移、`handlers/system.rs`、`ModeBanner.vue`、`ModeBanner.spec.ts`、`scripts/verify-deployment.sh`），全部在工作区，尚未提交
