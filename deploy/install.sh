# MQTT MCP Server — ARM64 部署包
#
# 使用：
#   1. 把 mqtt-mcp-server-arm64 二进制和 config.yaml 放到设备上
#   2. chmod +x install.sh && sudo ./install.sh
#
# 设备要求：
#   - ARM64（树莓派 3B+/4/5、CM4 等）
#   - 已安装 Mosquitto MQTT Broker（sudo apt install mosquitto）
#   - systemd（Raspberry Pi OS 默认）

# ── 安装 ──
# sudo ./install.sh
# 或指定参数：sudo ./install.sh --broker tcp://192.168.1.100:1883 --listen 0.0.0.0:3000

set -e

BINARY="mqtt-mcp-server-arm64"
CONFIG="config.yaml"
SERVICE_NAME="mqtt-mcp-server"
INSTALL_DIR="/opt/mqtt-mcp-server"

# 默认参数（可通过命令行覆盖）
BROKER="${BROKER:-tcp://localhost:1883}"
LISTEN="${LISTEN:-0.0.0.0:3000}"
WEB_PORT="${WEB_PORT:-8080}"
TOPICS="${TOPICS:-#}"
DB_PATH="${DB_PATH:-/var/lib/mqtt-mcp-server/data.db}"

# 解析命令行参数
while [ $# -gt 0 ]; do
    case "$1" in
        --broker) BROKER="$2"; shift 2 ;;
        --listen) LISTEN="$2"; shift 2 ;;
        --web-port) WEB_PORT="$2"; shift 2 ;;
        --topics) TOPICS="$2"; shift 2 ;;
        --dir) INSTALL_DIR="$2"; shift 2 ;;
        *) echo "未知参数: $1"; exit 1 ;;
    esac
done

echo "══════════════════════════════════════"
echo "MQTT MCP Server — ARM64 安装"
echo "══════════════════════════════════════"
echo ""
echo "安装目录:   $INSTALL_DIR"
echo "MQTT Broker: $BROKER"
echo "SSE 监听:    $LISTEN"
echo "Web 面板:    http://$(hostname -I | awk '{print $1}'):$WEB_PORT"
echo ""

# 创建目录
mkdir -p "$INSTALL_DIR"
mkdir -p "$(dirname "$DB_PATH")"

# 拷贝二进制
if [ -f "./$BINARY" ]; then
    cp "./$BINARY" "$INSTALL_DIR/mqtt-mcp-server"
    chmod +x "$INSTALL_DIR/mqtt-mcp-server"
    echo "✓ 二进制已安装"
else
    echo "✗ 找不到 $BINARY，请放在当前目录"
    exit 1
fi

# 生成配置文件
if [ -f "./$CONFIG" ]; then
    cp "./$CONFIG" "$INSTALL_DIR/config.yaml"
else
    # 生成默认配置
    cat > "$INSTALL_DIR/config.yaml" << YAML
# MQTT MCP Server 配置
mqtt:
  broker: "$BROKER"
  topics: ["$TOPICS"]

ai:
  enabled: false
  provider: "openai"
  model: "gpt-4o"
  window_size: 100

storage:
  db_path: "$DB_PATH"

rules: []
devices: []
YAML
fi
echo "✓ 配置文件已生成"

# 创建 systemd 服务
cat > "/etc/systemd/system/$SERVICE_NAME.service" << SERVICE
[Unit]
Description=MQTT MCP Server — AI Agent 操控 IoT 设备
Documentation=https://github.com/baiyanlong/mqtt-mcp-server
After=network.target mosquitto.service
Wants=mosquitto.service

[Service]
Type=simple
User=root
ExecStart=$INSTALL_DIR/mqtt-mcp-server \\
    --config $INSTALL_DIR/config.yaml \\
    --mode sse \\
    --listen $LISTEN \\
    --web $WEB_PORT
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# 安全加固
NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
ReadWritePaths=$INSTALL_DIR $(dirname "$DB_PATH")
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
SERVICE

echo "✓ systemd 服务已创建"

# 重载并启动
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

sleep 2
if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo ""
    echo "══════════════════════════════════════"
    echo "✓ 安装成功！"
    echo "══════════════════════════════════════"
    echo ""
    echo "服务状态:  systemctl status $SERVICE_NAME"
    echo "查看日志:  journalctl -u $SERVICE_NAME -f"
    echo "SSE 端点:  http://$(hostname -I | awk '{print $1}'):$(echo $LISTEN | cut -d: -f2)/sse"
    echo "Web 面板:  http://$(hostname -I | awk '{print $1}'):$WEB_PORT"
    echo ""
    echo "AI Agent 连接（Claude Desktop）:"
    echo "  {"
    echo "    \"mcpServers\": {"
    echo "      \"mqtt\": {"
    echo "        \"transport\": \"sse\","
    echo "        \"url\": \"http://$(hostname -I | awk '{print $1}'):$(echo $LISTEN | cut -d: -f2)/sse\""
    echo "      }"
    echo "    }"
    echo "  }"
    echo ""
    echo "MCP Inspector:"
    echo "  npx @anthropic-ai/mcp-inspector sse http://$(hostname -I | awk '{print $1}'):$(echo $LISTEN | cut -d: -f2)/sse"
else
    echo "✗ 启动失败，查看日志: journalctl -u $SERVICE_NAME -n 30"
    exit 1
fi
