FROM golang:1.21-alpine AS builder

WORKDIR /app

RUN apk add --no-cache git ca-certificates

COPY go.mod go.sum* ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o metaclouds-backend .

FROM alpine:3.19

WORKDIR /app

RUN apk --no-cache add ca-certificates tzdata

COPY --from=builder /app/metaclouds-backend .
COPY --from=builder /app/.env.production .env

RUN addgroup -g 1000 appgroup && \
    adduser -u 1000 -G appgroup -s /bin/sh -D appuser

USER appuser

EXPOSE 8000

ENV ENVIRONMENT=production
ENV SERVER_PORT=8000
ENV SERVER_HOST=0.0.0.0
ENV MEMORY_STORE_ENABLED=false
ENV DATABASE_TYPE=redis
ENV REDIS_HOST=redis
ENV REDIS_PORT=6379
ENV PROMETHEUS_ENABLED=true
ENV LOG_LEVEL=info
ENV LOG_FORMAT=json

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8000/health || exit 1

ENTRYPOINT ["./metaclouds-backend"]
