# ==============================================================================
# Metaclouds Backend Dockerfile
# ==============================================================================
# 构建阶段
FROM golang:1.24-alpine AS builder

WORKDIR /app

# 安装依赖
RUN apk add --no-cache git ca-certificates tzdata

# 复制 go.mod 和 go.sum
COPY go.mod go.sum ./

# 下载依赖
RUN go mod download

# 复制源代码
COPY . .

# 构建
ARG LOG_LEVEL=debug
ENV LOG_LEVEL=${LOG_LEVEL}

# CGO_ENABLED=0 产出静态二进制（生产强制 PostgreSQL，sqlite 仅开发用；实测 GOOS=linux
# CGO_ENABLED=0 全量重编译通过，无 CGO 依赖问题）。
# -trimpath/-s -w 去除路径与符号信息，产出更小、更安全的生产二进制。
RUN CGO_ENABLED=0 GOOS=linux go build -trimpath -ldflags="-s -w" -o metaclouds-backend .

# ==============================================================================
# 运行阶段
FROM alpine:latest

WORKDIR /app

# 安装运行时依赖
RUN apk add --no-cache ca-certificates tzdata curl

# 创建必要目录
RUN mkdir -p /var/log/metaclouds /var/backups/metaclouds /run/metaclouds

# 复制二进制文件
COPY --from=builder /app/metaclouds-backend .

# 复制配置文件（可选）
COPY --from=builder /app/.env.production.example .env.example

# 设置时区
ENV TZ=Asia/Shanghai

# 暴露端口
EXPOSE 8000

# 健康检查
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1

# 运行命令
CMD ["./metaclouds-backend"]
