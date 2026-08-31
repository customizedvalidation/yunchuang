# Metaclouds Scripts Dependencies

## 系统依赖

### 必需命令（Linux 核心工具）
这些命令在大多数 Linux 发行版中默认可用：

| 命令 | 包名 | 说明 |
|------|------|------|
| `bash` | bash | Bash shell |
| `date` | coreutils | 日期时间工具 |
| `mkdir` | coreutils | 创建目录 |
| `rm` | coreutils | 删除文件 |
| `cat` | coreutils | 查看文件内容 |
| `cut` | coreutils | 文本切割 |
| `head` | coreutils | 显示文件开头 |
| `tail` | coreutils | 显示文件结尾 |
| `awk` | gawk | 文本处理 |
| `sed` | sed | 流编辑器 |
| `grep` | grep | 文本搜索 |
| `find` | findutils | 文件查找 |
| `stat` | coreutils | 文件状态 |
| `sha256sum` | coreutils | SHA256 校验 |
| `curl` | curl | HTTP 客户端 |

### 可选命令

#### PostgreSQL 相关（备份功能必需）
| 命令 | 包名 | 安装命令 |
|------|------|---------|
| `pg_dump` | postgresql-client | `apt install postgresql-client` |
| `pg_isready` | postgresql-client | `apt install postgresql-client` |
| `pg_restore` | postgresql-client | `apt install postgresql-client` |

#### AWS S3 上传（可选）
| 命令 | 包名 | 安装命令 |
|------|------|---------|
| `aws` | awscli | `pip install awscli` |

#### GPG 加密（可选）
| 命令 | 包名 | 安装命令 |
|------|------|---------|
| `gpg` | gnupg | `apt install gnupg` |

#### Redis 检查（可选）
| 命令 | 包名 | 安装命令 |
|------|------|---------|
| `redis-cli` | redis-tools | `apt install redis-tools` |

#### 邮件发送（可选）
| 命令 | 包名 | 安装命令 |
|------|------|---------|
| `sendmail` | sendmail | `apt install sendmail` |

## 依赖检查

### 快速检查脚本
运行以下命令检查所有依赖：

```bash
./verify_deployment.sh
```

### 手动检查

```bash
# 检查必需命令
for cmd in bash date mkdir rm cat cut head tail awk sed grep find stat sha256sum curl; do
    command -v "$cmd" && echo "✓ $cmd" || echo "✗ $cmd"
done

# 检查 PostgreSQL 命令
for cmd in pg_dump pg_isready pg_restore; do
    command -v "$cmd" && echo "✓ $cmd" || echo "✗ $cmd"
done
```

## 安装脚本

### Debian/Ubuntu
```bash
# 安装 PostgreSQL 客户端
sudo apt update
sudo apt install postgresql-client coreutils findutils gawk sed grep curl gnupg redis-tools

# 可选：AWS CLI
pip install awscli

# 可选：发送邮件
sudo apt install sendmail-bin
```

### RHEL/CentOS/Fedora
```bash
# 安装 PostgreSQL 客户端
sudo dnf install postgresql-client coreutils findutils gawk sed grep curl gnupg redis

# 可选：AWS CLI
pip install awscli

# 可选：发送邮件
sudo dnf install sendmail
```

### Alpine Linux
```bash
# 安装依赖
apk add bash postgresql-client coreutils findutils gawk sed grep curl gnupg2 redis bash
```

## 目录结构

### 必需目录
| 目录 | 权限 | 说明 |
|------|------|------|
| `/var/log/metaclouds` | 755 | 日志目录 |
| `/var/backups/metaclouds` | 755 | 备份目录 |
| `/run/metaclouds` | 755 | 运行时目录 |

### 自动创建
安装脚本会自动创建这些目录。

## 环境变量

### 必需环境变量
| 变量 | 说明 | 示例 |
|------|------|------|
| `DATABASE_HOST` | PostgreSQL 主机 | `localhost` |
| `DATABASE_PORT` | PostgreSQL 端口 | `5432` |
| `DATABASE_NAME` | 数据库名 | `metaclouds` |
| `DATABASE_USER` | 数据库用户 | `metaclouds_user` |
| `DATABASE_PASSWORD` | 数据库密码 | `your_password` |

### 可选环境变量
| 变量 | 说明 | 示例 |
|------|------|------|
| `S3_BUCKET` | S3 存储桶 | `my-backups` |
| `S3_PREFIX` | S3 前缀 | `metaclouds/backups` |
| `BACKUP_ENCRYPTION_KEY` | GPG 加密密钥 | `your_key` |
| `ALERT_EMAIL` | 告警邮箱 | `admin@example.com` |
| `REDIS_HOST` | Redis 主机 | `localhost` |
| `REDIS_PORT` | Redis 端口 | `6379` |
| `REDIS_PASSWORD` | Redis 密码 | `your_password` |
| `API_HOST` | API 主机 | `localhost` |
| `API_PORT` | API 端口 | `8000` |

## 兼容性

### Linux
- ✅ Ubuntu 18.04+
- ✅ Debian 10+
- ✅ CentOS 7+
- ✅ RHEL 7+
- ✅ Fedora 30+

### macOS
- ⚠️ 部分功能可能受限（`free`, `stat` 等命令行为不同）
- ⚠️ `date -d` 不可用

### Windows (WSL)
- ✅ 在 WSL2 环境下完全兼容

## 故障排查

### 命令未找到
```bash
# 检查命令是否安装
which <command>

# 查看完整路径
type <command>

# 安装缺失命令
sudo apt install <package>
```

### 权限问题
```bash
# 添加执行权限
chmod +x *.sh

# 设置目录权限
sudo mkdir -p /var/log/metaclouds /var/backups/metaclouds
sudo chown -R root:adm /var/log/metaclouds
sudo chmod 755 /var/log/metaclouds /var/backups/metaclouds
```

### 数据库连接问题
```bash
# 测试连接
PGPASSWORD=your_password psql -h localhost -U metaclouds_user -d metaclouds

# 检查 pg_isready
pg_isready -h localhost -p 5432 -U metaclouds_user
```
