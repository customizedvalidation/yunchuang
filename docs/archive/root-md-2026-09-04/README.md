# 根目录文档归档（2026-09-04）

本目录是从仓库根目录归档过来的 9 份 `.md` 文件，归档原因：

- **位置不当**：这些文件都是 2026-05-08 / 2026-05-27 "上线冲刺"期间直接堆在仓库根目录的；
  当时还没有 `docs/` 规范。自 2026-09-01 起项目统一用 `docs/` 做文档归宿
  （见 `docs/retrospective-frontend-redesign.md` 等同期复盘文档的命名风格），
  根目录应当保持干净，只放入口配置（`.env*`、`docker-compose.yml`、`init.sql`、
  `prometheus.yml`、构建/部署脚本等）。
- **过程产物**：`FINAL_VERIFICATION_REPORT.md` / `OPTIMIZATION_GUIDE.md` /
  `PRODUCTION_CONFIG_CHECK_REPORT.md` / `PRODUCTION_CONFIG_CHECK_REPORT_FINAL.md`
  都是带时间戳的一次性过程报告，标题里就有日期和 "FINAL" 字样，已经过期。
- **重复**：`PRODUCTION_CONFIG_CHECK_REPORT.md` 与 `_FINAL.md` 是同一份报告的两次迭代；
  `DEPLOYMENT.md` 与 `DEPLOYMENT_CHECKLIST.md` 内容部分重叠（部署指南 vs 部署清单）。

## 文件分类

| 文件 | 原判断 | 建议处理 |
|---|---|---|
| `ARCHITECTURE.md` | 5/8 写的系统架构（7KB），技术栈版本已陈旧 | 若需保留：**重新评审后**升级到 `docs/ARCHITECTURE.md`；否则保持归档 |
| `CHANGELOG.md` | 5/27 写的变更日志（3KB），**已 3 个月没更新** | 若需保留：迁到 `docs/CHANGELOG.md` 并补 6/7/8/9 月的 commit；否则归档 |
| `DEPLOYMENT.md` | 5/8 部署指南（5.7KB），与 `DEPLOYMENT_CHECKLIST.md` 部分重叠 | 已被 `docs/DEPLOYMENT_CHECKLIST.md`（更新更全）覆盖，可删 |
| `DEPLOYMENT_CHECKLIST.md` | 5/27 部署清单（12.9KB），较新 | 迁到 `docs/DEPLOYMENT_CHECKLIST.md` |
| `FINAL_VERIFICATION_REPORT.md` | 5/27 一次性验收报告 | 删（已过期 3 个月） |
| `OPTIMIZATION_GUIDE.md` | 5/8 一次性优化总结 | 删（已过期） |
| `PRODUCTION_CONFIG_CHECK_REPORT.md` | 5/27 过程产物 | 删（已被 `_FINAL` 覆盖） |
| `PRODUCTION_CONFIG_CHECK_REPORT_FINAL.md` | 5/27 过程产物 | 删（一次性，已过期） |
| `skill.md` | 5/8 "算力调度平台框架系统开发"立项/设计书（41KB） | 留档，作为项目立项原始记录 |

## 后续动作

- **本次提交只搬运，不删除**——保留 `git mv` 历史可逆。
- 下次"文档周"（任一周）按上表"建议处理"列做最终判决。
- 任何想"复活"的文档用 `git mv` 即可还原到原位置或新位置。
