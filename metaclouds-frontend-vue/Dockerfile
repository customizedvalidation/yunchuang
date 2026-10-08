# ==============================================================================
# Metaclouds Frontend (Vue 3) Dockerfile
# 多阶段构建：node:22-alpine 构建产物 -> nginx:alpine 静态托管
# ==============================================================================

# --- Builder 阶段 ---
# 与项目工具链对齐：node 22（本地 managed 22.x），避免与 Vite 5 / vue-tsc 兼容问题
FROM node:22-alpine AS builder

WORKDIR /app

# npm ci 依据 package-lock.json 做可复现安装（CI/生产推荐）
COPY package*.json ./
RUN npm ci

COPY . .

# 关闭 CodeBuddy 安全删除保护，避免构建脚本误判
ENV CODEBUDDY_SAFE_DELETE_ENABLED=0

# build = vue-tsc --noEmit && vite build，产物输出到 dist/
RUN npm run build

# --- Runtime 阶段 ---
FROM nginx:alpine

# Vite 默认输出目录 dist/
COPY --from=builder /app/dist /usr/share/nginx/html

# 自定义 nginx 配置（SPA 回退 + 静态资源缓存策略）
COPY nginx.conf /etc/nginx/conf.d/default.conf

# 非 root 运行（对齐 K8s runAsUser: 101）：
#  - 站点文件、nginx 运行时目录（缓存/pid/log）归 uid 101
#  - 容器默认不再以 root 启动，满足非特权容器基线
RUN chown -R 101:101 /usr/share/nginx/html \
    && chown -R 101:101 /var/cache/nginx /var/run /tmp /var/log/nginx
USER 101

# nginx 非特权端口 8080（USER 101 无法绑定特权端口 80，与 K8s containerPort 8080 对齐）
EXPOSE 8080

# 容器级健康检查（nginx 自带 busybox wget，探测 8080 上 /healthz 静态页）
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1

CMD ["nginx", "-g", "daemon off;"]
