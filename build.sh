#!/bin/bash
# MQTT MCP Server — Docker 构建脚本
# 用法: ./build.sh
set -e

echo "=== 1. 编译 release ==="
cargo build --release

echo "=== 2. 构建 Docker 镜像 ==="
docker build -t mqtt-mcp-server:latest .

echo "=== 3. 验证 ==="
docker images mqtt-mcp-server

echo ""
echo "✅ 构建完成！"
echo ""
echo "启动方式："
echo "  docker run -p 8080:8080 --network host mqtt-mcp-server --broker tcp://your-broker:1883 --web 8080"
echo ""
echo "或 docker-compose:"
echo "  docker-compose up -d"
