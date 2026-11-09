# Metaclouds 系统全面复盘与优化路线图

> ✅ **状态回标（2026-11-09 下午 · 第二轮）**：本报告出具后，P0-1~3 / P1-1~5 / P2-1 / P2-3 / P2-4 / P2-6 已按 [`docs/system-fixes-2026-11-09.md`](system-fixes-2026-11-09.md) 落地：**S0 止血全部完成**（生产 fail-closed 守卫、`/system/mode` 模式接口、常驻演示横幅、配置显式化）、**S1 多副本语义部分完成**（启动期租约选主、全局并发原子化、SIGTERM 排水、完成幂等）、**S2 的 CI 真实性修复已落地**（真实断言、镜像扫描、SHA 钉版本、部署路径收敛）。下文第五节的门禁记录是**第一轮 6 项优化**的实测，第六节「S0 已执行 4/5 项」与第八节结论均已过时；当前状态与完整实测以修复报告为准。**核心事实未变：执行体仍为模拟**——本轮解决的是「不可误用」，不是「换成真实」。
>
> ✅ **第三轮回标（2026-11-09 傍晚 · 运维前置条件与阻塞台账）**：修复报告第四节原先要求「推送前必须一次性配齐全部运维变量」，本轮改为 CI `deploy-prereqs` 总闸 + `DEPLOY_ENABLED` / `DEPLOY_TARGETS` 显式开关 —— **推送不再被运维配置阻塞**（未启用时部署 job 明确 skipped），声明启用却缺配置时一次报全并变红。`scripts/check-deploy-prereqs.sh` 成为前置条件唯一真源；同时修掉 K8s `02-secret.yaml` 占位模板导致 Guard 必然失败 / `apply -k` 覆盖真实 Secret 的死结，以及 `database-url` 与 `postgres-password` 不同源的陷阱。剩余未决项已台账化为修复报告第五节 R1–R10，配套 `docs/real-executor-contract.md`、`docs/p2-3-preemption-backlog.md`。完整 runbook：`docs/deployment-prerequisites.md`。

- 日期：2026-11-09
- 基线 HEAD：`2be6ebe`（2026-11-09 11:45），工作区干净，`main` 领先 `origin/main` **11 个提交**
- 方法：逐文件核验当前工作区 + 本地实跑门禁，不沿用历史复盘的自述结论
- 范围：Vue3 前端（12,484 LOC / 17 页 / 10 测试文件）、Rust 后端（25,459 LOC / 104 `.rs` / 13 组双驱动迁移）、认证安全、compose / 19 份 K8s 清单 / CI、文档资产
- 可视化版：`docs/system-review-and-optimization-2026-11-09.html`
- 本轮已落地：第一轮 6 项代码/文档优化（见 4.1）+ 第二轮系统性修复（见修复报告），均经前后端门禁复验

---

## 一、核心结论

**工程质量在快速变好，但系统的「真实度」没有跟上能力清单的扩张。**

四轮历史复盘（10-09 / 10-27 / 10-29 / 10-30）中的绝大多数工程债已真实落地：导航三源统一、PG 多副本翻转、CI 安全门禁、调度状态机与配额/GPU 事务化。剩余问题集中在「最后一公里」：真实执行器、CI 端到端可信、配置/文档/代码三者一致性。

| # | 结论 | 判定 |
|---|---|---|
| 1 | 调度闭环是「状态机真实、执行体模拟」的混合体 | P0 |
| 2 | 多副本 HA 架构与内存态执行任务不兼容 | P0 |
| 3 | CI 门禁的「绿」尚未被证明，验证 job 恒成功 | P0 |

---

## 二、头号问题：一条作业链路上的真与假

真实落库：`提交作业 → 配额条件原子 UPDATE（含借用/回收）→ GPU 事务行锁分配 → running → completed`
模拟/伪造：`执行体 sleep(2s)`、`实耗 = 申请 × 0.75`、`MockK8sClient` 的 K8s 页面、P2-5 缺失的计费数据源。

### 证据链

| 项 | 位置 | 事实 |
|---|---|---|
| 执行体 | `src/services/dispatch.rs:479-484` | `tokio::spawn` 内 `sleep(sim_ms)` 后直接 `complete_running_job`；文件头 `:22` 自述「执行体为模拟」 |
| 实耗伪造 | `dispatch.rs:486-501`、`config.rs:254,389` | `UsageSample = 申请 × simulated_utilization`，默认 0.75，写入 `jobs.*_actual`（迁移 013） |
| K8s 假数据 | `src/handlers/k8s.rs:20,31,42,55` | 四处硬编码 `MockK8sClient::new()`；`services/k8s.rs:56` 注明 kube-rs 后续接入 |
| 开关失效 | `config.rs:78/230/361`、`tests/config_test.rs:54` | `K8S_SIMULATION_MODE` **零业务消费**；而 `k8s/01-configmap.yaml:75` 与 `docker-compose.prod.yml:28` 均设 `false` |
| 生产默认开启 | `config.rs:235` | `dispatch_enabled: true`，ConfigMap 未覆盖 → 3 副本各自跑模拟派发 |
| 用户无感知 | `src/` 全量检索 | 无「模拟/演示」UI 标注；归档文档 `PRODUCTION_CONFIG_CHECK_REPORT_FINAL.md:125` 反而写「已禁用模拟模式 ✅」 |

**后果**：生产提交训练作业 2 秒后显示「已完成」，并生成并不存在的利用率账本；Gang / 拓扑 / 时间片 / 双账本超卖回收的效果验证全部建立在模拟数据上。72 个单测验证的是**状态机与账本逻辑**，不是**算力交付**。

---

## 三、问题清单

### P0（须优先处置）

| # | 问题 | 关键证据 | 建议 |
|---|---|---|---|
| P0-1 | 调度「真实闭环」名实不符 | 见第二节完整证据链 | S0 显式化 + fail-closed 守卫；S1 接真实执行器 |
| P0-2 | 多副本与内存态执行冲突 | `main.rs:118-126` 每副本启 loop；`:156-158` 关闭仅 `h.abort()` 主循环，已 spawn 任务丢失致作业卡 running；`dispatch.rs:144-158` `max_concurrent` 为进程内计数（3 副本实际 30）；`:891+` reconcile 无 leader/lease，3 副本可互踩，300s 宽限期内停滞 | 执行任务 DB 持久化 + worker lease/心跳；全局并发计数；reconcile 幂等 |
| P0-3 | CI 门禁可信度 | 路线图 `:159-164` 自述最近 8 次 push 全失败于 `Security Pre-Scan`、13 job 全 skipped；`ci-cd.yml:481-501` `verify-deployment` 用 `if: always()` + 占位域名 curl + 只 echo，恒 success；`:397-410` staging 校验失败仅 warning/skipped；14 job 全部 `needs: security-scan` 单点扇出；Trivy 仅 `scan-type: fs`（:245,306） | 真实断言、拆分 needs、镜像扫描、观察首个跑通的 Rust job |

### P1（交付与安全残余）

| # | 问题 | 证据 | 相对历史 |
|---|---|---|---|
| P1-1 | 部署双路径并存 | `ci-cd.yml:386-395` SSH 源码构建（`git pull && cargo build --release && systemctl restart`）vs GHCR 镜像 + `deploy-k8s`；SSH 路径默认 `USE_SQLITE=true`（`config.rs:323`），K8s 为 false | 遗留 A4 |
| P1-2 | 镜像版本未钉死 | `13-kustomization.yaml:45,47` `newTag: main` 浮动；4 处 Dockerfile 基础镜像未钉 digest | 遗留 |
| P1-3 | K8s 集成自相矛盾 | `05-deployment.yaml:73` `automountServiceAccountToken: false` vs `01-configmap.yaml:73` `K8S_ENABLED: true` + Role；叠加 simulation_mode 无消费者 | 遗留 |
| P1-4 | 撤销名单默认 fail-open | `config.rs:300,446` `revocation_fail_closed: false`；`auth/middleware.rs:148` Redis 故障放行已撤销 JWT | 遗留 |
| P1-5 | PG 单点 | 后端 3 副本 + HPA 2/10 + PDB minAvailable:2，但 `17-postgresql.yaml` 单副本无 HA | 遗留 A1 |
| P1-6 | 角色边界未在文档声明 | `stores/auth.ts:24` localStorage 存 role（既定设计），`router/index.ts:63` fail-closed 守卫正确；本轮已在 README 的 RBAC 章节补「Authorization boundary」小节 | ✅ 已修复 |
| **P1-7** | **作业表单折叠分组在生产不渲染**（本轮新发现的生产缺陷） | 模板共 38 个 `el-*`，唯一注册入口 `src/plugins/element-plus.ts` 缺 `ElCollapse` / `ElCollapseItem` 及样式，页面无局部注册 → 10-29 的 F-7 折叠分组生产不渲染；56 个全绿单测只断言「对话框打开」、未检查内容。本轮补注册与样式，复验告警计数归零、产物仅 +0.03 kB | ✅ 已修复 |

### P2（技术债与可维护性）

| # | 问题 | 证据 | 相对历史 |
|---|---|---|---|
| P2-1 | 双调度实现并存 | `priority_scheduler.rs` 647 行死 mock 与 `dispatch.rs` 2,435 行生产闭环并存；`scheduler/tasks.rs:50-82` skeleton TODO | 新增 |
| P2-2 | P2-3 抢占搁置 | 保全于 `park/p2-3-preemption`，已知 major：victim 未按 cluster/partition 过滤 + run_token 契约 | 新增 |
| P2-3 | P2 backlog 3 项 | reconcile 回退不释放 CPU/内存配额；CoLocate 保守计数误 blocked；`complete_running_job` 未校验 `rows_affected` | 新增 |
| P2-4 | P2-5 未启动 | 计费/showback、DCGM 真实采集、goodput 均无实现 | 新增 |
| P2-5 | 前端测试噪声 | 原现象为 `JobManagement.spec.ts` 大量 `Failed to resolve component: el-collapse` 告警；追根因后确认是生产组件缺失（升级为 P1-7 并修复），复验告警计数 0 | ✅ 已修复 |
| P2-6 | 文档与身份漂移 | roadmap 开头过时结论、10-27/10-29 已修项无状态回写、`Cargo.toml:5` 曾自述 `"Rust spike PoC"`；本轮已加「状态回标」横幅 + 首节标历史留存 + 改生产后端描述。剩余：历史报告逐项回写、version 0.1.0 与前端 2.0.1 对齐 | ⚠️ 部分修复 |

---

## 四、已修复项对照

| 历史项 | 状态 | 当前证据 |
|---|---|---|
| P0-1 导航三源统一 | ✅ 已修复 | `src/nav.ts` 单一真源；`router/index.ts:3,26,62` 用 `getNavRoles` |
| P1-2 useAutoRefresh / status / useFetch 抽取 | ✅ 已修复 | 三个独立模块，构建分离 chunk |
| A1 部署翻转 PG 多副本 | ✅ 已修复 | `01-configmap.yaml:42` `USE_SQLITE=false`；replicas 3 + RollingUpdate；HPA/PDB；13 组双驱动迁移 |
| A4/A5 镜像名统一 + ImagePullBackOff | ✅ 已修复 | 三处统一 GHCR 层级名；`newTag: placeholder → main`；`scripts/deploy-k8s.sh` 支持 GIT_SHA |
| A3 前端单测 CI 阻断 | ✅ 已修复 | `ci-cd.yml:165` 移除 continue-on-error |
| A6 限流默认关闭 | ⚠️ 部分 | 代码默认 false，但 `01-configmap.yaml:80` 生产已设 true |
| A7 Cargo.toml 自述 PoC | ✅ 已修复 | 本轮改为生产后端描述；version 暂留 0.1.0，避免牵动 Cargo.lock 与发布流程 |
| 10-27 建议：Checkov/SBOM/cargo audit/依赖评审 | ✅ 已修复 | `ci-cd.yml:85,110,116,545,179` |
| 10-27 建议：SSH host key / Redis 口令 / 镜像 tag | ✅ 已修复 | `StrictHostKeyChecking=accept-new`；redis.conf 文件注入；17/18 钉 16.4/7.4-alpine |
| 10-27 建议：REVOCATION_FAIL_CLOSED 开关 | ⚠️ 部分 | 开关已加，默认仍 false |
| 10-29 修 36 项 + 调度 P0/P1/P2-1/2/4 | ✅ 已修复 | 本地实测 72 passed / clippy 零告警 / fmt 干净 |
| `release_gpu` 参数绑定错位 | ✅ 已修复 | `fb4a169` + 突变测试反向验证 |
| CI fmt 漂移 | ✅ 已修复 | `d6476e4`；本地 fmt check 通过 |

### 4.1 第一轮直接落地的优化（2026-11-09 上午）

全部为无需目标环境、不改变生产运行行为的确定性改动；改后已重跑前后端门禁（见第五节）。

| # | 改动文件 | 内容与验证 |
|---|---|---|
| 1 | `metaclouds-frontend-vue/src/plugins/element-plus.ts` | 补 `ElCollapse` / `ElCollapseItem` 深路径导入、按需样式与全局注册，修复生产作业表单折叠分组不渲染（P1-7）。验证：组件告警计数归零、56 用例全绿、element-plus 产物仅 +0.03 kB |
| 2 | `metaclouds-backend-rust/src/main.rs` | 派发循环启动时输出 WARN，显式声明「模拟执行 / 合成实耗 / 不得把作业状态与利用率当作真实算力交付」，带 3 个配置值便于排查；纯观测增强 |
| 3 | `metaclouds-backend-rust/k8s/01-configmap.yaml` | 显式声明 `DISPATCH_ENABLED` / `DISPATCH_SIMULATED_RUN_MS` / `DISPATCH_SIMULATED_UTILIZATION`（取值与代码默认一致，行为不变）；注释说明 `K8S_SIMULATION_MODE` 无业务消费者、设 false 不会切真实集群 |
| 4 | `metaclouds-backend-rust/Cargo.toml` | description 由 `"Rust spike PoC..."` 改为生产后端描述（Rust/axum + SQLx、SQLite/Postgres 双驱动） |
| 5 | `docs/metaclouds-optimization-roadmap.md` | 加「状态回标（2026-11-09）」横幅，首节改标「2026-10-30 初始基线 · 历史留存」，指向第六节与本报告 |
| 6 | `metaclouds-backend-rust/README.md` | RBAC 章节新增「Authorization boundary」：前端 localStorage 的 role 仅用于界面过滤、可被篡改但不产生服务端权限，真正边界在 `jwt_auth` + `RequirePermission` 与租户隔离 |

另有本报告两份交付物（HTML 可视化版 + 本 Markdown 归档版），不计入上述 6 项改动。

---

## 五、实测门禁（2026-11-09 本地）

工具链 `cargo` / `rustc` **1.96.0**，与 CI `dtolnay/rust-toolchain@1.96.0` 一致。每条命令均在「核验基线」与「本轮改动后」各跑一次。

| 层 | 命令 | 结果 | 说明 |
|---|---|---|---|
| 前端 | `npm run type-check` | ✅ | 基线 + 改动后均零类型错误 |
| 前端 | `npm run test` | ✅ | 11 文件 / 56 用例全绿；**改动后组件解析告警计数由数十条降为 0**（P1-7），证明修复有区分力 |
| 前端 | `npm run build` / `vite build --outDir <全新目录>` | ✅ | 首次被沙箱 safe-delete 守卫阻断（Vite 清空 `dist/` 触发 50 文件阈值，**非代码缺陷**）；改全新输出目录复验：基线 2,376 modules / 改动后 2,378 modules，均成功；element-plus chunk 951.34 → 951.37 kB（+0.03 kB）、CSS 205.39 → 206.12 kB，按需注册无打包回归 |
| 后端 | `cargo fmt --all -- --check` | ✅ | 基线 + 改动后均 FMT_OK |
| 后端 | `cargo check --quiet` | ✅ | 改动后 CHECK_OK |
| 后端 | `cargo test --lib` | ✅ | 基线 + 改动后均 72 passed / 0 failed / 0 ignored |
| 后端 | `cargo clippy --all-targets -- -D warnings` | ✅ | 基线 + 改动后均退出 0、零 lint 告警；改动后 stderr 的 `error deleting lock file ... 存取被拒 (os error 5)` 为 Windows 沙箱增量编译锁清理失败的环境噪声，非代码告警 |
| 仓库 | `git status` / `rev-list` | ⚠️ | 核验基线时工作区干净，HEAD `2be6ebe`；本轮产生 8 个文件改动（6 项优化 + 2 份报告）；11 个提交未推送 |

> **不要把本地绿等同于 CI 绿**：`cargo audit`、`cargo test --all-features`、Postgres 双驱动冒烟、Trivy、Checkov 均需 GitHub runner 与网络，本地无法等价覆盖。路线图 `:164` 自身也把「观察首个真正跑通的 Rust job」列为遗留核验点。

> **第二轮补充实测（2026-11-09 下午）**：见 `docs/system-fixes-2026-11-09.md` 第三节。要点：后端 `cargo check --all-features` / `cargo test --quiet` / `cargo test --features legacy-scheduler` 全通过；前端 12 文件 / 60 用例全绿、`vite build` 全新目录成功（element-plus chunk 951.37 kB）；`scripts/verify-deployment.sh` 经本地 mock 双场景实测（真实就绪 JSON → PASS，SPA 回退 HTML + 200 → 拒绝）；CI YAML 依赖图无悬空 `needs`。本节表格为第一轮 6 项优化当时的记录，保留作基线。

---

## 六、优化路线图

| 阶段 | 目标 | 主要动作 | 风险 |
|---|---|---|---|
| **S0 止血** | 让「模拟」显式且不可误用 | 生产 fail-closed 启动守卫；ConfigMap / compose 显式声明 `DISPATCH_ENABLED` / `DISPATCH_SIMULATED_*`；`/api/v1/system/mode` 模式接口 + 前端常驻演示横幅；回标 roadmap 过时结论；Cargo.toml 去 PoC 更名 | ✅ **已全部完成**（第二轮补上模式接口与 UI 横幅，见修复报告） |
| **S1 地基** | 接真实执行器 + 修多副本语义 | kube-rs 提交 Job/CRD 替换 `sleep`；执行任务 DB 持久化 + worker lease/心跳；并发改 DB 全局计数；reconcile 幂等 + owner；DCGM 替换 0.75 模拟采样 | ⚠️ **部分完成**：全局并发原子化、启动期 reconcile 租约选主、SIGTERM 排水、完成幂等、`usage_source` 合成标记已落地；**真实执行器 / 任务持久化 / DCGM 仍未做** |
| **S2 交付** | 让 CI 的绿可被信任 | 推送 11 提交并观察首个完整 run；`verify-deployment` 真实断言、去 `if: always()` 恒绿；按路径拆 `needs`；补 Trivy image scan；`newTag` 钉 SHA、Dockerfile 钉 digest；收敛部署路径 | ✅ **代码修复已落地**（真实断言脚本、镜像扫描、SHA 钉版本、双路径收敛、PG HA 标记）；**首个完整 run 仍待推送**；Dockerfile digest 未做 |
| **S3 安全** | 收口残余边界 | 翻转 `REVOCATION_FAIL_CLOSED=true`；删除/隔离死 mock；~~README 声明 role UI-only~~（本轮已做）；PG HA（CloudNativePG/托管）；解决 token 挂载矛盾 | 中 |
| **S4 能力** | 补齐高级调度与质量 | P2-3 抢占回归（victim 过滤 + run_token）；清 3 项 backlog；P2-5 计费/DCGM/goodput；~~前端测试桩注册 Element Plus 消噪~~（本轮已随 P1-7 修复完成）；文档单一真源 | 低-中 |

**执行状态（2026-11-09 下午更新）**：S0 已 5/5 全部落地；S2 的代码修复已全部落地、只剩「推送后观察首个完整 run」这一环境前置；S1 只完成了不依赖目标环境的多副本语义与数据可区分部分，真实执行器受限于无 K8s 集群 / GPU / DCGM / kubeconfig 与系统定位未定，本轮不启动。完整清单、实测门禁与推送前必须配置的仓库变量见 `docs/system-fixes-2026-11-09.md`。

---

## 七、决策落点与阻塞

### 需拍板

| 决策 | 方向 | 影响 |
|---|---|---|
| **系统定位**（最高优先） | A 演示/教学平台 · B 生产算力平台 | A 只需 S0；B 必须接真实执行器并重做验收口径 |
| 真实执行器形态 | kube-rs Job/CRD · Slurm 适配 · 外部调度器 API | 决定 RBAC、token 挂载、生命周期与重试语义 |
| 多副本调度模型 | 单副本 + leader 选举 · DB lease 多活 | 前者简单但冲突 HPA；后者复杂但保住 HA |
| 部署路径 | 收敛到镜像 + K8s · 保留 SSH 源码构建 | 决定 CI 简化幅度与 SQLite/PG 漂移 |
| 数据库 HA | 托管 PG · CloudNativePG/Zalando | 决定 3 副本后端是否真高可用 |

### 阻塞（明确原因）

| 阻塞 | 原因 | 解除条件 |
|---|---|---|
| 11 个提交未推送 | 离线沙箱无 GH 凭据，`GIT_TERMINAL_PROMPT=0` push 报 `could not read Username`；此前约定手动 `git push origin main` | 手动推送或配置凭据 |
| CI 从未真正跑通 | 需 GitHub-hosted runner 与网络（cargo audit / postgres:16 service / Checkov / Trivy）；门禁修复刚落地 | 推送后观察首个完整 run |
| 真实执行无法验证 | 无 K8s 集群、无 GPU/DCGM、无 kubeconfig | 提供集群与设备 |
| 前端 build 首次中断 | 沙箱 safe-delete 守卫触发（非代码问题），已用新输出目录复验通过 | 已绕过，无残留 |
| 生产 Secret 未注入 | `02-secret.yaml` 为占位值，`database-url` / `jwt-secret` 需人工提供，缺失时后端 fail-closed 退出 | 运维注入真实凭据 |

---

## 八、当前状态与后续

**上午（第一轮）**：6 项不依赖决策的确定性优化落地并通过前后端门禁复验。

**下午（第二轮）**：把三个 P0 从「隐含事实」变成「显式、有守卫、可查询、界面可感知、数据可区分」的状态。S0 全部完成，S2 的 CI 真实性修复已落地，S1 的多副本语义部分完成。逐项证据、约 30 个文件的改动面、完整实测门禁见 `docs/system-fixes-2026-11-09.md`。

**根本事实未变**：作业执行体仍是 `sleep` 后直接置 `completed`，实耗仍是合成值，K8s 页面仍是 mock。本轮修的是「不能把假的当真的用」，不是「把假的换成真的」。真实执行器改造仍受限于目标环境与系统定位，未启动。

**下一步的硬前置**（不是待授权动作，而是环境 / 运维条件）：

1. 按修复报告第四节配置 `DEV/STAGING/PROD/K8S_BASE_URL`、各类 SSH secret 与 `KUBE_CONFIG`；否则推送后流水线会**按设计明确变红**（不再假绿）。
2. 由具备 GitHub 凭据的一方执行 `git push origin main`（`main` 领先 11 个提交，离线沙箱无凭据无法代推）。
3. 观察首个完整 CI run，验证 `cargo audit` / `postgres:16` 双驱动集成 / Trivy 镜像扫描 / Checkov 等 runner 侧门禁。
4. 拍板「系统定位」（演示/教学平台 vs 生产算力平台），据此决定是否启动 S1 真实执行器与 P2-3 抢占分支回归。
