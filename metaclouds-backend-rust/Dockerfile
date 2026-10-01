# syntax=docker/dockerfile:1
# ==============================================================================
# Metaclouds Backend (Rust) - multi-stage build
# Builder: rust:1.96-alpine (musl). Runtime: alpine:3.20.
# Non-root UID 10001. Target binary ~35MB.
# NOTE: toolchain must match CI (dtolnay/rust-toolchain@1.96.0) — deps like
#       opentelemetry 0.27 / utoipa 5 / redis 0.27 require MSRV > 1.81.
# ==============================================================================

# ---- Stage 1: builder (musl static build) ----
FROM rust:1.96-alpine AS builder

# rustls is used (no native OpenSSL needed); only musl C headers + binutils for strip.
RUN apk add --no-cache musl-dev binutils && \
    rm -rf /var/cache/apk/*

WORKDIR /app

# Dependency caching layer: copy manifests first
COPY Cargo.toml Cargo.lock ./

# Build a placeholder to cache compiled dependencies
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    mkdir -p src && \
    echo "fn main() {}" > src/main.rs && \
    echo "// placeholder" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy real source and rebuild only this crate
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    touch src/main.rs src/lib.rs && \
    cargo build --release && \
    mkdir -p /out && \
    cp /app/target/release/metaclouds-backend-rust /out/metaclouds-backend-rust && \
    strip /out/metaclouds-backend-rust

# ---- Stage 2: runtime ----
FROM alpine:3.20

RUN apk add --no-cache ca-certificates tzdata wget && \
    cp /usr/share/zoneinfo/Asia/Shanghai /etc/localtime && \
    echo "Asia/Shanghai" > /etc/timezone && \
    addgroup -g 10001 appgroup && \
    adduser -u 10001 -G appgroup -s /sbin/nologin -D appuser && \
    mkdir -p /app /tmp /run/metaclouds && \
    chown -R appuser:appgroup /app /tmp /run/metaclouds && \
    rm -rf /var/cache/apk/*

WORKDIR /app

# 产物在 /out（RUN 内从 cache mount 复制到持久层，cache mount 不落层）
COPY --from=builder /out/metaclouds-backend-rust /usr/local/bin/metaclouds-backend

COPY .env.example /app/.env.example

EXPOSE 8000

ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8000 \
    SERVER_ENV=production \
    RUST_LOG=info \
    LOG_LEVEL=info \
    LOG_FORMAT=json

# /health 是后端始终提供的存活端点（src/routes.rs 根级注册，无鉴权）；
# 此前打 /metrics，在 PROMETHEUS_ENABLED=false 时恒 404，容器被误判 unhealthy。
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8000/health || exit 1

USER appuser

CMD ["metaclouds-backend"]
