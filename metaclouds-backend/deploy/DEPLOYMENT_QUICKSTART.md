# ==============================================================================
# Metaclouds 环境配置快速部署指南
# ==============================================================================

## 📁 环境配置文件清单

| 文件 | 说明 | 使用场景 |
|------|------|----------|
| `.env.production.example` | 生产环境配置 | 直接部署到生产服务器 |
| `.env.development.example` | 开发环境配置 | 本地开发测试 |
| `deploy/kubernetes/metaclouds-secrets.yaml` | Kubernetes Secret | K8s 部署 |

---

## 🚀 快速部署步骤

### Linux 服务器部署

```bash
# 1. 复制配置文件
cp .env.production.example .env.production

# 2. 编辑配置（必须修改必填项）
nano .env.production

# 3. 验证配置
chmod +x deploy/validate_env.sh
./deploy/validate_env.sh -f .env.production

# 4. 部署安装
chmod +x deploy/install.sh
./deploy/install.sh

# 5. 加载环境变量并启动
source .env.production
./metaclouds-backend
```

### Docker 部署

```bash
# 1. 复制并配置环境变量
cp .env.production.example .env

# 2. 编辑配置
nano .env

# 3. 验证配置
chmod +x deploy/validate_env.sh
./deploy/validate_env.sh

# 4. 启动容器
docker-compose -f docker/docker-compose.yml up -d
```

### Kubernetes 部署

```bash
# 1. 创建命名空间
kubectl create namespace metaclouds

# 2. 配置 Secret
# 修改 deploy/kubernetes/metaclouds-secrets.yaml 中的密码
nano deploy/kubernetes/metaclouds-secrets.yaml

# 3. 应用 Secret 和 ConfigMap
kubectl apply -f deploy/kubernetes/metaclouds-secrets.yaml

# 4. 应用其他 K8s 配置
kubectl apply -f deploy/kubernetes/backup-cronjob.yaml
kubectl apply -f deploy/kubernetes/deployment.yaml

# 5. 验证部署
kubectl get pods -n metaclouds
kubectl logs -n metaclouds -l app=metaclouds
```

---

## 🔐 必填配置（必须修改）

### 1. JWT 密钥
```bash
# 生成强密钥
openssl rand -base64 64

# 在 .env.production 中设置
SECURE_JWT_SECRET=生成的密钥粘贴到这里
```

### 2. 管理员密码
```bash
SECURE_DEFAULT_ADMIN_PASSWORD=YourStrongPassword123!
```

### 3. 数据库密码
```bash
SECURE_DATABASE_PASSWORD=YourDatabasePassword123!
```

### 4. CORS 源
```bash
# 生产环境必须配置
ALLOWED_ORIGINS=https://yourdomain.com,https://app.yourdomain.com
```

---

## ✅ 验证清单

部署前请确认：

- [ ] `SECURE_JWT_SECRET` 已修改（至少 32 字符）
- [ ] `SECURE_DEFAULT_ADMIN_PASSWORD` 已修改
- [ ] `SECURE_DATABASE_PASSWORD` 已修改
- [ ] `ALLOWED_ORIGINS` 已配置
- [ ] `DATABASE_HOST` 指向正确的数据库服务器
- [ ] `REDIS_HOST` 指向正确的 Redis 服务器
- [ ] `LOG_PATH` 目录存在且可写
- [ ] 运行 `./deploy/validate_env.sh -f .env.production` 通过

---

## 🔧 常用配置组合

### 最小配置（仅必需项）
```bash
# ⚠️ 以下 <...> 处必须替换为真实强密码，切勿使用 ChangeMe123! 等弱口令（会被 validate_env.sh 拒绝）
SECURE_JWT_SECRET=<运行: openssl rand -base64 48>
SECURE_DEFAULT_ADMIN_PASSWORD=<至少16位强密码，勿与下方相同>
SECURE_DATABASE_PASSWORD=<至少16位强密码>
SERVER_ENV=production
ALLOWED_ORIGINS=https://yourdomain.com
USE_SQLITE=false
MEMORY_STORE_ENABLED=false
DATABASE_HOST=localhost
```

### 完整配置（生产推荐）
```bash
# 安全配置
SECURE_JWT_SECRET=openssl生成的64字符密钥
SECURE_DEFAULT_ADMIN_PASSWORD=复杂密码
SECURE_DATABASE_PASSWORD=数据库密码

# 服务器配置
SERVER_ENV=production
ALLOWED_ORIGINS=https://yourdomain.com

# 数据库配置
USE_SQLITE=false
MEMORY_STORE_ENABLED=false
DATABASE_HOST=prod-db.example.com
DATABASE_PORT=5432
DATABASE_NAME=metaclouds
DATABASE_USER=metaclouds_user
DATABASE_SSL_MODE=require

# Redis 配置
REDIS_ENABLED=true
REDIS_HOST=prod-redis.example.com
REDIS_PORT=6379

# 日志配置
LOG_LEVEL=warn
LOG_OUTPUT=file
LOG_PATH=/var/log/metaclouds/backend.log

# 限流配置
RATE_LIMIT_ENABLED=true
RATE_LIMIT_REQUESTS=1000

# 熔断器配置
CIRCUIT_BREAKER_ENABLED=true
CIRCUIT_BREAKER_THRESHOLD=50

# 监控配置
PROMETHEUS_ENABLED=true
```

### 开发配置
```bash
SERVER_ENV=development
USE_SQLITE=true
REDIS_ENABLED=false
LOG_LEVEL=debug
LOG_OUTPUT=console
RATE_LIMIT_ENABLED=false
CIRCUIT_BREAKER_ENABLED=false
PROMETHEUS_ENABLED=false
JWT_SECRET=dev-secret-key-min-32-chars
DEFAULT_ADMIN_PASSWORD=DevPassword123!
```

---

## 📊 配置检查命令

```bash
# 验证配置语法
grep -E "^[A-Z_]+=.+$" .env.production | head -20

# 检查必填项
grep "CHANGE_THIS\|your-\|example" .env.production

# 查看所有配置
cat .env.production

# 运行验证脚本
./deploy/validate_env.sh -f .env.production
```

---

## 🔍 故障排查

### 配置未生效
```bash
# 确保加载了正确的配置文件
source .env.production
echo $SECURE_JWT_SECRET

# 或使用 env 文件启动
./metaclouds-backend --env-file .env.production
```

### 数据库连接失败
```bash
# 检查环境变量
echo $DATABASE_HOST
echo $DATABASE_PORT

# 测试连接
pg_isready -h $DATABASE_HOST -p $DATABASE_PORT
```

### 权限问题
```bash
# 设置文件权限
chmod 600 .env.production
chown root:root .env.production

# 设置目录权限
sudo mkdir -p /var/log/metaclouds /var/backups/metaclouds
sudo chown -R $USER:$USER /var/log/metaclouds /var/backups/metaclouds
```
