# 全菜单功能完成度复盘（2026-11-16）

> 用户问题：**「全面复盘与优化此系统的所有菜单项，是否已完善了所有功能？」**
> 结论先行：**没有全部完善**。菜单结构本身完整、无死链；页面 CRUD 大多真实接通后端；
> 但存在 **1 处后端能力缺口（真实 K8s / 真实执行体）**、**2 类数据合成**、
> **1 处权限口径不一致** 与 **3 处"伪装功能"**。本轮修复了能安全落地的部分，
> 其余如实列为未完成，不写成已交付。

---

## 1. 复盘口径与证据来源

| 层 | 单一真源 | 核查方式 |
|---|---|---|
| 菜单 | `metaclouds-frontend-vue/src/nav.ts`（14 一级 + 6 子项） | 逐项读源码 + 与 router 交叉核对 |
| 路由 / 守卫 | `src/router/index.ts` | 全量读完 + 与菜单 key 对照 |
| 页面功能 | `src/pages/*.vue`（15 个，含 Login） | 子代理逐页核查 + 关键页亲自复核 |
| API 封装 | `src/api/index.ts` / `http.ts` | 逐函数与后端路由对照 |
| 后端端点 | `metaclouds-backend-rust/src/routes.rs` | 全量读完 + 与前端端点对照 |
| 权限矩阵 | `src/authz/mod.rs` | 读完 `MANAGER_PERMISSIONS` / `USER_PERMISSIONS` |
| 数据真实性 | `services/dispatch.rs`、`services/k8s.rs`、`handlers/system.rs` | 读源码与启动告警 |

**动作口径**：`真实` = 调用后端 API 并持久化；`UI 壳` = 按钮存在但无后端能力 /
仅提示 / 仅本地状态；`无` = 该动作不存在。

---

## 2. 菜单结构：完整（无死链、无孤儿路由）

- 菜单 20 个 key（14 一级项 + `/job/*` 3 子项 + `/k8s/*` 3 子项）与 router 全部一一对应。
- 唯一角色约束：`/tenant` → `['admin','manager']`；菜单过滤、路由守卫、Sidebar 均从
  `nav.ts` 同源取值，fail-closed。
- 隐藏路由仅 `/login`、首页重定向与 404 兜底；无未知死链。
- badge 由 9 个真实 API 聚合，60s 刷新；`/k8s/pods` 的误导徽标此前已主动移除。

**结论：菜单"导航层"是完善的。问题在菜单背后的"功能层"。**

---

## 3. 逐页功能完成度矩阵

| 菜单项 | 路径 | 真实能力 | 缺失 / 受限 | 判定 |
|---|---|---|---|---|
| 仪表盘 | `/dashboard` | 9 个 GET 聚合、KPI、刷新、跳转 | 时间范围切换是假控件（本轮移除） | 🟡 → 🟢 |
| 作业管理 | `/job*` | 建/取消/提交 K8s/详情/checkpoint/筛选/分页 | 无编辑、无删除、无排序/批量/导出 | 🟢（核心闭环） |
| 资源管理 | `/resource` | 列表、编辑、厂商概览、筛选 | 无新增/删除（设计上为资源池） | 🟢 |
| GPU 设备 | `/gpus` | 增/改/删/分配/释放/维护/记录/详情 | 隔离能力卡片是硬编码展示 | 🟡（展示项非真数据） |
| 分区管理 | `/partitions` | CRUD、优先级、运行时长、权限增删 | 无独立详情页 | 🟢 |
| 集群管理 | `/cluster` | CRUD、重建、扩容、详情、搜索、分页 | 无排序/批量/导出 | 🟢 |
| K8S 管理 | `/k8s/*` | 三 Tab 均有数据、搜索、分页 | **三 Tab 语义借用，非真实 K8s 对象** | 🔴（本轮加显著标注） |
| 调度器 | `/schedulers` | CRUD、队列/节点、同步作业、健康检查 | 无 | 🟢 |
| 拓扑感知 | `/topology` | CRUD、评分、搜索、筛选 | 表格"机架ID"读 `labels.rack`，与表单 `rack_id` 口径不一 | 🟡 |
| 加速套件 | `/acceleration` | 列表、编辑、启停、详情、分类 | 无新增/删除（后端端点不存在） | 🟡 |
| 数据集 | `/datasets` | 数据集 CRUD + Fluid 缓存 CRUD + 启停/预取 | 无 | 🟢 |
| 监控告警 | `/monitoring` | 指标、告警列表、解决/忽略（真实写库） | **趋势图为 `Math.sin` 合成**；重新打开无后端接口（本轮禁用） | 🔴（合成数据） |
| 多租户 | `/tenant` | 租户 CRUD + 多维配额 CRUD + 用量 | manager 只读、`used` 前端写死 0 | 🟢（写操作受限合理） |
| 安全管理 | `/security` | 策略列表、编辑、启停、规则详情 | 无新增/删除（后端仅 list/update 接前端） | 🟡 |
| 登录 | `/login` | JWT + CSRF 真实鉴权 | 失败锁定为纯前端逻辑，无后端限流 | 🟢（后端防线独立） |

**统计**：核心 CRUD 闭环 **8 项 🟢**；部分受限 **5 项 🟡**；能力性缺口 **2 项 🔴**。

---

## 4. 已确认的真实缺陷与本轮修复

| # | 问题 | 证据 | 修复 | 验证 |
|---|---|---|---|---|
| 1 | 告警"解决/忽略"对 manager 可见但必 403 | 前端 `Can roles=['admin','manager']`；后端 `routes.rs` 挂 `ALERT_WRITE`，`authz/mod.rs` 仅 admin 持有 | 收窄到 `['admin']` | 新增测试：无 role 无按钮、admin 可见 |
| 2 | "重新打开"点了只弹提示、状态不变 | `setStatusOverride` 的 else 分支仅 `ElMessage.warning` | 改为禁用态 span + tooltip 说明 | 新增测试断言不是 button |
| 3 | Dashboard 时间切换只改文案 | `range` 只驱动 `rangeLabel`，不参与任何请求；后端 `/monitoring/dashboard` 无时间参数 | 移除控件，改为"实时快照" | 浏览器 DOM 确认 body 无 `el-radio-button` |
| 4 | 租户 KPI 卡对 user 可见但点不进 | 卡片恒为 `RouterLink to='/tenant'`，user 被守卫弹回 | 用 `navConfig` 同源角色判断，无权限渲染静态卡 | 新增 admin/无角色双分支测试 |
| 5 | K8S 三 Tab 名不副实 | 节点=GPU 资源、Pod=运行中作业、服务=资源列表；后端 `/k8s/*` 为 mock 且前端未接 | 加显著来源标注横幅 | 新增测试锁死诚实性契约 |

**未修但已定性**：
- `TopologyManagement` 的 `rack_id` / `labels.rack` 字段口径差异：需确认后端 schema 后改，
  本轮不臆测数据含义。
- `Sidebar` 对所有角色请求 `tenantApi.list`：多余但无害（user 有 `TENANT_READ`），暂不改。

---

## 5. 全局能力缺口（非 UI 可修，不伪装为已完成）

| 缺口 | 代码证据 | 影响面 |
|---|---|---|
| 作业执行体为模拟 | `services/dispatch.rs`：`sleep(sim_ms)` 后直接 complete；`usage_source='simulated'` | 作业"完成态"与实耗均非真实 |
| K8s 客户端为 mock | `services/k8s.rs`：`MockK8sClient` 固定 2 Pod / 3 节点；真实 kube-rs 未引入 | `/k8s/*` 与三个 Tab 均无真实编排数据 |
| 生产 fail-closed | `config.rs` / `handlers/system.rs`：生产未设 `ALLOW_SIMULATED_EXECUTION` 则拒绝启动；`/system/mode` 驱动前端常驻横幅 | 用户已被明确告知演示模式 |
| 监控趋势合成 | `MonitoringAlert.vue`：`buildTrend` 用 `Math.sin`，吞吐用 `i%3` 规律 | CPU/内存/磁盘/网络趋势非采集值 |
| 后端有能力、前端无入口 | `/users` CRUD、`/auth/profile`、`/auth/change-password`、alert CRUD、告警规则、security create/delete/enable/disable 等 | 需产品决策是否新增菜单，非 bug |

> 这些都是**真实能力缺口**。系统通过 `ModeBanner` 常驻横幅诚实声明演示模式，
> 本轮新增的 K8S 标注进一步把语义借用到页面上明示。

---

## 6. 验证

| 检查 | 结果 |
|---|---|
| `type-check` | 0 错误 |
| `vitest` | **74/74 通过**（上轮 71 + 本轮 3：Dashboard admin/无角色双分支、告警权限与禁用态、K8S 来源标注） |
| `vite build` | 成功，18s |
| 真实浏览器 DOM 核验 | 登录 + 三页 DOM：Dashboard 无假控件（残留文案仅在我写的 HTML 注释里）、admin 租户卡正常 `href="/tenant"`、K8S 标注可见、告警禁用 span 存在 |

新增测试全部是**加固**：把"权限口径"与"诚实性标注"从口头约定变成可执行断言。

---

## 7. 直接回答用户问题

> **"是否已完善了所有功能？"**

**没有，也不应该声称已经完善。**

- **完善的**：菜单/路由结构、14 个模块中的 8 个核心 CRUD 闭环、认证与 RBAC 三层口径。
- **部分完善的**：5 个模块（功能可用但缺新增/删除/独立详情，或展示项非真数据）。
- **本质未完成的**：真实算力执行、真实 K8s 编排、真实监控时序——这三项目前是**后端能力缺口**，
  前端能做的是诚实标注而非伪造。本轮已经把三处"看起来有、实际没有"的伪装（假时间切换、
  空按钮、语义借用）清理或标注掉。
- **下一步优先级**：① 真实执行器（kube-rs Job/CRD，见 `docs/real-executor-contract.md`）；
  ② 监控时序数据源；③ 用户管理/告警规则等"后端有、前端无入口"的产品决策；
  ④ Topology 字段口径核对。

---

## 8. 边界

- 本轮仅改前端 3 个页面 + 3 个测试文件，未改后端、CI、部署开关。
- `DEPLOY_ENABLED` 仍关闭；根事实未变（执行体模拟、实耗合成、K8s mock）。
- 改动尚未提交，等待用户确认。
