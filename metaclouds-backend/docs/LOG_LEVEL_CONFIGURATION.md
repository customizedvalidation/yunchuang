# 日志级别控制配置指南

## 概述

本文档详细描述了 Metaclouds 后端服务的日志级别控制功能，包括配置方式、使用方法、CI/CD 集成以及最佳实践。

---

## 1. 日志级别说明

### 支持的日志级别

| 级别 | 优先级 | 说明 | 适用场景 |
|------|--------|------|----------|
| `DEBUG` | 4 | 最详细的调试信息，包括函数调用、参数值、中间结果 | 开发调试、问题排查 |
| `INFO` | 3 | 一般业务信息，如服务启动、关键操作完成 | 测试环境、日常监控 |
| `WARN` | 2 | 警告信息，如资源不足、潜在问题 | 生产环境 |
| `ERROR` | 1 | 仅错误信息，如系统异常、操作失败 | 生产环境（最小日志） |

### 日志级别继承关系

- 设置为 `DEBUG` 时，会输出 **DEBUG + INFO + WARN + ERROR**
- 设置为 `INFO` 时，会输出 **INFO + WARN + ERROR**
- 设置为 `WARN` 时，会输出 **WARN + ERROR**
- 设置为 `ERROR` 时，仅输出 **ERROR**

---

## 2. 配置方式

### 2.1 环境变量配置

通过环境变量 `LOG_LEVEL` 设置应用日志级别：

```bash
# 开发环境
export LOG_LEVEL=debug

# 生产环境
export LOG_LEVEL=warn
```

### 2.2 命令行参数配置

部署脚本支持 `-l/--log-level` 参数：

```bash
# 使用命令行参数覆盖默认配置
./deploy.sh -e production -l DEBUG

# 查看帮助
./deploy.sh -h
```

### 2.3 环境配置文件

每个环境有独立的配置文件，在 `.env.{env}` 中设置：

```bash
# .env.development
LOG_LEVEL=debug
LOG_FORMAT=text

# .env.staging
LOG_LEVEL=info
LOG_FORMAT=json

# .env.production
LOG_LEVEL=warn
LOG_FORMAT=json
```

### 2.4 端口工具库日志级别

端口工具库 `port_utils.sh` 支持独立的日志级别控制：

```bash
# 通过环境变量设置
export PORT_UTILS_LOG_LEVEL=DEBUG

# 通过函数动态设置
port_utils_set_log_level "DEBUG"
port_utils_enable_debug   # 快捷函数
```

---

## 3. 使用方法

### 3.1 启动应用时配置

```bash
# 方式1：直接设置环境变量
LOG_LEVEL=debug ./metaclouds-backend

# 方式2：使用部署脚本
./deploy.sh -e development -l DEBUG

# 方式3：使用 docker-compose
docker-compose up -d  # 从 docker-compose.yml 读取配置
```

### 3.2 动态调整日志级别

在脚本中动态调整日志级别：

```bash
#!/bin/bash
source deploy/utils/port_utils.sh

# 设置日志级别
port_utils_set_log_level "DEBUG"

# 获取当前日志级别
current_level=$(port_utils_get_log_level)
echo "Current log level: $current_level"

# 快捷切换函数
port_utils_enable_debug
port_utils_enable_info
port_utils_enable_warn
port_utils_enable_error
```

### 3.3 在 Go 代码中使用

```go
// main.go
logger.InitGlobalLogger(cfg.LogLevel)

// 使用日志
logger.Debug("Debug message")
logger.Info("Info message")
logger.Warn("Warning message")
logger.Error("Error message")
```

---

## 4. CI/CD 集成

### 4.1 GitHub Actions 配置

在 `.github/workflows/ci-cd.yml` 中配置：

```yaml
workflow_dispatch:
  inputs:
    environment:
      description: 'Target environment'
      required: true
      default: 'staging'
      type: choice
      options:
        - development
        - staging
        - production
    log_level:
      description: 'Log level for deployment'
      required: true
      default: 'INFO'
      type: choice
      options:
        - DEBUG
        - INFO
        - WARN
        - ERROR

jobs:
  deploy-production:
    environment:
      name: production
    env:
      LOG_LEVEL: WARN
    steps:
      - name: Deploy
        run: ./deploy.sh -e production -l ${{ inputs.log_level }}
        env:
          PORT_UTILS_LOG_LEVEL: ${{ inputs.log_level }}
```

### 4.2 环境默认日志级别

| 环境 | 默认日志级别 | Docker Build Arg |
|------|--------------|-----------------|
| Development | DEBUG | `LOG_LEVEL=DEBUG` |
| Staging | INFO | `LOG_LEVEL=INFO` |
| Production | WARN | `LOG_LEVEL=WARN` |

---

## 5. 日志输出格式

### 5.1 支持的格式

| 格式 | 说明 | 适用场景 |
|------|------|----------|
| `text` | 人类可读的文本格式 | 开发调试 |
| `json` | JSON 格式，便于日志解析 | 生产环境、日志收集 |

### 5.2 配置方式

```bash
# 通过环境变量设置
export LOG_FORMAT=json

# 在 .env 文件中设置
LOG_FORMAT=json
```

### 5.3 输出示例

**Text 格式：**
```
[INFO] 2024-01-15 10:30:00 - Server started on :8000
[DEBUG] 2024-01-15 10:30:01 - GPU allocated: 2
```

**JSON 格式：**
```json
{"level":"INFO","time":"2024-01-15T10:30:00Z","message":"Server started on :8000"}
{"level":"DEBUG","time":"2024-01-15T10:30:01Z","message":"GPU allocated: 2","cluster_id":1}
```

---

## 6. 最佳实践

### 6.1 环境日志级别建议

| 环境 | 推荐级别 | 理由 |
|------|----------|------|
| 开发环境 | DEBUG | 需要详细调试信息 |
| 测试环境 | INFO | 需要了解业务流程 |
| 预发布环境 | INFO | 接近生产环境，便于问题排查 |
| 生产环境 | WARN | 减少日志量，仅关注重要问题 |

### 6.2 临时调试

在生产环境遇到问题时，可以临时提高日志级别：

```bash
# 临时设置为 DEBUG 进行调试
export LOG_LEVEL=debug
./deploy.sh -e production -l DEBUG

# 调试完成后恢复
export LOG_LEVEL=warn
./deploy.sh -e production -l WARN
```

### 6.3 日志轮转

建议在生产环境配置日志轮转，避免日志文件过大：

```bash
# /etc/logrotate.d/metaclouds
/var/log/metaclouds/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 644 www-data www-data
}
```

---

## 7. 相关文件

| 文件 | 路径 | 说明 |
|------|------|------|
| 部署脚本 | [`deploy.sh`](../deploy.sh) | 支持 `-l` 参数设置日志级别 |
| 端口工具库 | [`deploy/utils/port_utils.sh`](../deploy/utils/port_utils.sh) | 日志级别控制函数 |
| 环境配置 | `.env.*` | 各环境的日志级别配置 |
| CI/CD 配置 | [`.github/workflows/ci-cd.yml`](../.github/workflows/ci-cd.yml) | GitHub Actions 集成 |
| Logger 实现 | [`pkg/logger/logger.go`](../pkg/logger/logger.go) | Go 日志库实现 |

---

## 8. 故障排除

### 8.1 日志不输出

**问题**：设置了 `DEBUG` 级别但没有看到 DEBUG 日志

**解决方案**：
1. 检查环境变量是否正确设置：`echo $LOG_LEVEL`
2. 检查配置文件是否正确加载
3. 确认应用已重新启动

### 8.2 日志格式不正确

**问题**：期望 JSON 格式但输出文本格式

**解决方案**：
1. 检查 `LOG_FORMAT` 环境变量
2. 确认配置文件中的设置

### 8.3 CI/CD 日志级别不生效

**问题**：GitHub Actions 中设置的日志级别没有生效

**解决方案**：
1. 检查 workflow 文件中的环境变量设置
2. 确认部署脚本正确接收 `-l` 参数
3. 检查 Docker build arg 是否传递正确

---

## 附录：日志级别控制函数参考

| 函数 | 说明 | 参数 |
|------|------|------|
| `port_utils_set_log_level(level)` | 设置日志级别 | `level`: DEBUG/INFO/WARN/ERROR |
| `port_utils_get_log_level()` | 获取当前日志级别 | 无 |
| `port_utils_is_valid_log_level(level)` | 验证日志级别是否有效 | `level`: 日志级别字符串 |
| `port_utils_should_log(level)` | 检查是否应该输出指定级别 | `level`: 日志级别 |
| `port_utils_enable_debug()` | 快捷设置为 DEBUG | 无 |
| `port_utils_enable_info()` | 快捷设置为 INFO | 无 |
| `port_utils_enable_warn()` | 快捷设置为 WARN | 无 |
| `port_utils_enable_error()` | 快捷设置为 ERROR | 无 |

---

**版本**: v1.0  
**最后更新**: 2024-01-15  
**适用项目**: Metaclouds Backend