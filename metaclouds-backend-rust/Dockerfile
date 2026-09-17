# ==============================================================================
# Metaclouds Backend (Rust) — Dockerfile (multi-stage build)
# ==============================================================================
# 目标：运行时镜像 ≤ 40 MB（alpine 基础 ~7MB + strip 后二进制 ~25-30MB）。
#
# 构建策略对齐 Go 版 docker/Dockerfile：
#   - Builder 阶段：先拷贝 Cargo.toml + Cargo.lock 构建依赖层（缓存优化）
#   - Runtime 阶段：alpine:3.20 + 非 root 用户（UID 10001）+ 只读必要目录
#
# 构建命令（本机无 Docker，待 CI/目标环境执行）：
#   docker build -t ghcr.io/customizedvalidation/yunchuang/backend-rust:latest .
#   docker images | grep backend-rust   # 验证大小 ≤ 40MB
#
# Rust TLS 说明：Cargo.toml 使用 runtime-tokio-rustls（纯 Rust TLS），
#   无需 OpenSSL，可静态链接到 musl，运行时不依赖 libssl。
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Builder —— rust:1.81-alpine（musl 工具链，产出静态二进制）
# ------------------------------------------------------------------------------
FROM rust:1.81-alpine AS builder

# musl-tools 提供 musl-gcc；pkg-config 供 crate 探测；openssl-dev 兜底
# （虽然 rustls 不链接 OpenSSL，但部分 crate 的 build.rs 仍会探测 pkg-config）。
RUN apk add --no-cache musl-dev pkg-config openssl-dev binutils

WORKDIR /app

# ---- 依赖层缓存优化 ----
# 先只拷贝 manifest 文件，创建占位源码，编译依赖（这一层会被 Docker 缓存）。
COPY Cargo.toml Cargo.lock ./

# 占位源码：让 cargo 认为 lib/bin target 存在以便预编译全部依赖。
# 真实 src/ 复制后会因 mtime 变化触发重编译（见下方 touch）。
RUN mkdir -p src && \
    echo "pub fn main() {}" > src/main.rs && \
    echo "// placeholder" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# ---- 拷贝真实源码并编译 ----
COPY . .

# touch 占位文件使 cargo 认为源码已变更，触发针对真实代码的重编译。
RUN touch src/main.rs src/lib.rs && \
    cargo build --release && \
    # strip 二进制：去除调试符号与符号表，显著减小体积
    strip /app/target/release/metaclouds-backend-rust || \
    strip /app/target/release/metaclouds-backend

# ------------------------------------------------------------------------------
# Stage 2: Runtime —— alpine:3.20（最小化运行时）
# ------------------------------------------------------------------------------
FROM alpine:3.20

# 运行时依赖：
#   ca-certificates — HTTPS 出站（OTLP/Jaeger/GitHub API）
#   tzdata          — 时区数据（日志时间戳对齐 Asia/Shanghai）
#   libgcc          — Rust 运行时（musl 静态链接通常不需要，但兜底）
#   wget            — docker-compose healthcheck（alpine 无 curl）
RUN apk add --no-cache ca-certificates tzdata wget && \
    # 设置时区
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone && \
    # 清理 apk 缓存减小体积
    rm -rf /var/cache/apk/*

WORKDIR /app

# 创建非 root 用户（UID 10001，对齐 Go 版 K8s 安全上下文）
RUN addgroup -g 10001 appgroup && \
    adduser -u 10001 -G appgroup -s /sbin/nologin -D appuser

# 从 builder 阶段拷贝 release 二进制
COPY --from=builder /app/target/release/metaclouds-backend-rust /usr/local/bin/metaclouds-backend

# 拷贝示例配置（容器内不作为运行配置，仅作参考）
COPY .env.example /app/.env.example

# 创建可写临时目录（K8s readOnlyRootFilesystem 需要 emptyDir 挂载到 /tmp）
RUN mkdir -p /tmp /run/metaclouds && \
    chown -R appuser:appgroup /tmp /run/metaclouds /app

# 容器监听端口 8000（对齐 Go 版和 K8s Service；Rust 默认 8001 但通过 SERVER_PORT=8000 覆盖）
EXPOSE 8000

# 生产环境默认环境变量（敏感配置通过 K8s Secret / docker-compose environment 注入）
ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8000 \
    SERVER_ENV=production \
    RUST_LOG=info \
    LOG_LEVEL=info \
    LOG_FORMAT=json

# 健康检查（docker-compose 用；K8s 探针在 Deployment 中定义）
# 注意：Rust 后端根路径无 /health 端点，使用 /metrics（无需 JWT，返回 200）
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8000/metrics || exit 1

# 切换到非 root 用户
USER appuser

# 启动命令
CMD ["metaclouds-backend"]
