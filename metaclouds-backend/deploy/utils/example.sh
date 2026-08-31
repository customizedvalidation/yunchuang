#!/bin/bash

# ==============================================
# 示例脚本 - 如何使用端口工具函数库
# ==============================================

echo "=== 端口工具函数库使用示例 ==="
echo ""

# 1. 引入工具库
echo "[步骤 1] 引入工具库..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PORT_UTILS_SCRIPT="$SCRIPT_DIR/port_utils.sh"

if [[ -f "$PORT_UTILS_SCRIPT" ]]; then
    source "$PORT_UTILS_SCRIPT"
    echo "✓ 工具库引入成功"
else
    echo "✗ 工具库未找到: $PORT_UTILS_SCRIPT"
    exit 1
fi
echo ""

# 2. 测试端口检测
echo "[步骤 2] 检测常用端口..."
ports_to_check=(22 80 443 8000 8080 3306)
for port in "${ports_to_check[@]}"; do
    if port_utils_is_port_in_use "$port"; then
        echo "✗ 端口 $port 已被占用"
    else
        echo "✓ 端口 $port 可用"
    fi
done
echo ""

# 3. 获取示例端口信息
echo "[步骤 3] 尝试获取端口 8000 的进程信息..."
if port_utils_is_port_in_use 8000; then
    echo "发现端口 8000 已被占用，获取进程信息..."
    port_utils_get_process_info 8000
fi
echo ""

# 4. 查找可用端口
echo "[步骤 4] 查找可用端口..."
free_port=$(port_utils_find_available_port)
echo "找到可用端口: $free_port"
echo ""

# 5. 从示例 .env 文件读取端口
echo "[步骤 5] 读取端口配置..."
temp_env=$(mktemp)
echo "SERVER_PORT=8000" > "$temp_env"
env_port=$(port_utils_get_port_from_env "$temp_env")
echo "从 $temp_env 读取的端口: $env_port"
rm "$temp_env"
echo ""

echo "=== 示例结束 ==="
echo "使用提示："
echo "  - 在你的脚本中 source port_utils.sh"
echo "  - 然后调用如 port_utils_ensure_port_available 等函数"
