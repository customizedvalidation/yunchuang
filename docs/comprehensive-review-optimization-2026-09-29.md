# Metaclouds 全面复盘与优化报告（2026-09-29）

> 范围：前端 14 个业务页面（UI 审计 + 修复 + 主题地基）、Rust/Axum 后端架构复盘 + 生产修复包、CI/部署清单静态验证、前后端本地冒烟与浏览器视觉复验。
> 方法：静态审计逐条 `file:line` 取证；对比度全部按 WCAG 相对亮度公式 node 实算；所有结论可回溯至证据文件（见 §12）。
> 约束：本机无 Docker / 离线环境，Postgres/Redis/K8s 动态验证由 CI 覆盖；本轮未 git commit。

---

## 1. 执行摘要

本轮目标：在既有 Go→Rust 迁移完成、前端 MDS v2.0 令牌已立的基线上，把交付水准推到"国际一流"——即**页面分布无破版、配色全部过 WCAG 对比度门禁、运行时缺陷闭环、启动装配与文档一致**。

关键结论：

- **UI 审计**：14 页共发现 **P0 × 20 / P1 × 51 / P2 × 52**。最大面积的 P0 是 Element Plus 主色从未主题化（`#409EFF` 链接文字实算 2.78:1），一次全局令牌映射即消除多页 P0；其次是"状态色原色当文字"（GPU 统计数字 2.20/1.83/2.97:1）与 KPI 渐变图标底色过浅（2.25–3.19:1）。
- **主题地基 MDS v2.1**：补齐 EP 核心变量映射、ECharts `mc-light` 主题、`:focus-visible`/`prefers-reduced-motion` 全局打磨；**22 项对比度自检全部 PASS**（普通文本 ≥4.5:1）。
- **页面修复**：14 页中 A 组闭环 6 P0 / 20 P1 / 19 P2，B 组闭环 2 P0（Topology 评分条、Monitoring KPI 图标）+ 其余 P1/P2；分类 chip 中性化、K8S 死列移除 12 列、MonitoringAlert 状态映射补齐、触控目标补到 44px、Accel 可点击卡键盘可达。
- **运行时缺陷闭环 3 项**：Dashboard 饼图不渲染（v-if + pre-flush watch 时序）、≤1023px 主内容塌缩为 0（窄屏网格双轨道）、登录后不跳转（pinia store 未更新）。
- **Rust 架构复盘**：P0 × 1 / P1 × 10 / P2 × 10；核心问题是"代码写得干净、启动装配层缺位"——Postgres 分支是死代码、GPU 分配无事务、限流默认关且信任可伪造 XFF、CORS/超时未接线、redis/cron 未启动、Swagger/metrics 无鉴权。
- **Rust 生产修复包**：3 项修复（Postgres 启动路径接线、GPU 分配事务化、`/health` 探 DB）；修复后 `cargo build / clippy -D warnings / fmt / test` 全绿，**336 passed / 0 failed / 2 ignored**。
- **代码质量基线**：Vue `type-check / build / test` 全绿（**31/31 用例**）；Rust 336 测试全绿。
- **生产落地**：2 条阻塞（K8s 清单目录缺失、CI 镜像名扁平/子路径拼法冲突必致 ImagePullBackOff）、11 条外部依赖 TODO。

---

## 2. 方法与范围

| 工作流 | 内容 | 环境约束 |
|---|---|---|
| 前端 UI 审计 | 14 页逐行静态审计，对比度 node 实算 | 只读，不改源码 |
| 主题地基 | MDS v2.0→v2.1 升级，EP 变量映射，ECharts 主题 | 升级不重写，.vue 零改动 |
| 前端页面修复 | A 组 7 页 + B 组 7 页 scoped 修复 | 不改业务逻辑/API/Can 包裹/PageState |
| Rust 架构复盘 | 启动装配 + 横切能力深路径审计 | 只读静态，证据 file:line |
| Rust 生产修复 | 受限修复包，不新增依赖、不改公共 API | postgres 请求方言 192 处留下一阶段 |
| 生产落地审查 | CI/compose/prometheus/runbook 一致性 | 无 Docker/kubectl，逐字段人工通读 |
| 冒烟与视觉验证 | 前后端本地启动 + 浏览器目检截图 | preview 代理临时配置后清理 |

验收标准映射：对比度 ≥4.5:1（正文）/ ≥3:1（图形图标）、响应式无横向溢出、触控目标 ≥44px、键盘可达、构建与测试全绿、`/health` 200、运行时无 console 错误。

---

## 3. 前端 UI 审计结果（14 页汇总）

### 3.1 分页计数与最严重发现

| 页面 | P0 | P1 | P2 | 该页最严重发现（一句话） |
|---|---:|---:|---:|---|
| Dashboard | 3 | 3 | 7 | KPI 渐变图标浅底白图标对比度 2.25–2.47:1，告警 footer 用原始 danger 色作文字 2.97:1 |
| JobManagement | 2 | 4 | 5 | 作业名/取消 link 按钮渲染 EP 默认蓝 #409EFF（2.78:1），且"取消"漏包 Can |
| GPUManagement | 6 | 5 | 4 | 三张统计卡直接拿 success/warning/danger 原色作文字（2.20/1.83/2.97:1），本页 P0 最密 |
| ResourceManagement | 0 | 3 | 3 | 类型/厂商/繁忙态标签混用 EP 蓝灰与 MDS 语义色，分类维度错用状态色 |
| ClusterManagement | 3 | 3 | 4 | 集群名/编辑/扩容/删除 link 全部 EP 默认蓝 2.78:1；厂商 chip 滥用品牌蓝 |
| K8SManagement | 2 | 6 | 4 | 约 13 列恒为"—/0"占位死列；active/online 映射到无样式的 mc-status 类，状态色失效 |
| PartitionManagement | 2 | 5 | 3 | link 按钮 EP 默认蓝；访问级别 admin→danger 红，用危险色表示权限级别 |
| SchedulerManagement | 0 | 3 | 3 | 类型 tag/link 吃 EP 默认蓝（2.53–2.78:1）；固定 720px Dialog / 760px Drawer 窄屏溢出 |
| TopologyManagement | 1 | 3 | 2 | 评分进度条硬编码 #faad14(1.90)/#00b8a9(2.49) 两色图形对比度不达标 |
| DatasetManagement | 0 | 3 | 3 | Fluid 缓存 Drawer 固定 920px 为全工程最宽面板，窄屏必横向溢出 |
| AccelerationSuiteManagement | 0 | 4 | 4 | 可点击分类卡 `div@click` 无 role/tabindex/键盘事件，键盘用户不可筛选；激活态误用 EP 主色变量 |
| MultiTenantManagement | 0 | 3 | 4 | 配额超 80% 进度条 `status=exception` 渲染 EP 红 #f56c6c（2.90:1）；label-width 与 label-position 矛盾 |
| MonitoringAlert | 1 | 4 | 3 | KPI 图标绿/红/黄浅底白图标 2.20/2.97/1.83:1 三处不达标；告警 active/resolved/ignored 全落 idle 灰点 |
| SecurityManagement | 0 | 2 | 3 | 策略类型 tag 吃 EP 蓝；`.rules-pre` 引用不存在的 `--mc-bg-2` 令牌 |
| **合计** | **20** | **51** | **52** | — |

### 3.2 跨页共性问题 Top

1. **【根因级】EP 主题未与 MDS 对齐**：`main.ts` 仅引 EP 默认 CSS，从未覆写 `--el-color-primary`，导致所有 `type="primary"` link/text/tag 渲染 `#409EFF`（2.78:1）；danger link `#f56c6c`（2.90:1）、warning link `#e6a23c`（2.19:1）、info tag `#909399`（3.08:1）。7 页无一幸免。
2. **状态色原色被直接当文字用**：GPU 统计大数字用 `--mc-success/--warning/--danger`（2.20/1.83/2.97:1）；Dashboard footer 内联 danger 文字。MDS 本已提供 `-fg` 版却未用。
3. **KPI 渐变图标底色过浅**：Dashboard `#69b1ff`(2.25)/`#ff7a9c`(2.47)/`#5b8bff`(3.19) 上放白图标，均不足 3:1。
4. **固定 px 弹窗/抽屉无窄屏兜底**：15 处 `width/size` 固定 480–920px（最差 Dataset 920px）。
5. **状态色语义漂移为分类色**：厂商"国产 X→danger 红"、运行态→EP primary 蓝、访问级别 admin→danger 红。
6. **操作列拥挤 + 触控目标不足**：各页操作列塞 3–6 个 `link size="small"`（约 24–28px 高，<44px 触控门槛）。
7. **K8S 页大量恒为"—"的占位死列**（约 13 列），信息密度虚高。
8. **Dashboard/Monitoring ECharts 默认色板脱离 MDS**：`#91cc75`(1.89)/`#fac858`(1.56)/`#73c0de`(2.03) 浅线图形对比度不达标。
9. **间距魔数散布**：`gutter=16`、工具栏控件宽 130–240px、`gap:12px` 未走 `--mc-gap`；4 处本地重复 `.mc-mono` 引用不存在的 `--mc-mono-font`。

### 3.3 对比度不达标实例（实算值）

| 组合 | 实算 | 门槛 | 判定 |
|---|---:|---|---|
| EP link 主蓝 `#409EFF` on 白（全页 link） | 2.78:1 | 正文≥4.5 | 不达标 |
| EP primary tag 文字 `#409EFF` on `#ecf5ff` | 2.53:1 | 正文≥4.5 | 不达标 |
| EP info tag `#909399` on `#f4f4f5` | 2.80:1 | 正文≥4.5 | 不达标 |
| MDS `#16c784` 作文字 on 白（GPU 可用数字） | 2.20:1 | 正文≥4.5 | 不达标 |
| MDS `#ffb020` 作文字 on 白（GPU 已分配） | 1.83:1 | 正文≥4.5 | 不达标 |
| MDS `#ff5c7a` 作文字 on 白（Dashboard footer） | 2.97:1 | 正文≥4.5 | 不达标 |
| 白图标 on `#69b1ff`（Dashboard 运行中 KPI） | 2.25:1 | 图形≥3 | 不达标 |
| 白图标 on `#ff7a9c`（Dashboard 告警 KPI） | 2.47:1 | 图形≥3 | 不达标 |
| 白图标 on `#16c784`/`#ff5c7a`/`#ffb020`（Monitoring KPI） | 2.20/2.97/1.83:1 | 图形≥3 | 不达标 |
| 进度条 `#faad14`/`#00b8a9` on 白（Topology 评分） | 1.90/2.49:1 | 图形≥3 | 不达标 |
| ECharts 默认线 `#91cc75`/`#fac858`/`#73c0de` on 白 | 1.89/1.56/2.03:1 | 图形≥3 | 不达标 |

---

## 4. 配色与设计令牌（本轮核心）

### 4.1 MDS v2.1 令牌增补

权威源仍为 `src/styles/index.css :root` 的 `--mc-*`，`tokens.ts` 为 TS 镜像（FORCE-SYNC 约定：先改 CSS 再同步 TS）。新增：

| 令牌 | 值 | 用途 |
|---|---|---|
| `--mc-brand-700` | `#1a44b8` | 品牌 scale 深档（按压/hover/图表深系列） |
| `--mc-brand-800` | `#163694` | 品牌 scale 最深档 |
| `--mc-focus-ring` | `#2f6bff` | `:focus-visible` 焦点环色 |
| `--mc-info` | `#55647e` | info 语义实心（石板蓝） |
| `--mc-info-fg` | `#3a4a63` | info 软底可读文字 |
| `--mc-info-soft` | `#eef4ff` | 纳入 info 成对语义 |

v2.0 既有令牌（品牌 `#2f6bff` 系、语义 `-soft/-fg`、中性、圆角/阴影/动效）全部保留未删。

### 4.2 Element Plus 核心变量映射策略

EP primary 与语义色同时承担"填充（白字）"与"文字（浅底）"双重角色。亮填充色 `#2f6bff/#16c784/#ffb020/#ff5c7a` 作文字时对比度仅 3.94/2.20/1.83/2.97，**故 EP 实心基色统一取 MDS 的 `-fg` 可读深档**，浅档按 EP 混白算法派生；CTA 主按钮仍由 `.el-button--primary` 叠加亮蓝渐变 `--mc-brand-grad`（白字 on 渐变中点 `#4555ec` = 5.58:1）。

| EP 变量 | 映射值 | 说明 |
|---|---|---|
| `--el-color-primary` | `var(--mc-brand-fg)` `#1e50e0` | 链接/primary 文字 on 白 = **6.40:1** |
| `--el-color-success` | `var(--mc-success-fg)` `#0b7a54` | 白字 on 它 = 5.35:1 |
| `--el-color-warning` | `var(--mc-warning-fg)` `#8a5a00` | 白字 on 它 = 5.93:1 |
| `--el-color-danger` | `var(--mc-danger-fg)` `#c2185b` | 白字 on 它 = 5.87:1 |
| `--el-color-info` | `var(--mc-info)` `#55647e` | 白字 on 它 = 5.98:1 |
| text/border/fill/bg 系列 | 全部映射 MDS 中性令牌 | 含 `--el-text-color-primary/regular/secondary`、`--el-border-color*`、`--el-fill-color*`、`--el-bg-color-page` |

light-3/5/7/8/9 与 dark-2 按 EP 混白/混黑算法逐档派生（primary-light-9=`#e9eefc`、danger-light-9=`#f9e8ef` 等）。

### 4.3 ECharts `mc-light` 主题

注册主题名 `mc-light`；`color` 循环品牌蓝打头、语义色穿插：`#2f6bff / #00b8a9 / #16c784 / #ffb020 / #ff5c7a / #5b8bff / #55647e / #1f52e0 / #69b1ff / #647189`。textStyle/图例用 `--mc-text-2`，轴标签用 `--mc-text-3`，分割线 `#eef2f8`。`registerMcLightTheme()` 幂等。

### 4.4 对比度自检 22 项 PASS 摘要

普通文本 ≥4.5:1 全部通过，摘关键项：白字 on primary `#1e50e0`=6.40、on success=5.35、on warning=5.93、on danger=5.87、on info=5.98；text-2 on 白=6.90、text-3 on 白=4.92、on bg=4.59；success-fg on success-soft=4.84、warning-fg=5.50、danger-fg=5.16、brand-fg on brand-50=5.80、info-fg on info-soft=8.13；primary on light-9=5.52；primary 作文字 on 白=6.40。**合计 22 PASS / 0 FAIL**。

### 4.5 响应式全局兜底

- `:focus-visible`：去默认 outline，改双层 box-shadow（外层 surface 留白 + 内层 `--mc-focus-ring`），跟随圆角。
- `@media (prefers-reduced-motion: reduce)`：动效令牌降级 1ms。
- 窄屏 `@media (max-width:767px)` 对 `.el-dialog/.el-drawer/.el-message-box` 强制 `width/max-width:calc(100vw-…)!important`。
- 修复 ≤1023px `.mc-app-shell` 由双轨道 `0 minmax(0,1fr)` 改为单列 `minmax(0,1fr)`，主内容不再塌缩为 0。

---

## 5. 页面分布与修复

### 5.1 14 页改动要点

| 页面 | 关键改动 |
|---|---|
| Dashboard | KPI 图标统一 `--mc-brand-grad`（白图标 4.50:1）、告警图标用 `--mc-danger-fg`（5.87:1）；footer danger 文字改 `-fg`；8 处手写渐变 hex 令牌化；接入 `mc-light` 主题与 palette；厂商/状态色板替换；图表标题裸 span→h3；dot 内联颜色改由 mc-status 类提供 |
| JobManagement | 全局 EP 映射解 P0；Dialog 加 `max-width:92vw`；link 触控目标扩到 32px |
| GPUManagement | 统计三原色数字改 `-fg`（2.20/1.83/2.97→5.35/5.93/5.87）；厂商 chip 统一 info 中性；隔离卡 `:xs/:sm/:md` 响应式；弹窗 max-width 兜底 |
| ResourceManagement | 类型/厂商/busy 标签统一 info 中性；弹窗兜底；toolbar 间距走 `--mc-gap` |
| ClusterManagement | 全局 EP 映射解 P0；厂商/调度器 chip 中性化；详情弹窗 h3→div 消除标题层级跳跃；fallback 状态不用品牌蓝 |
| K8SManagement | **移除 12 列恒为"—/0"的死列**（节点表 5、Pod 表 3、服务表 4）；active/online 归并到 running 修饰类；三处"刷新"实心 primary 改默认按钮；查看详情滥用 ElMessage 移除 |
| PartitionManagement | 全局 EP 映射解 P0；accessType admin→warning（琥珀提示权限提升）、其余 info；弹窗/Drawer max-width 兜底 |
| SchedulerManagement | 类型 tag→`.mc-chip` 中性；inline margin 移除；toolbar gap 令牌化；link 触控 44px |
| TopologyManagement | 评分条硬编码三色改 MDS `successFg/brand/warningFg`（1.90/2.49→5.35/4.50/5.93）；网络类型 chip 中性 |
| DatasetManagement | 源类型 chip 中性；删除本地重复 `.mc-mono`（引用不存在的 `--mc-mono-font`）；toolbar 令牌化 |
| AccelerationSuiteManagement | 分类卡补 `role="button" tabindex="0" :aria-pressed` + Enter/Space 键盘触发；激活态边框 `--el-color-primary`→`--mc-brand`；厂商/分类 chip 中性 |
| MultiTenantManagement | 资源类型 chip 中性；删除与 `label-position=top` 矛盾的 `label-width`；超配额进度条由全局 danger-fg 映射解决（2.90→5.87） |
| MonitoringAlert | KPI 图标 solid 语义底+白图标 → `*-soft` 浅底+`*-fg` 深图标（2.20/2.97/1.83→4.83/5.87/5.52/8.13）；**statusClass 补 active→pending、resolved→completed、ignored→idle**；ECharts 注入 MDS 深色 line palette（浅线 1.56–2.03→3.19–5.93）；GPU 利用率图标底 danger→info 中性；分页右对齐 |
| SecurityManagement | 策略类型 chip 中性；`.rules-pre` `--mc-bg-2`→`--mc-surface-3`、圆角 4px→`--mc-radius-xs`；删本地 `.mc-mono` |

### 5.2 三个运行时缺陷的根因与修复

| # | 缺陷 | 根因 | 修复 | 复验 |
|---|---|---|---|---|
| 1 | Dashboard 两张饼图不渲染（canvas=0） | 图表容器用 `v-if`（数据到达后才挂载），而初始化写在 `watch(option)` 里且默认 pre-flush——watch 回调先于 DOM patch，`chartRef.value` 仍为 undefined，else 分支不成立，之后不再触发 | 引入 `nextTick` + 统一 `syncCharts()`（容器已挂载且未 init 才 init，之后一律 setOption）；三个 watch 改 `{flush:'post'}`；onMounted 走 nextTick；新增 ResizeObserver 监听 `.mc-app-content` 自动 resize | 桌面 1440 视口 `canvas.length=2`、`_echarts_instance_=2`，资源分布/GPU 厂商环形图正常绘制 |
| 2 | ≤1023px 主内容区宽塌缩为 0 | 窄屏 sidebar/overlay 均 `position:fixed` 脱离网格流，`.mc-app-shell` 仍双轨道 `0 minmax(0,1fr)`，唯一在流内的 `.mc-app-main` 自动落进 0px 轨道 | 窄屏媒体查询改单列 `grid-template-columns:minmax(0,1fr)`，sidebar fixed 浮层 + overlay 遮罩逻辑不变 | 967 视口网格列=957px 单列，`.mc-app-content`=957，KPI 四列铺满、环形图居中；汉堡开合正常 |
| 3 | 登录成功后不跳转 dashboard | 原逻辑手写 localStorage，但 pinia `useAuthStore().user` 未更新，路由守卫 `isLoggedIn` 仍 false，`router.push` 被弹回 `/login?redirect=/dashboard` | 改用 `useAuthStore().login()`（正确设置 reactive user + 持久化 + 拉 CSRF），成功后 `router.push(redirect||'/dashboard')` | 清空 localStorage 后填表登录，POST /auth/login 200 → 自动跳 /dashboard（含 /auth/csrf） |

---

## 6. Rust 架构复盘

安全基线（JWT HS256 fail-closed、RBAC fail-closed、CSRF 双提交、httpOnly Cookie、生产禁 SQLite/内存/公开注册/通配 origin）经核对在位且有测试，本轮不降级。问题集中在**启动装配层（wiring）与横切能力**。

| ID | 级别 | 维度 | 证据 file:line | 影响 | 建议 |
|---|---|---|---|---|---|
| A-01 | **P0** | 可靠性/部署 | `main.rs:26`；`db.rs:196-216`（`connect_and_migrate`→`connect_sqlite`）；`auth/middleware.rs:19`（pool 类型 `SqlitePool`）；`docker-compose.yml:39-41` | compose 下发 `postgresql://` DSN 却永远走 SQLite 驱动，要么启动失败要么在 CWD 生成孤儿 SQLite 文件、PG 容器旁路 | main 改 `connect_pool(&config)`，pool 抽象为 `DatabasePool` |
| A-02 | P1 | 性能 | `services/gpu.rs:207-234` | GPU 分配 N+1：先拉全部设备再循环 `SELECT SUM(fraction)`，N 次串行 RTT | 改单条 `LEFT JOIN ... GROUP BY device_id` |
| A-03 | P1 | 并发 | `services/gpu.rs:241-275,293-319`；全仓 `transaction()|begin()` = 0 | 分配三步无事务无行锁，并发可超额分配 fraction>1.0；中途失败留孤儿 allocation | 包 `pool.begin()` 事务 + `FOR UPDATE` |
| A-04 | P1 | 安全 | `middleware/rate_limit.rs:98-103,136-145,54,77-86`；`config.rs:312` | 限流默认关；信任可伪造 XFF 首段；空 bucket 从不驱逐致内存单调增长 | 登录路由单独固定窗口；trusted_proxies 内才读 XFF；后台清扫 |
| A-05 | P1 | 安全 | `Cargo.toml` 无 tower-http；`config.rs:249,408-417`；`routes.rs:488-496` | CORS 配置只校验不接线，非同源 SPA 带 Cookie 写请求被拦却给人已配错觉 | 引入 `CorsLayer` 按精确 origin 列表装配 |
| A-06 | P1 | 可靠性 | `config.rs:328-331`；`main.rs:39-44` | 无请求体上限、无读写空闲超时，超大 body 打内存、慢连接拖死下线 | `RequestBodyLimitLayer` + `TimeoutLayer` |
| A-07 | P1 | 信息泄露 | `routes.rs:478,486-491` | Swagger UI/OpenAPI/`/metrics` 生产无鉴权裸奔 | `if !is_production()` 条件挂载；metrics 绑内网/加 basic auth |
| A-08 | P1 | 安全 | `db.rs:251,263` | 种子 admin 硬编码 `Admin@123456`，无首登强制改密、无启动告警 | 启动 warn + 要求 env 覆盖或 `must_change_password=1` |
| A-09 | P1 | 可观测 | `handlers/health.rs:44-50`；`docker-compose.yml:65-67` | `/health` 恒 200 不探 DB，liveness/readiness 同一端点，DB 挂探针仍绿 | 拆 live/ready，ready `SELECT 1` 失败 503 |
| A-10 | P1 | 可靠性 | `cache/redis.rs:251`（无调用方）；`scheduler/mod.rs:53,122`；`main.rs:26-44` | Redis 缓存与 cron 调度器在生产启动路径根本没启动，配置开关是心理安慰 | main 装配 cache 注入 AppState、`Scheduler::start()` 并在 shutdown 等待 |
| A-11 | P1 | 可靠性 | `main.rs:39-47` | 优雅停机无超时、不等待后台任务，in-flight 请求挂起则进程永不退出 | `timeout(30s)` 兜底 + 显式 await scheduler.shutdown |
| B-01~B-10 | P2 | 性能/可维护 | `services/monitoring.rs:36-83`；`services/gpu.rs:356-393`；migrations 缺索引；`orm/mod.rs:131` 分页上限双信源；`middleware/mod.rs:37-54` 熔断器死代码；`services/auth.rs:153` JWT 登出不失效；sqlx 无 trace span；登录失败表进程内 HashMap；compose healthcheck 注释过时；403 回显过宽 | 看板每次 12 个串行 COUNT、GPU 利用率全表拉内存、缺索引、配置死项、token 不可吊销、慢 SQL 不可观测等 | 见 §11 下一轮建议 |

**Top 结论**：Postgres 接线死代码（P0）、GPU 分配 N+1+无事务、限流默认关+信任可伪造 XFF、CORS/超时未接线、redis/cron 未启动、Swagger/metrics 无鉴权、种子密码硬编码。

---

## 7. Rust 生产修复包

在 build/clippy/fmt/test 全绿基线上做受限修复（未新增依赖、未改公共 API/路由、未动前端）。

| 修复 | 文件 | 内容 |
|---|---|---|
| 【P0】Postgres 启动路径接线 | `src/db.rs`、`src/main.rs` | 新增 `is_postgres_url()`（识别 `postgres://` 与 `postgresql://`）、`wants_postgres(&Config)`、`DatabasePool::as_postgres()`、`seed_admin_if_empty_postgres(&PgPool)`（`$N`+`RETURNING` 方言）；main 按配置分支：sqlite 路径逐字节不变，postgres 路径真 `connect_pool`→`run_migrations(migrations/postgres)`→播种；postgres 模式完成启动期真连后因请求池仍为 SqlitePool 而**显式报错**，不再静默兜底 sqlite 文件。新增 3 个单测 |
| 【P1】GPU 分配事务化 | `src/services/gpu.rs` | `allocate_gpu` 的 INSERT allocation 与 UPDATE device 包进同一 `pool.begin()` 事务，任一步失败整体回滚，消除孤儿写一半窗口；签名与返回结构不变 |
| 【P1】/health 探活 DB | `src/handlers/health.rs` | 注入 `State<AppState>`，执行 `SELECT 1` 轻量探活；DB 不可用返回 503 + 统一错误信封；正常时 200 信封结构（success/data.status/version/uptime）与此前完全一致 |

**边界说明**：postgres **请求层**方言（约 192 处 `?N`/`last_insert_rowid()` → `$N`/`RETURNING`）属下一阶段大改，本轮未做；当前 postgres 模式在完成启动期真连+迁移+播种后会显式报错提示，不再静默错写 sqlite 文件。种子密码 `Admin@123456`、rate_limit 默认值、CORS/tower-http、redis/cron 接入、Swagger 鉴权本轮未动。

**验证（修复后复跑）**：

| 命令 | Exit | 结果 |
|---|---:|---|
| `cargo build` | 0 | 1m09s，无 warning/error |
| `cargo clippy --all-targets -- -D warnings` | 0 | 28.3s，0 warning（CI 严格级别） |
| `cargo fmt --all -- --check` | 0 | 通过 |
| `cargo test` | 0 | **336 passed / 0 failed / 2 ignored**（较基线 333 增 3 个新单测；2 ignored 为 postgres 双驱动用例，由 CI `rust-test-postgres` 覆盖） |

---

## 8. 代码质量验证

### 8.1 Vue 前端

| 检查项 | 命令 | 结果 |
|---|---|---|
| 类型检查 | `npm run type-check`（vue-tsc --noEmit） | exit 0，无 TS 错误 |
| 生产构建 | `npm run build`（vue-tsc && vite build） | exit 0，2289 模块，11.94–17.41s |
| 单元测试 | `npm run test`（vitest run） | exit 0，**5 文件 / 31 用例全过**（http 10、auth store 5、Login 6、Dashboard 3、JobManagement 7） |

### 8.2 Rust 后端基线

`cargo build`（0 warning/0 error）、`cargo clippy --all-targets -- -D warnings`（0 warning）、`cargo fmt --check`（通过）、`cargo test`（336 passed / 0 failed / 2 ignored，覆盖 RBAC/攻击面/安全中间件/Redis/cron/tracing/PG 冒烟）。CI workflow `rust-lint-test`/`rust-test-postgres`/`rust-coverage`/`docker-build-backend-rust` 逐项核对配置一致，仅一处头部注释依赖图 cosmetic 漂移（不影响功能）。

---

## 9. 生产落地静态验证

本机无 Docker/kubectl/pyyaml，结论来自对 ci-cd.yml、compose（根+嵌套+prod）、init.sql、prometheus.yml、alerts.yml、daemon.json、.env.example、config.rs、main.rs、db.rs、routes.rs、health.rs、Dockerfile、runbook、guide 的逐字段人工通读。

### 9.1 K8s 清单缺失（阻塞）

`scripts/` 下无任何 K8s YAML（全部为凭证轮换脚本）；全仓 YAML 仅 ci-cd.yml、docker-compose.yml（根+嵌套）、prometheus.yml、alerts.yml、docker-compose.prod.yml。**不存在**任何 Deployment/Service/Ingress/ConfigMap/Secret/HPA/PDB/NetworkPolicy/ServiceMonitor/PrometheusRule 清单。runbook §4.2 列出 18 份、guide §D.1 列出 14 份 `metaclouds-backend-rust/k8s/*.yaml`，但该目录在工作树中**不存在**（Test-Path=False）。端口/探针/资源/Secret/Ingress 对应关系均"无法校验"。

### 9.2 CI 镜像名拼法冲突（必致 ImagePullBackOff）

CI 构建产物是 `ghcr.io/customizedvalidation/yunchuang-backend-rust`（扁平，来自 `RUST_BACKEND_IMAGE_NAME=${github.repository}-backend-rust`）；而 runbook/guide/compose.prod/K8s 引用 `ghcr.io/customizedvalidation/yunchuang/backend-rust`（子路径）。一旦 `push:true`，拉取侧必错。CI 两端当前 `push:false`（待 GHCR 写权限），属设计内但必须先统一拼法。

### 9.3 漂移点

- `prometheus.yml` 的 `metaclouds-jobs` job 抓根级 `/monitoring/metrics`（无前缀、无鉴权），而真实路径是 `/api/v1/monitoring/metrics` 且在 protected 组需 JWT → 会 404/401。
- runbook §5.3 port-forward 8080 vs 后端实际监听 8000；runbook 预期 `/health` 响应体 `{"status":"healthy","dependencies":1}` vs 实际 `{"success":true,"data":{"status":"ok",...}}`；runbook/guide 引用 `/api/v1/health` 但路由只有根级 `/health`。
- `init.sql`（列 `users.password` bcrypt）与 Rust 迁移（`password_hash` 列）双轨建表，接线后会撞 schema。
- prometheus.yml `alertmanager:9093` 但 compose 无 alertmanager 服务；alerts.yml 引用 node_exporter/kube-state-metrics/pg/redis/etcd exporter 均未部署，规则在缺失序列上永不触发。
- `.env.example` 的 `DEFAULT_ADMIN_PASSWORD`/`DEFAULT_USER_PASSWORD` 未被 seed 消费（seed 硬编码 `Admin@123456`），属死配置。

### 9.4 外部依赖 TODO 清单（11 条，步骤级）

1. **找回/重建 K8s 清单集**：从正确分支取回 `k8s/`，或按 guide §D.1 重建 00-namespace…13-kustomization，`kubectl apply --dry-run=server -k`。
2. **统一镜像名拼法并启用 GHCR push**：二选一并贯穿 CI/runbook/compose.prod/K8s，然后 `push:false→true`，验收 `docker pull` 成功无 ImagePullBackOff。
3. **main.rs 切换双驱动**（本轮已完成 db.rs 侧接线，见 §7）：验收嵌套/根 compose 起后端日志打印 `using postgres`、migrations/postgres 全 up。
4. **真实 Secret 注入**：GitHub Environments + K8s Secret，JWT_SECRET/DATABASE_PASSWORD/REDIS_PASSWORD 不落明文。
5. **真实 Postgres/Redis 实跑与迁移验证**：起 PG16+Redis7，跑 `postgres_smoke_test`/`postgres_integration_test --ignored`，验收 20 张表建成、重启数据不丢。
6. **DNS/Ingress/TLS**：A 记录 + ingress-nginx + cert-manager，验收 `curl -fI https://<域名>/health` 200。
7. **真实集群 apply + 滚动/回滚演练**：3 副本 Ready、三探针通过、HPA/PDB/NetworkPolicy 生效。
8. **修正 Prometheus 抓取路径并补齐采集器/告警发送**：改抓 `/metrics` 或后端暴露无鉴权端点；补 alertmanager:9093 与各 exporter。
9. **联网 cargo build / docker build 实测**：本机无 Docker/离线，需在联网环境验证镜像可构建。
10. **收敛文档漂移**：runbook 端口 8080→8000、`/health` 响应体改实际信封、删/修 `/api/v1/health`、标注 `.env.example` 死配置。
11. **init.sql 与 sqlx 迁移双轨收敛**：生产以 sqlx `migrations/postgres/` 为唯一建表路径，避免"表已存在"报错。

---

## 10. 冒烟与视觉验证

### 10.1 本地启动

- 前端 `npm run build` exit 0（2289 模块，17.41s），`dist/` 产出 index.html + assets（含 echarts/element-plus 手动分包）。
- 后端 sqlite 模式起在 **8001**（vite.config.ts 注释：原 Go :8000，Rust 迁移后默认 :8001；前端代理默认指向 8001）。`GET http://localhost:8001/health` → **HTTP 200** `{"success":true,"data":{"status":"ok","version":"0.1.0",...}}`。
- preview `vite preview --port 4173`：因 preview 不继承 `server.proxy`，临时加 `preview.proxy {'/api'→8001}`（用完即删，未改 vite.config.ts）；`POST /api/v1/auth/login`（admin/Admin@123456）→ **200 真实 JWT**。

### 10.2 浏览器目检与截图清单

| 页面 | 结果 | 截图 |
|---|---|---|
| /login | 科技蓝渐变主视觉、蓝紫渐变登录按钮；console 无错误 | `01-login.png` |
| /dashboard（窄视口 967） | 修复后内容铺满、KPI 四列、环形图居中 | `11-narrow-closed.png` |
| /dashboard（桌面 1440） | 深蓝侧栏、KPI 行、最近告警；两张饼图正常绘制（科技蓝 61.54%） | `07-desk-dashboard-top.png`、`10-desk-dash-fixed.png`、`12b-check.png` |
| /gpus | 统计卡（总390/可用354/已分配36/维护0）、表格真实数据；console 无错误 | `08-gpus.png` |
| /job/list | 标题/筛选栏/Tab/空状态正常；console 无错误 | `09-job.png` |

### 10.3 运行时与复验

- 各页面 `console_messages()` 均为空数组，**无 JS console 错误**；登录接口 200、各只读列表接口随页正常返回。
- 缺陷修复后复验：登录自动跳 /dashboard（含 /auth/csrf）；桌面 1440 视口 `canvas.length=2`、`_echarts_instance_=2`，资源分布环形图（已用 0%/可分配 100%）与 GPU 厂商环形图（NVIDIA 61.54% 科技蓝 + 多厂商灰蓝色块）正常绘制；967 窄视口网格列=957px 单列、`.mc-app-content`=957，汉堡开合侧栏浮层+遮罩正常。

---

## 11. 遗留问题与下一轮建议

| 优先级 | 事项 | 说明 |
|---|---|---|
| P0 | Postgres 请求层方言移植 | 约 192 处 `?N`/`last_insert_rowid()` → `$N`/`RETURNING`；当前 postgres 模式启动期真连后会显式报错，需完成连接层抽象（`AppState.pool` 从 `SqlitePool` 改 `DatabasePool`，service 签名批量改 `&Db`） |
| P1 | 限流默认策略 | 登录路由单独固定窗口（如 10 次/分钟）；`client_ip` 仅在 trusted_proxies 内才读 XFF；后台清扫空 bucket |
| P1 | CORS / body-limit / tower-http | 引入 `tower-http::cors::CorsLayer` 按精确 origin 装配；加 `RequestBodyLimitLayer` 与 `TimeoutLayer` |
| P1 | Redis 缓存与 cron 调度器接线 | main 装配 `build_cache` 注入 AppState、`Scheduler::start()` 并在 shutdown 等待；顺带解决 JWT 登出 jti 黑名单（B-06）与多实例限流共享（B-08） |
| P1 | Swagger/metrics 生产鉴权 | `if !is_production()` 条件挂载 Swagger；`/metrics` 绑内网或 basic auth |
| P1 | 种子密码托管 | 启动 warn + 要求 `BOOTSTRAP_ADMIN_PASSWORD` env 覆盖，或登录后强制 `must_change_password=1` |
| P1 | runbook/K8s 清单补全 | 见 §9.4 TODO-1/10，清单目录缺失与文档漂移 |
| P1 | 镜像名统一 | 见 §9.2，扁平/子路径二选一贯穿全链 |
| P2 | 性能 | dashboard 12 个串行 COUNT 合并/GROUP BY 或 10–15s 短 TTL 缓存；GPU 利用率全表拉内存改聚合 SQL；补 `(tenant_id,status)`/`(user_id,status)` 索引 |
| P2 | 死代码/死配置 | circuit_breaker 接线或删除；`is_production()` 改读 `config.environment`；分页上限双信源收敛；sqlx 开 trace feature |
| P2 | 前端 P2 打磨 | 工具栏控件 inline 宽度令牌化、表格列合并（"编辑+更多"下拉、次要列收入抽屉）、chart-box 固定高改 aspect-ratio |
| P2 | JobManagement"取消"按钮包 Can | 本轮受"不改 Can 包裹"约束保留，下一轮与后端 RBAC 对齐 |

---

## 12. 证据索引

### 12.1 子产物（10 份）绝对路径

1. 前端 UI 审计 A 组：`C:\Users\IES165225\AppData\Local\DoubaoWork\User Data\Default\.doubaowork\agent_mode\workspace\.sessions\38440126559900930\agents\s_000cbnaWz2j\artifacts\audit-A.md`
2. 前端 UI 审计 B 组：`...\agents\s_000cbnaWQJa\artifacts\audit-B.md`
3. Rust 架构复盘：`...\agents\s_000cbnaWqJs\artifacts\arch-review-2026-09-29.md`
4. Rust 代码质量 + 生产修复包：`...\agents\s_000cbnaWfSt\artifacts\rust-code-quality.md`
5. Vue 代码质量：`...\agents\s_000cbnaWsAS\artifacts\vue-code-quality.md`
6. 设计令牌与主题地基：`...\agents\s_000cbnC1DVn\artifacts\theme-foundation.md`
7. A 组 7 页修复：`...\agents\s_000cDrEsWfK\artifacts\fix-A.md`
8. B 组 7 页修复：`...\agents\s_000cDrEsw5G\artifacts\fix-B.md`
9. 生产落地验证：`...\agents\s_000cDrEJecT\artifacts\production-landing.md`
10. 冒烟与视觉验证：`...\agents\s_000cDMyS1Ep\artifacts\smoke-2026-09-29.md`

（`...` = `C:\Users\IES165225\AppData\Local\DoubaoWork\User Data\Default\.doubaowork\agent_mode\workspace\.sessions\38440126559900930`）

### 12.2 本轮改动文件清单（`git status --short` + `git diff --stat`）

共 **20 个已跟踪文件修改 + 1 个未跟踪目录**，合计 **609 insertions / 244 deletions**：

**后端 Rust（4 文件）**
- `metaclouds-backend-rust/src/db.rs`（+130 行，双驱动工厂/PG 播种）
- `metaclouds-backend-rust/src/handlers/health.rs`（+20 行，/health 探 DB）
- `metaclouds-backend-rust/src/main.rs`（+37 行，启动路径按配置分支）
- `metaclouds-backend-rust/src/services/gpu.rs`（+9 行，分配事务化）

**前端 Vue 页面（15 文件）**
- `src/pages/AccelerationSuiteManagement.vue`、`ClusterManagement.vue`、`Dashboard.vue`（+160/-… 改动最大，含图表时序修复）、`DatasetManagement.vue`、`GPUManagement.vue`、`JobManagement.vue`、`K8SManagement.vue`（死列移除）、`Login.vue`（登录跳转 store 修复）、`MonitoringAlert.vue`、`MultiTenantManagement.vue`、`PartitionManagement.vue`、`ResourceManagement.vue`、`SchedulerManagement.vue`、`SecurityManagement.vue`、`TopologyManagement.vue`

**前端样式与主题（1 修改 + 1 新增目录）**
- `src/styles/index.css`（+147 行，EP 变量映射 + 全局打磨 + 窄屏网格修复）
- `src/theme/`（未跟踪新增目录：`tokens.ts` + `echarts.ts`）

### 12.3 关键截图路径

冒烟验证产物目录下：`01-login.png`、`07-desk-dashboard-top.png`、`08-gpus.png`、`09-job.png`、`10-desk-dash-fixed.png`（桌面饼图修复后）、`11-narrow-closed.png`（窄屏铺满）、`12b-check.png`（窄屏侧栏浮层 + GPU 厂商环形图）。
