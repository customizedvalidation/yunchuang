# syntax=docker/dockerfile:1
# ==============================================================================
# Metaclouds Backend (Rust) — Dockerfile (multi-stage build, C8 优化版)
# ==============================================================================
# 目标：运行时镜像 ≤ 35 MB（alpine:3.20 ~7MB + ca-certificates ~1MB + tzdata ~3MB
#       + strip 后二进制 ~20-25MB）。
#
# 相对 P3-06 初版的 C8 优化点：
#   1. 基础镜像评估：Cargo.toml 使用 runtime-tokio-rustls（纯 Rust TLS，不链接
#      OpenSSL），redis/tonic 均为纯 Rust 实现，无 glibc 依赖 → 继续用
#      rust:*-alpine（musl）builder + alpine:3.20 runtime，比 debian-slim(~80MB)
#      小一个数量级。
#   2. BuildKit 缓存挂载：RUN --mount=type=cache 复用 cargo registry 与 target/
#      目录，重复构建跳过依赖编译（需 Docker BuildKit，见下方语法指令头）。
#   3. 层合并：runtime 阶段 apk 安装 / 时区 / 建用户 / 建目录合并为更少 RUN 层。
#   4. strip 二进制：builder 阶段编译后 strip，去除调试符号（alpine binutils）。
#   5. 非 root 用户：UID 10001（对齐 Go 版 K8s securityContext）。
#   6. 只读根文件系统：配合 K8s readOnlyRootFilesystem: true，/tmp 与
#      /run/metaclouds 通过 emptyDir 挂载。
#
# 【重要】本机无 Docker，未实际构建。以上大小为静态估算，
#         待 CI/目标环境 `docker build` 实测确认（验收标准 #6）。
#
# 构建命令（需 BuildKit；Docker 18.09+ 默认开启）：
#   DOCKER_BUILDKIT=1 docker build -t ghcr.io/customizedvalidation/yunchuang/backend-rust:latest .
#   docker images | grep backend-rust   # 验证大小 ≤ 35MB
#
# Rust TLS 说明：sqlx 启用 runtime-tokio-rustls（纯 Rust TLS），opentelemetry-otlp
#   grpc-tonic 走 rustls，运行时不依赖 libssl / glibc，可静态链接 musl。
#   openssl-dev 仅在 builder 安装，供部分 crate 的 build.rs 探测 pkg-config，
#   不进入 runtime 镜像。
# ==============================================================================

# ------------------------------------------------------------------------------
# Stage 1: Builder —— rust:1.81-alpine（musl 工具链，产出静态二进制）
# ------------------------------------------------------------------------------
FROM rust:1.81-alpine AS builder

# musl-dev 提供 musl-gcc；pkg-config 供 crate 探测；binutils 提供 strip；
# openssl-dev 兜底（rustls 不链接 OpenSSL，但 build.rs 可能探测）。
# 一次性安装并清理 apk 缓存，减小 builder 层（builder 不进最终镜像）。
RUN apk add --no-cache musl-dev pkg-config openssl-dev binutils && \
    rm -rf /var/cache/apk/*

WORKDIR /app

# ---- 依赖层缓存优化 ----
# 先只拷贝 manifest 文件，创建占位源码，编译依赖（这一层会被 Docker 缓存）。
COPY Cargo.toml Cargo.lock ./

# 占位源码：让 cargo 认为 lib/bin target 存在以便预编译全部依赖。
# 使用 BuildKit cache mount 复用 cargo registry 与 target 目录：
#   target=/usr/local/cargo/registry —— crates.io 下载缓存
#   target=/app/target               —— 编译产物缓存
# 注意：cache mount 仅在 BuildKit 下生效；非 BuildKit 构建会忽略 --mount 行
#       （需 Dockerfile 首行 `# syntax=docker/dockerfile:1` 启用前沿前端）。
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    mkdir -p src && \
    echo "pub fn main() {}" > src/main.rs && \
    echo "// placeholder" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# ---- 拷贝真实源码并编译 ----
COPY . .

# touch 占位文件使 cargo 认为源码已变更，触发针对真实代码的重编译。
# 再次使用 cache mount 复用依赖编译产物，仅重编本 crate。
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    touch src/main.rs src/lib.rs && \
    cargo build --release && \
    # strip 二进制：去除调试符号与符号表，显著减小体积
    # （metaclouds-backend-rust 为当前 bin 名；兜底尝试 metaclouds-backend）
    (strip /app/target/release/metaclouds-backend-rust || \
     strip /app/target/release/metaclouds-backend)

# ------------------------------------------------------------------------------
# Stage 2: Runtime —— alpine:3.20（最小化运行时）
# ------------------------------------------------------------------------------
FROM alpine:3.20

# 合并多个 RUN 为单层（层合并优化）：
#   - apk 安装运行时依赖：ca-certificates(HTTPS 出站) / tzdata(时区) / wget(健康检查)
#   - 设置时区 Asia/Shanghai
#   - 创建非 root 用户 appuser(UID 10001) 与 appgroup(GID 10001)
#   - 创建可写目录 /tmp、/run/metaclouds 并 chown
#   - 清理 apk 缓存减小层体积
# libgcc 注释：musl 静态链接通常不需要，保留兜底说明不强制安装。
#
# 只读根文件系统：K8s Deployment securityContext 设
#   readOnlyRootFilesystem: true，/tmp 与 /run/metaclouds 须以 emptyDir 挂载，
#   否则非 root 进程无法写入。
RUN apk add --no-cache ca-certificates tzdata wget && \
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone && \
    addgroup -g 10001 appgroup && \
    adduser -u 10001 -G appgroup -s /sbin/nologin -D appuser && \
    mkdir -p /tmp /run/metaclouds && \
    chown -R appuser:appgroup /tmp /run/metaclouds /app && \
    rm -rf /var/cache/apk/*

WORKDIR /app

# 从 builder 阶段拷贝 strip 后的 release 二进制
COPY --from=builder /app/target/release/metaclouds-backend-rust /usr/local/bin/metaclouds-backend

# 拷贝示例配置（容器内不作为运行配置，仅作参考）
COPY .env.example /app/.env.example

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
