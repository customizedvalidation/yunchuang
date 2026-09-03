
# 与项目工具链对齐：node 22（本地 managed 22.22.2），避免 node 18 与 Vite 5/依赖的兼容风险
FROM node:22-alpine AS builder

WORKDIR /app

# npm ci 依据 package-lock.json 做可复现安装（比 npm install 更稳、更快，且 CI/生产推荐）
COPY package*.json ./
RUN npm ci

COPY . .

# 构建脚本为 "tsc && vite build"，产物输出到 dist/（不是 build/）
RUN npm run build

FROM nginx:alpine

# Vite 默认输出目录是 dist/；原 /app/build 是错误路径，会导致 COPY 失败或空目录
COPY --from=builder /app/dist /usr/share/nginx/html

COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 3000

CMD ["nginx", "-g", "daemon off;"]
