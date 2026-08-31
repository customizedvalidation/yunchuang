# GPU资源分配监控仪表盘部署指南

## 概述

本指南描述如何将GPU资源分配监控仪表盘集成到Grafana中，实时展示GPU分配和释放的监控数据。

## 仪表盘文件位置

```
deploy/grafana/dashboards/gpu-allocation-dashboard.json
```

## 前置条件

1. 已安装并运行 Grafana（推荐版本 8.0+）
2. 已配置 Prometheus 数据源
3. 后端服务已配置Prometheus指标端点

## 导入仪表盘

### 方法一：通过Grafana UI导入

1. 登录 Grafana 管理界面
2. 点击左侧菜单的 **Dashboards** -> **Manage**
3. 点击 **Import** 按钮
4. 选择 **Upload JSON file**
5. 选择 `gpu-allocation-dashboard.json` 文件
6. 选择对应的 Prometheus 数据源
7. 点击 **Import** 完成导入

### 方法二：通过API导入

```bash
curl -X POST http://<grafana-host>:<port>/api/dashboards/db \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <api-token>" \
  -d @deploy/grafana/dashboards/gpu-allocation-dashboard.json
```

### 方法三：配置Grafana自动加载（推荐）

修改 Grafana 配置文件 `grafana.ini`：

```ini
[dashboards]
enabled = true
path = /path/to/dashboards

[dashboard.provisioning]
providers = dashboardproviders.yaml
```

创建 `dashboardproviders.yaml`：

```yaml
apiVersion: 1
providers:
  - name: 'default'
    orgId: 1
    folder: ''
    type: file
    disableDeletion: false
    updateIntervalSeconds: 10
    options:
      path: /path/to/deploy/grafana/dashboards
```

## 仪表盘面板说明

### 1. GPU资源分配概览

| 面板 | 说明 | PromQL查询 |
|------|------|-----------|
| Total GPUs Allocated | 总分配GPU数量 | `sum(gpu_allocated_total)` |
| Total GPUs Released | 总释放GPU数量 | `sum(gpu_released_total)` |
| Net GPU Usage | 净GPU使用量 | `sum(gpu_allocated_total) - sum(gpu_released_total)` |

### 2. GPU分配速率

| 面板 | 说明 | PromQL查询 |
|------|------|-----------|
| GPU Allocation Rate | GPU分配速率（GPUs/秒） | `rate(gpu_allocated_total[5m])` |
| GPU Release Rate | GPU释放速率（GPUs/秒） | `rate(gpu_released_total[5m])` |

### 3. 按集群统计

| 面板 | 说明 | PromQL查询 |
|------|------|-----------|
| GPU Allocation by Cluster | 各集群GPU分配情况 | `gpu_allocated_total` |
| GPU Release by Cluster | 各集群GPU释放情况 | `gpu_released_total` |

### 4. 分配失败监控

| 面板 | 说明 | PromQL查询 |
|------|------|-----------|
| GPU Allocation Failure Rate | GPU分配失败率 | `rate(gpu_allocation_failed_total[5m])` |

## 配置告警规则

建议配置以下告警规则：

### GPU分配失败告警

```yaml
groups:
- name: gpu_allocation_alerts
  rules:
  - alert: GPUAllocationFailureRateHigh
    expr: rate(gpu_allocation_failed_total[5m]) > 1
    for: 1m
    labels:
      severity: critical
    annotations:
      summary: "GPU分配失败率过高"
      description: "过去5分钟内GPU分配失败率超过1次/秒"
```

### GPU资源耗尽告警

```yaml
- alert: GPUResourceExhausted
  expr: sum(gpu_allocated_total) / sum(gpu_total) > 0.9
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "GPU资源即将耗尽"
    description: "GPU使用率已超过90%"
```

## 注意事项

1. **刷新频率**：仪表盘默认刷新间隔为5秒，可根据实际需求调整
2. **时间范围**：默认显示最近6小时数据
3. **数据源**：确保已正确配置Prometheus数据源
4. **权限**：导入仪表盘需要Grafana管理员权限

## 故障排除

### 问题：仪表盘显示"No data"

**可能原因：**
1. Prometheus数据源未正确配置
2. 后端服务未正确暴露metrics端点
3. 网络连接问题

**解决方案：**
1. 检查Grafana数据源配置
2. 验证Prometheus能否抓取到指标：
   ```bash
   curl http://<backend-host>:<port>/metrics | grep gpu_allocated
   ```
3. 检查防火墙和网络配置

### 问题：某些面板没有数据

**可能原因：**
1. 对应指标尚未产生数据
2. 指标名称不匹配

**解决方案：**
1. 确认后端服务已启动并运行
2. 确认已提交过作业（触发GPU分配）
3. 检查Prometheus指标名称是否正确

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v1.0 | 2026-05-25 | 初始版本 |
