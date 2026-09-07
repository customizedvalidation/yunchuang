# CI/CD 工作流已迁移

> **重要提示**：本目录下的 CI/CD 工作流已迁移至仓库根目录。

## 迁移说明

原工作流文件 `ci-cd.yml` 已从 `metaclouds-backend/.github/workflows/` 迁移至仓库根目录 `.github/workflows/ci-cd.yml`。

### 迁移原因

GitHub Actions 仅识别仓库根目录下的 `.github/workflows/` 目录中的工作流文件。放置在子目录（如 `metaclouds-backend/.github/workflows/`）中的工作流不会被 GitHub 识别和执行。

### 新工作流位置

```
D:\YCYD\.github\workflows\ci-cd.yml
```

### 主要变更

1. **Monorepo 感知**：所有路径调整为 monorepo 相对路径，通过 `working-directory` 切换子项目
2. **拆分构建 Job**：后端和前端分别有独立的 Docker 构建 Job
3. **新增前端测试 Job**：包含 `npm ci`、`tsc --noEmit`、ESLint、`vite build`、`npm audit`
4. **新增 govulncheck**：Go 依赖漏洞扫描
5. **覆盖率阈值调整**：整体 25%（当前可达到），`pkg/priorityscheduler` ≥ 60%，附提升路线图 TODO
6. **构建产物缓存**：前端 `node_modules` 缓存
7. **Artifact 上传**：后端二进制、前端 `dist` 均作为 artifact 上传
8. **迁移验证**：在 backend-test Job 中添加迁移文件完整性检查

### 本目录

本目录保留为空，仅用于说明迁移历史。请勿在此目录下新增工作流文件。
