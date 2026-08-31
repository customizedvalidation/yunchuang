# Metaclouds Deploy Scripts

## 📁 脚本清单

| 脚本 | 说明 | 必需 |
|------|------|------|
| [install.sh](install.sh) | 一键安装脚本 | 推荐 |
| [verify_deployment.sh](verify_deployment.sh) | 部署验证脚本 | 推荐 |
| [backup_postgresql.sh](backup_postgresql.sh) | PostgreSQL 备份脚本 | 是 |
| [verify_backup.sh](verify_backup.sh) | 备份验证脚本 | 可选 |
| [health_check.sh](health_check.sh) | 健康检查脚本 | 可选 |
| [check_disk_space.sh](check_disk_space.sh) | 磁盘空间检查脚本 | 可选 |
| [collect_metrics.sh](collect_metrics.sh) | Prometheus 指标收集脚本 | 可选 |

## 🚀 快速开始

### 1. 安装依赖
```bash
# Debian/Ubuntu
sudo apt update
sudo apt install postgresql-client coreutils curl

# 运行安装脚本
chmod +x install.sh
./install.sh
```

### 2. 验证部署
```bash
# 设置环境变量
export DATABASE_PASSWORD=your_password

# 运行验证脚本
./verify_deployment.sh
```

### 3. 测试备份
```bash
# 列出备份
./backup_postgresql.sh list

# 创建备份
./backup_postgresql.sh daily
```

## 📋 配置文件

### logrotate 配置
- [metaclouds.logrotate](metaclouds.logrotate) - 日志轮转配置

### cron 配置
- [metaclouds-cron](metaclouds-cron) - 定时任务配置

## 🔧 配置

### 环境变量
详细的环境变量配置请参考 [DEPENDENCIES.md](DEPENDENCIES.md)。

### 必需配置
```bash
export DATABASE_HOST=localhost
export DATABASE_PORT=5432
export DATABASE_NAME=metaclouds
export DATABASE_USER=metaclouds_user
export DATABASE_PASSWORD=your_secure_password
```

### 可选配置
```bash
# S3 备份上传
export S3_BUCKET=your-bucket
export S3_PREFIX=metaclouds/backups

# 备份加密
export BACKUP_ENCRYPTION_KEY=your_gpg_key

# 告警
export ALERT_EMAIL=admin@example.com
```

## 🏗️ 架构

```
/var/log/metaclouds/           # 日志目录
├── backup.log                 # 备份日志
├── backup_daily.log          # 每日备份日志
├── backup_hourly.log         # 每小时备份日志
├── backup_weekly.log         # 每周备份日志
├── health_check.log          # 健康检查日志
└── disk_space.log            # 磁盘空间日志

/var/backups/metaclouds/       # 备份目录
├── metaclouds_daily_20240527_020000.dump.gz
├── metaclouds_weekly_20240521_030000.dump.gz
└── ...
```

## ⏰ 定时任务

| 时间 | 任务 | 说明 |
|------|------|------|
| 每小时 | backup_postgresql.sh hourly | 小时备份（保留24小时） |
| 每天 02:00 | backup_postgresql.sh daily | 日备份（保留30天） |
| 每周日 03:00 | backup_postgresql.sh weekly | 周备份（保留12周） |
| 每5分钟 | health_check.sh | 健康检查 |
| 每小时 | check_disk_space.sh | 磁盘空间检查 |
| 每天 04:00 | find ... -mtime +30 | 清理旧日志 |

## 🐛 故障排查

### 查看日志
```bash
# 查看备份日志
tail -f /var/log/metaclouds/backup.log

# 查看健康检查日志
tail -f /var/log/metaclouds/health_check.log

# 查看所有日志
ls -la /var/log/metaclouds/
```

### 手动运行
```bash
# 测试备份
./backup_postgresql.sh daily

# 测试验证
./verify_backup.sh latest

# 测试健康检查
./health_check.sh

# 测试磁盘空间检查
./check_disk_space.sh
```

## 📊 监控

### Prometheus 指标
```bash
# 收集指标
./collect_metrics.sh

# 查看指标
curl http://localhost:8000/metrics
```

### 健康检查端点
```bash
# API 健康检查
curl http://localhost:8000/health

# 数据库健康检查
pg_isready -h localhost -U metaclouds_user
```

## 🔒 安全

### 权限设置
```bash
# 设置脚本权限
chmod 700 *.sh
chmod 600 *.env

# 设置目录权限
chown -R root:adm /var/log/metaclouds
chmod 750 /var/log/metaclouds
```

### 备份加密
```bash
# 使用 GPG 加密备份
export BACKUP_ENCRYPTION_KEY=your_secure_key
./backup_postgresql.sh daily
```

## 📝 版本历史

- **v1.1.0** (2024-05)
  - 添加跨平台兼容性支持
  - 添加依赖检查函数
  - 添加部署验证脚本
  - 添加一键安装脚本

- **v1.0.0** (2024-04)
  - 初始版本
  - 备份脚本
  - 健康检查
  - 磁盘空间监控
