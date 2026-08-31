# Metaclouds 部署指南

本指南说明了如何使用部署脚本部署 Metaclouds 后端服务，以及如何处理常见问题如端口被占用。

## 快速开始

### Linux/macOS

```bash
# 部署生产环境（默认）
./deploy.sh

# 部署并自动杀死占用端口的进程
./deploy.sh -k

# 后台运行
./deploy.sh -d
```

### Windows

```cmd
# 部署生产环境（默认）
deploy.bat

# 部署并自动杀死占用端口的进程
deploy.bat /k

# 后台运行
deploy.bat /d
```

## 脚本参数说明

### Linux/macOS (deploy.sh)

| 参数 | 说明 |
|------|------|
| `-h, --help` | 显示帮助信息 |
| `-e, --env ENV` | 指定环境（production/staging/development）|
| `-b, --build` | 仅构建，不启动 |
| `-s, --start` | 仅启动，不构建 |
| `-d, --daemon` | 后台运行 |
| `-k, --kill` | 自动杀死占用端口的进程 |

### Windows (deploy.bat)

| 参数 | 说明 |
|------|------|
| `/h, /help` | 显示帮助信息 |
| `/e:ENV` | 指定环境（production/staging/development）|
| `/b` | 仅构建，不启动 |
| `/s` | 仅启动，不构建 |
| `/d` | 后台运行 |
| `/k` | 自动杀死占用端口的进程 |

## 端口被占用处理

### 场景：端口被占用

当启动服务时，如果配置的端口被占用，脚本会自动检测并提供处理选项。

#### 方案 1：自动杀死进程（推荐开发环境）

```bash
# Linux/macOS
./deploy.sh -k

# Windows
deploy.bat /k
```

脚本会自动查找并杀死占用端口的进程。

#### 方案 2：交互式选择（生产环境）

不使用 `-k` 参数时，脚本会提示以下选项：

```
Options:
1) Kill the existing process and start anyway
2) Change SERVER_PORT in .env file
3) Cancel deployment
```

**选项 1**：杀死现有进程并启动
**选项 2**：修改 .env 文件中的端口号
**选项 3**：取消部署

#### 方案 3：手动修改端口

可以先手动编辑 `.env` 文件，修改 `SERVER_PORT` 配置：

```bash
# 编辑 .env 文件
vim .env

# 修改端口配置
SERVER_PORT=8001
```

然后再运行部署脚本。

## 常见问题

### Q: 如何查看哪个进程占用了端口？

**Linux/macOS:**
```bash
lsof -i :8000
```

**Windows:**
```cmd
netstat -ano | findstr ":8000"
```

### Q: 如何手动杀死占用端口的进程？

**Linux/macOS:**
```bash
kill -9 <PID>
```

**Windows:**
```cmd
taskkill /F /PID <PID>
```

### Q: 如何检查服务是否正常运行？

**Linux/macOS:**
```bash
# 检查 PID 文件
cat metaclouds.pid

# 检查进程
ps aux | grep metaclouds-backend

# 检查端口
curl http://localhost:8000/health
```

**Windows:**
```cmd
# 检查进程
tasklist | findstr metaclouds-backend

# 检查端口（在浏览器中访问）
# http://localhost:8000/health
```

## 部署流程

1. **验证环境文件** - 检查 `.env.{ENV}` 文件是否存在
2. **复制配置** - 将 `.env.{ENV}` 复制为 `.env`
3. **构建应用** - 使用 Go 编译二进制文件
4. **检查端口** - 检测配置的端口是否被占用
5. **启动服务** - 前台或后台运行应用程序

## 日志文件

- **Linux/macOS 后台模式**: `/var/log/metaclouds.log`
- **前台模式**: 标准输出

## 故障排查

### 部署失败 - 权限不足

**问题**: `permission denied`

**解决**:
```bash
chmod +x deploy.sh
```

### 部署失败 - Go 未安装

**问题**: `Go is not installed`

**解决**: 安装 Go 1.18+

### 部署失败 - 端口仍被占用

**问题**: 杀死进程后端口仍被占用

**解决**: 等待几秒钟后重试，或重启相关服务

### 端口配置未生效

**问题**: 服务仍然使用旧端口

**解决**: 
1. 检查 `.env` 文件是否正确更新
2. 确认服务已完全停止
3. 重新启动服务
