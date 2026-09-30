# Metaclouds P0/P1/P2 差距复核与落地报告（2026-09-30）

> 基线：`docs/comprehensive-review-optimization-2026-09-29.md`（12 章复盘，§9 生产落地遗留 18 条、§11 遗留分级 12 条、其他章节 8 条，共 38 条逐一复核）。
> 方法：三路并行只读审计（文档盘点 / 后端代码审计 / 前端+CI 审计），全部结论带 file:line 证据，不凭记忆；复核后确定本轮落地范围并实施。

---

## 0. 摘要

- **云端对齐**：本地与远端 `main` 均处于 `0441669`，`git rev-list HEAD...origin/main` = 0/0 无分叉；上轮云端 CI Run #55/56/57 全 success。
- **本轮新落地（已过门禁并推送）**：
  1. **P1｜种子口令 env 化**（用户确认项，R-P1-5 / L-9.3-5）：`src/db.rs` 两处 seed 从硬编码 `Admin@123456` 改为读取 `DEFAULT_ADMIN_PASSWORD`（走 `bootstrap_password`：env 已设且 ≥12 字符用 env；生产未设 **fail-secure 拒播种、启动即失败**；非生产回退 `Admin@123456`）。此前 `bootstrap_password` 是全仓零调用的死代码。
  2. **K8s Secret 接线（随 1 强制联动）**：因 `01-configmap.yaml:23` 已设 `SERVER_ENV=production`，`05-deployment.yaml` 补 `DEFAULT_ADMIN_PASSWORD` env（`secretKeyRef: default-admin-password`）；`02-secret.yaml` 键集对齐为规范四键 `jwt-secret / database-url / redis-password / default-admin-password`；`docker-compose.yml` 过时注释更新并补 env 透传。
  3. **文档漂移收敛（P2，6 处）**：runbook 端口 8080→8000、`/health` 预期体改实际信封、`/api/v1/health` 残留清除、`backend:latest` 旧镜像、`DATABASE_PASSWORD`/`database-password` 键名对齐、§12.6 迁移文件名纠错。
- **门禁**：`cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo build --release` / `cargo test` 全 exit 0；登录/播种定向复跑全绿（api_test 11/11、auth_test 6/6、b1_auth_test 13/13、b1_tenant_test 12/12、db_test 5/5 含 `seed_admin_creates_default_tenant_and_admin`）。K8s 改动经 Node 文本级静态校验 77 项通过（本机 PyYAML 缺失、pip 离线，采用回退方案；推送后 CI `validate-k8s` job 做 YAML 全量复验）。
- **仍遗留**：P0×1（PG 请求层方言移植）、P1×4（限流/CORS/redis-cron/Swagger-metrics 鉴权）、P2×4（性能/死代码/前端打磨/Can 补齐）及待目标环境项若干——均明确标注，未冒充完成。

---

## 1. 云端对齐状态

| 项目 | 状态 | 证据 |
|---|---|---|
| 本地 HEAD | `0441669` | `git log --oneline -1` |
| 远端 origin/main | `0441669` | `git fetch origin` 后 `rev-parse` |
| 分叉 | 0/0 | `git rev-list --left-right --count HEAD...origin/main` |
| 上轮 CI | #55/56/57 全 success | 上轮结论；本轮推送后由云端重新触发（见 §6） |

---

## 2. P0/P1/P2 复核矩阵（38 条基线逐项核对）

### 2.1 已闭环（上轮遗留，本轮确认不再存在）

| 编号 | 条目 | 上轮状态 | 本轮结论与证据 |
|---|---|---|---|
| L-9.1 | K8s 清单缺失 | 阻塞（目录不存在） | **已闭环**：`k8s/` 00–16 共 17 文件齐全（含 13-kustomization）；`05-deployment.yaml:96`=`ghcr.io/customizedvalidation/yunchuang/backend-rust`、`14-frontend-deployment.yaml:63`=`…/frontend`，与 runbook §4.2/§1.3 一致 |
| L-9.2 | CI 镜像名拼法冲突 | 阻塞（扁平名必致 ImagePullBackOff） | **已闭环**：`ci-cd.yml:60-61` 用 `${{ github.repository }}/backend-rust`、`/frontend` 层级命名，注释明写避免扁平名；仅剩 `push:false`（见 §2.5 待目标环境项） |
| R-P1-7 | 镜像名统一 | P1 | **已闭环**（同上；deploy-k8s `kubectl set image` 亦为同名，ci-cd.yml:789/791） |
| R-P1-6 | runbook/K8s 清单补全 | P1 | **基本闭环**：清单齐全；本轮继续收敛 runbook 文档漂移（见 §2.2 第 3 项） |
| O-7 | preview.proxy 临时配置 | 用完即删 | **已闭环**（未改 vite.config.ts） |
| O-8 | 本机未 commit | 本轮未提交 | **已闭环**：本轮全部改动已 commit + push |

### 2.2 本轮新修复（已落地、过门禁、随 commit 推送）

| 编号 | 条目 | 分级 | 落地内容与证据 |
|---|---|---|---|
| R-P1-5 | 种子密码托管 | P1 | `src/db.rs:279/326` 两处 seed 改走 `bootstrap_password("DEFAULT_ADMIN_PASSWORD","admin").map_err(AppError::bad_request)?` 后再 `hash_password(&admin_password)`；生产缺变量 → seed 报错 → main 启动即失败（fail-secure）。`bootstrap_credentials.rs:59-80` 语义核对属实（MIN_BOOTSTRAP_PASSWORD_LEN=12；is_production_env 判 ENVIRONMENT/SERVER_ENV/GO_ENV） |
| L-9.3-5 | .env.example 密码死配置 | 未分级 | **随之解决**：`DEFAULT_ADMIN_PASSWORD` 现被 seed 消费（此前死配置） |
| （联动） | K8s Secret 接线 | — | `05-deployment.yaml` env 补 `DEFAULT_ADMIN_PASSWORD`（secretKeyRef `metaclouds-secrets`/`default-admin-password`，缩进与现有条目一致）；`02-secret.yaml` 键集对齐四键并补 base64 占位（原文 `CHANGEME-default-admin-password`，实测解码一致）与 create-secret 命令（与 runbook §2.2 逐字符一致，含 `tr -d '/+='`） |
| （联动） | compose 注释与透传 | — | `docker-compose.yml:130-131` 过时注释改为新语义；backend environment 补 `DEFAULT_ADMIN_PASSWORD=${DEFAULT_ADMIN_PASSWORD:-Admin@123456}` |
| L-9.3-2 | runbook 端口/健康体漂移 | 未分级 | §5.3/§8.4 port-forward 8080→8000（`01-configmap.yaml:22` SERVER_PORT=8000、容器端口 8000）；`/health` 预期体改为实际信封 `{"success":true,"data":{"status":"ok","version":"…","uptime":…}}`（`src/handlers/health.rs:6-7`） |
| L-9.4-10 | 文档漂移收敛（部分） | TODO#10 | §8.1 `DATABASE_PASSWORD`→`DATABASE_URL`；§8.2 `backend:latest`→`backend-rust:latest`；§8.5 `database-password`→`database-url`（含 `psql "$DBURL"` 联动）；§5.4/§11.3 `/api/v1/health`→`/health`；§2.2/§2.3 create-secret 键集四键对齐 + 生产播种说明；§12.6 迁移文件名纠错 |
| L-9.4-11 | 迁移版本引用 | TODO#11 | §12.6 原引用 `000004_gpu_fine_grained.up.sql` 磁盘不存在；实测迁移集为 `001_initial.sql`~`009_legacy_fixes.sql`（sqlite 与 postgres/ 同名单），gpu/分区表落在 `006_b3_jobs.sql`/`007_b4_scheduler.sql`，runbook 已据实修订 |

**前端**：本轮无改动。审计确认登录页文案（`Login.vue:119-123`「初始密码由部署配置决定，请联系管理员获取」）与 env 驱动播种语义**一致**；主题科技蓝（`tokens.ts:77` brand.base=#2f6bff）、CSRF 双提交（`http.ts:17-24/44-53`）、httpOnly Cookie（`http.ts:36/71-78`）、Can 包裹写操作（12 个业务页）均达标。

### 2.3 仍遗留 P0

| 编号 | 条目 | 现状量化 | 说明 |
|---|---|---|---|
| R-P0-1 | **Postgres 请求层方言移植** | 运行时 sqlx 查询 153 处/43 文件；`?N` 占位 253 处/33 文件；`last_insert_rowid()` 需改 RETURNING 的 INSERT 站点 **21 个**；裸 `?` 5 处；`LIKE ?` 6 处；`soft_delete_update_sql` 1 定义/18 调用；编译期宏 = 0 | 更深阻塞：`AppState.pool` 需从 `SqlitePool` 具体类型枚举化为 `DatabasePool`。当前 `main.rs:48-53` **显式 fail-closed**（"request layer is not yet ported to postgres dialect (next stage)"），非静默降级；CI `rust-test-postgres` job 已用 PG16 service 跑 `postgres_smoke/integration_test --ignored`（迁移+播种路径有 CI 覆盖）。**独立里程碑，本轮未启动**（本机无 PG 运行时、改面大、风险高）。方言归一化辅助函数本轮亦不建议（池未枚举化无处挂载；裸 `?` 需顺序重编号易错） |

### 2.4 仍遗留 P1（代码类，列入生产加固批次）

| 编号 | 条目 | 证据/要点 |
|---|---|---|
| R-P1-1 | 限流默认策略 | 登录固定窗口；XFF 仅在 `trusted_proxies` 内读取——现 `rate_limit.rs:11-12` 无条件取 XFF 首段；后台清扫空 bucket |
| R-P1-2 | CORS / body-limit / tower-http | 精确 origin 装配 CorsLayer、RequestBodyLimitLayer、TimeoutLayer |
| R-P1-3 | Redis 缓存与 cron 接线 | build_cache 注入 AppState、Scheduler::start()；顺带 JWT jti 黑名单（B-06）与多实例限流共享（B-08） |
| R-P1-4 | Swagger/metrics 生产鉴权 | `!is_production()` 挂载 Swagger；`/metrics` 绑内网或 basic auth。⚠️ **`01-configmap.yaml:23` 已设 `SERVER_ENV=production`，真实部署前必须完成** |

### 2.5 仍遗留 P2

| 编号 | 条目 | 要点 |
|---|---|---|
| R-P2-1 | 性能 | dashboard 12 个串行 COUNT 合并/GROUP BY 或短 TTL 缓存；GPU 利用率全表拉内存改聚合 SQL；补 (tenant_id,status)/(user_id,status) 索引 |
| R-P2-2 | 死代码/死配置 | circuit_breaker 接线或删除；is_production() 改读 config.environment；分页上限双信源收敛；sqlx 开 trace feature |
| R-P2-3 | 前端打磨 | 工具栏 inline 宽度令牌化；表格列合并；chart-box 固定高改 aspect-ratio |
| R-P2-4 | JobManagement「取消」包 Can | `JobManagement.vue:117-119` 取消按钮未包 Can（仅状态判断） |
| 补强 | bootstrap_password 单元测试 | 注意 env 并行竞态（serial 或恢复现场），列 P2 |

### 2.6 待目标环境项（本机无法完成，**明确未完成/待配置**）

| 项 | 状态与证据 |
|---|---|
| KUBE_CONFIG secret | 未配置；`deploy-k8s` job 门控跳过并输出 warning（ci-cd.yml:752/812-816） |
| DEV/STAGING/PROD SSH secrets | 未配置；`${{ secrets.* }}` 空值守卫（ci-cd.yml:291-293/337-339/398-400），部署走 SSH+systemd |
| GHCR push:true | **未开启**：两个 build job `push:false`（ci-cd.yml:195/257），文件头 159 行注明待 GHCR 推送权限 |
| 真实 K8s apply | 未执行：3 副本/探针/HPA/PDB/NetworkPolicy 验收（L-9.4-1/2/6/7） |
| 生产 PostgreSQL/Redis 实跑 | 未执行：PG16+Redis7、20 表建成、重启数据不丢（L-9.4-5） |
| init.sql 与 sqlx 迁移双轨收敛 | 未执行：生产以 `sqlx migrations/postgres/` 为唯一建表路径（L-9.3-3/L-9.4-11） |
| Prometheus 栈 | 未部署：抓取路径 `/api/v1/monitoring/metrics` 鉴权问题、alertmanager/exporter 缺失（L-9.3-1/4、L-9.4-8） |
| DNS/Ingress/TLS | 未配置（L-9.4-6）；联网 cargo/docker build 实测未做（L-9.4-9，CI 已覆盖编译侧） |

---

## 3. 本轮改动清单与门禁

| 文件 | 改动 | 门禁结果 |
|---|---|---|
| `metaclouds-backend-rust/src/db.rs` | +14/−2：两处 seed 播种口令 env 化 | fmt/clippy/release build/test 全 exit 0；登录/播种定向复跑全绿（走非生产回退，符合预期） |
| `metaclouds-backend-rust/k8s/05-deployment.yaml` | +5：`DEFAULT_ADMIN_PASSWORD` env 接线 | Node 文本级校验 77 项通过 |
| `metaclouds-backend-rust/k8s/02-secret.yaml` | +9：四键对齐、占位与 create-secret 命令 | 同上（base64 占位实测解码一致） |
| `docker-compose.yml` | +7/−2：注释更新 + env 透传 | — |
| `docs/production-deployment-runbook.md` | +31/−29：文档漂移收敛（§2.2/§2.3/§5.3/§5.4/§8.1/§8.2/§8.4/§8.5/§11.3/§12.6） | 自检 grep：8080、/api/v1/health、backend:latest、database-password、DATABASE_PASSWORD 全 0 残留 |

K8s 静态校验说明：本机 PyYAML 缺失、`pip install` 离线挂起，采用 Node 文本级回退方案（校验脚本存于审计方 artifacts）；00-namespace（集群级对象）与 13-kustomization（kustomize 配置）的结构性例外与本次改动无关。**推送后 CI `validate-k8s` job 会做 YAML 全量复验**（镜像层级、kind/metadata/namespace、kustomization resources）。

---

## 4. 下一步建议

1. **P0 里程碑（PG 请求层方言移植）**：分两批——先 `soft_delete_update_sql` 生成器与 21 个 INSERT/RETURNING 改造，再全量 `?N`→`$N`；同步 `AppState.pool` 枚举化；CI `rust-test-postgres` 扩为常开。
2. **P1 生产加固批次**：R-P1-1/2/3/4 一次完成，并在真实部署前验证（configmap 已 `SERVER_ENV=production`，Swagger/metrics 鉴权必须先行）。
3. **目标环境配置**：KUBE_CONFIG / SSH secrets / GHCR push:true 就绪后，按 runbook §2（Secret 四键）、§5（健康验收）、§9（滚动）执行部署验收。
4. **P2 补强**：bootstrap_password 单测、JobManagement 取消 Can、前端打磨、性能 SQL 化。

---

## 5. 云端 CI 核验说明

- 本轮改动已 commit 并推送 `main`（提交号见 git log）。
- 本机 `gh auth login` 未完成，无 GitHub Actions 轮询凭据；云端 CI 由推送自动触发。
- 兜底门禁：全部改动已过本地门禁（cargo 四件套 + K8s 文本级校验）；推送后 CI `validate-k8s` 对 K8s 清单做全量复验，deploy-k8s 因 KUBE_CONFIG 未配置预期跳过（输出 warning）。
