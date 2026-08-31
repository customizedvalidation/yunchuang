#!/bin/bash

# Dashboard 组件测试运行脚本

echo "====================================="
echo "Dashboard 组件单元测试"
echo "====================================="
echo ""

# 检查依赖
echo "1. 检查测试依赖..."
if [ ! -d "node_modules/jest-environment-jsdom" ]; then
    echo "   ⚠️  jest-environment-jsdom 未安装"
    echo "   请运行: npm install --save-dev jest-environment-jsdom"
    exit 1
fi

echo "   ✅ 所有依赖已安装"
echo ""

# 运行测试
echo "2. 运行测试..."
npm test -- --verbose --coverage

echo ""
echo "====================================="
echo "测试完成"
echo "====================================="
