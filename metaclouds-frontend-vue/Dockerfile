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

# nginx 默认监听 80
EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
