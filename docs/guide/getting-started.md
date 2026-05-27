# 快速开始

5 分钟，让 AI 操控你的 IoT 设备。

## 1. 下载

```bash
# ARM64 树莓派
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64
mv mqtt-mcp-server-arm64 mqtt-mcp-server && chmod +x mqtt-mcp-server

# x86_64 Linux 服务器
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-x86_64
mv mqtt-mcp-server-x86_64 mqtt-mcp-server && chmod +x mqtt-mcp-server

# 或从源码安装
cargo install mqtt-mcp-server
```

## 2. 确保有 MQTT Broker

```bash
# 装 Mosquitto（如果没有）
sudo apt install -y mosquitto
```

## 3. 启动

```bash
# SSE 模式（推荐，支持远程 AI 连接）
./mqtt-mcp-server \
  --mode sse \
  --listen 0.0.0.0:3000 \
  --broker tcp://localhost:1883 \
  --topics '#'
```

## 4. 连接 AI Agent

### Claude Desktop

在 `claude_desktop_config.json` 中添加：

```json
{
  "mcpServers": {
    "mqtt": {
      "transport": "sse",
      "url": "http://你的树莓派IP:3000/sse"
    }
  }
}
```

### Cursor / Windsurf

同样方式，配 MCP Server 的 SSE 端点。

### MCP Inspector（调试用）

```bash
npx @anthropic-ai/mcp-inspector sse http://localhost:3000/sse
```

## 5. 试试效果

配好之后，在 Claude 里直接问：

> "有哪些设备在线？"  
> "3号泵现在温度多少？"  
> "发布指令到 devices/pump/3/command，内容 {\\"action\\":\\"reboot\\"}"

AI 会自动选择对应的 MCP 工具。

## 下一步

- [配置 AI 模型](/guide/ai-config) — 接 DeepSeek / 通义千问 / Ollama
- [规则引擎](/guide/rule-engine) — 自动检测异常
- [树莓派部署](/deploy/raspberry-pi) — 一键 systemd 服务
- [API 参考](/api/tools) — 8 个 MCP 工具详细说明
