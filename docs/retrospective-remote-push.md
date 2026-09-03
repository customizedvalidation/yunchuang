# 复盘：③ 远程仓库配置与推送

**日期**：2026-09-03
**仓库**：`https://github.com/customizedvalidation/yunchuang.git`
**结果**：✅ 推送成功，远端 `main` = `c119538`，本地与远端 `ahead 0 / behind 0` 完全同步

---

## 1. TL;DR

把本地 22 笔提交推送到新建的 GitHub 空仓库。过程中识别出「远端是空仓库、与本地历史无共同祖先」这一破坏性分叉，
经用户确认后采用强制推送；推送前先把远端原有 README 内容并入本地历史避免丢失。
主要阻力是沙箱出口代理对 `github.com` 的**间歇性阻断**，最终靠重试循环在第 3 次撞上连接窗口成功。

---

## 2. 交付清单（22 笔提交，最早 → 最新）

| # | Hash | 说明 |
|---|------|------|
| 1 | `2e64468` | feat(ui): MDS 设计令牌落地——全局顶栏 + 表格三态 + 深色模式 + 命令面板 + 可拖拽仪表盘 |
| 2 | `797c196` | chore(repo): 收口后端源码与顶层配置，纳入单一工作区仓库 |
| 3 | `c9722e4` | feat(ui): 8 个业务页面接入 MDS 设计系统，统一三态与状态色 |
| 4 | `be94102` | fix(ui): 收口 MDS 一致性问题与运行时告警 |
| 5 | `2b69196` | fix(fullstack): RBAC 接线、注册开关、401 修复与类型/中间件对齐 |
| 6 | `3a24ea8` | fix(config): 仅在启用限流时校验限流参数，修复生产启动陷阱 |
| 7 | `44881ed` | fix(validation): 拒绝租户/作业创建的负值配额与资源 |
| 8 | `88112a3` | fix(validation): 集群创建/更新同样拒绝负值资源 |
| 9 | `0ac50ae` | fix(frontend): 登录成功后落盘用户信息，修正顶栏身份显示 |
| 10 | `edeb437` | fix(validation): 资源更新拒绝负值 total/used/available/utilization |
| 11 | `1252ea9` | fix(auth): profile 契约对齐、菜单/路由按 role 收敛、接入静默续期 |
| 12 | `381b540` | fix(frontend): 按角色收敛写操作按钮可见性 |
| 13 | `72a95ff` | docs: 新增操作级权限收敛前后端对照清单 |
| 14 | `30c6c32` | feat(frontend): 响应式改版 M1-M4 落地 + 四档视口真机走查 |
| 15 | `cfc4dfc` | refactor(frontend): R3-b 侧边栏宽度真相合并 |
| 16 | `441f487` | docs: 前端响应式改版复盘报告（M1-M4 + R3-b） |
| 17 | `3f09e5f` | feat(frontend): Q6 密度切换（紧凑/舒适，localStorage 持久化） |
| 18 | `7acfda9` | refactor(frontend): R15 K8SManagement 内联魔法值收敛为 MDS 栅格类 |
| 19 | `75e8cb6` | feat(frontend): R19 监控页分栏 |
| 20 | `35be994` | fix(frontend): Q6 补 `DensityMode` 类型导入（构建门禁前置修复） |
| 21 | `cef252b` | fix(frontend): Q6 补 `useDensity` 导入（构建门禁前置修复） |
| 22 | `c119538` | docs: 并入远端仓库原有 README（yunchuang 项目简介） |

> 注：#20、#21 是补跑构建门禁时由 `tsc` 抓出的真实缺陷（详见第 5 节），#22 是为避免覆盖丢失而并入的远端 README。

---

## 3. 时间线与关键决策点

### 决策点 1：远端是空仓库 → 是否强制推送（**已征求用户**）

`git fetch` 后发现：

```
远端 main：acc28ca Update README.md → 9b05ca1 Initial commit   （仅 2 笔）
本地 main：21 笔，最早 2e64468
git merge-base main origin/main  →  NO COMMON ANCESTOR（无共同祖先）
```

即这不是「本地领先可快进」，也不是普通分叉，而是**两条互不相关的历史**。
强制推送会**丢弃远端原有 2 笔历史**，属破坏性操作，因此**没有自行决定**，而是列了三个选项请用户拍板：

1. 强制推送覆盖（若此仓库确为新建用于承载本项目）
2. 合并无关历史（`--allow-unrelated-histories`），保留双方历史
3. URL 给错了，换正确仓库

**用户选择：强制推送覆盖。**

> **复盘要点**：这一步是对的。面对 `NO COMMON ANCESTOR` + 强制推送这种不可逆操作，
> 停下来把选项和后果摆清楚，比「猜一个最可能的」然后再补救要安全得多。

### 决策点 2：远端 README 的保留（承诺并已执行）

强制推送会连远端 README 一起覆盖。但检查发现该 README 是**真实内容**，不是空壳：

```
# yunchuang
服务器研发测试用例与工厂灰度测试用例开发与系统部署
RAM OS的深度定制（linux Kernel）
DQA流程与规范的制定与落地
迭代更新测试用例与脚本
深入AI的研发与实验室评审
```

因此在强制推送**之前**，先 `git show origin/main:README.md > README.md` 并单独提交（`c119538`）。
本地原本没有 README.md，**零冲突**，远端内容被完整保留进新历史的最后一笔。

> **复盘要点**：破坏性操作前先做「有价值的存量内容」盘点，是低成本高收益的一步。
> 判断依据是内容实质（是否为空壳），而不是文件是否存在。

### 决策点 3：网络间歇性阻断 → 重试而非判死

推送过程的报错：

```
fatal: unable to access 'https://github.com/customizedvalidation/yunchuang.git/':
Failed to connect to github.com:443 after 21091 ms: Could not connect to server
```

但同时观察到：
- `git ls-remote`、`git fetch` 曾**成功**过；
- 失败是 `curl 28`（连接超时），不是 403/401；
- 仓库体积很小（`size-pack: 704.48 KiB`，395 个对象），不是大包超时。

结论是**间歇性阻断**而非「完全不可达」，于是改用重试循环：

```bash
for i in 1 2 3 4 5 6; do
  GIT_TERMINAL_PROMPT=0 git push -f -u origin main && break
  sleep 8
done
```

**第 1、2 次失败，第 3 次成功**：

```
To https://github.com/customizedvalidation/yunchuang.git
 + acc28ca...c119538 main -> main (forced update)
branch 'main' set up to track 'origin/main'.
```

> **复盘要点**：早期一度把「github 502」判定为硬性不可达并准备让用户自行推送。
> 这个判定**过于草率**——单次失败不能证明持续不可达。用重试循环验证后，问题当场解决。

---

## 4. 两个一度被误判、后来纠正的点

| 现象 | 初判 | 纠正后的真相 |
|------|------|-------------|
| `CONNECT tunnel failed, response 502` | 沙箱永久阻断 github，只能由用户侧推送 | 代理**间歇性**放行；重试第 3 次即成功 |
| `git push` 挂起 8 分钟无输出 | 缺少推送凭据（GCM 等待输入） | 凭据**是有的**：`git credential fill` 返回 `username=customizedvalidation` + 已缓存 token；挂起是连接慢 |

凭据排查方式（**注意脱敏**，不打印密钥）：

```bash
printf "protocol=https\nhost=github.com\n\n" | git credential fill \
  | sed 's/\(password=\).*/\1***MASKED***/'
```

---

## 5. 与构建门禁的交叉成果（重要）

本轮「补构建门禁」在推送前抓出两个真实缺陷，都属于**同一类根因**：
前序对同一文件的多处并行编辑发生竞态，导致 **import 行被回滚丢失**，而「用法」仍在。

| 文件 | 缺失导入 | 症状 | 修复提交 |
|------|---------|------|---------|
| `src/theme/antdTheme.ts` | `DensityMode` | `TS2304 Cannot find name 'DensityMode'` | `35be994` |
| `src/components/Topbar/Topbar.tsx:35` | `useDensity` | `TS2304 Cannot find name 'useDensity'` | `cef252b` |

**教训**：只 grep「符号的用法」不足以验证改动正确性，**必须同时核对 import 行是否存在**。
我在第一处修复后只对第二处做了用法 grep、没验 import，结果让 `Topbar` 的缺陷留到了构建阶段才暴露。

---

## 6. 做得对的地方

1. **破坏性操作前先征求用户**：强制推送前摆清三个选项与各自后果，而非自行决定。
2. **覆盖前先抢救存量内容**：远端 README 实质内容已并入 `c119538`，未丢失。
3. **用 `ls-remote` / `fetch` / `merge-base` 做前置体检**，在推送前就发现「无共同祖先」，避免了盲推失败或更糟的历史污染。
4. **失败后没有直接放弃**，而是用重试循环把「间歇性」问题解决掉。
5. **推送后做了闭环验证**：`ahead 0 / behind 0`、远端 HEAD = `c119538`、分支跟踪已建立。

## 7. 教训与改进

| 教训 | 改进建议 |
|------|---------|
| 单次网络失败被误判为「永久不可达」 | 网络类失败默认按**间歇性**处理：先重试 N 次再下结论，不要一次失败就转交用户 |
| 只验「用法」不验 import | 改动涉及新增跨文件符号时，同时 grep 用法 **和** import 行 |
| 对同一文件的多处编辑并行发出 | 同文件多处修改**顺序单发**，避免竞态回滚（前序已踩过，本轮再次应验） |
| 长耗时命令未加超时保护，挂起 8 分钟 | 网络类命令统一加 `timeout`，配合后台执行 |

## 8. 遗留事项与建议

1. **远端 `main` 现在是单一线性历史**（原 2 笔 README 历史已被覆盖）。若未来需要追溯到 `acc28ca`，
   只能靠 GitHub 的 reflog/API，本地已无该引用——如有保留需要请尽早确认。
2. **仓库根在 `D:\YCYD`**（`metaclouds-frontend` 只是子目录），推送的是整个工作区（backend + frontend + docs）。
   若后续希望前后端分仓，需要做 subtree/filter-repo 拆分，越早做成本越低。
3. **未跟踪文件未推送**：`dist-verify/`、`*.webp/jpg`（位于 `D:\YCYD` 根目录）等仍在工作区且未纳入版本控制，
   建议确认是否需要清理或加入 `.gitignore`。
4. **后续优化项**：构建产物中 `ResponsiveChart` 1.05MB（ECharts）、antd 核心 544kB 超 500kB 告警，
   可用 `build.rollupOptions.output.manualChunks` 拆分。
