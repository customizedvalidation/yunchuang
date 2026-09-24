# syntax=docker/dockerfile:1
# ==============================================================================
# Metaclouds Backend (Rust) - multi-stage build
# Builder: rust:1.81-alpine (musl). Runtime: alpine:3.20.
# Non-root UID 10001. Target binary ~35MB.
# ==============================================================================

# ---- Stage 1: builder (musl static build) ----
FROM rust:1.81-alpine AS builder

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
    ( strip /app/target/release/metaclouds-backend-rust || \
      strip /app/target/release/metaclouds-backend )

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

COPY --from=builder /app/target/release/metaclouds-backend-rust /usr/local/bin/metaclouds-backend

COPY .env.example /app/.env.example

EXPOSE 8000

ENV SERVER_HOST=0.0.0.0 \
    SERVER_PORT=8000 \
    SERVER_ENV=production \
    RUST_LOG=info \
    LOG_LEVEL=info \
    LOG_FORMAT=json

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8000/metrics || exit 1

USER appuser

CMD ["metaclouds-backend"]
