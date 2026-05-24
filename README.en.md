# 🔌 MQTT MCP Server

> Cloud AI (Claude/GPT/Cursor) ← Remote Control → Raspberry Pi Edge Gateway ← MQTT → Your Factory Devices

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 🌐 **English** | [中文](README.md)

**Plug a Raspberry Pi into your factory cabinet. Claude can now read sensors, control PLCs, and analyze equipment health — using natural language.** The cloud AI doesn't run on the Pi; the little box just translates between MQTT and the AI protocol. Lightweight, secure, 6.9MB single binary.

Author: [byl](https://github.com/byl)

---

## What is MCP?

**MCP (Model Context Protocol) is an open standard launched by Anthropic in 2025.** Think of it as "USB for AI." USB lets computers plug-and-play any peripheral; MCP lets AI plug-and-play any external tool.

```
Before MCP:
  AI could only chat. Want to query a database? Write custom API code yourself.

After MCP:
  AI ← MCP Protocol → Databases, Filesystems, MQTT devices, GitHub...
  Any MCP-compatible service, AI can operate directly.
```

**What does MQTT MCP Server do?** It builds a bridge between MCP and MQTT. The AI sees "8 tool functions," but each one actually controls real physical devices.

```
AI Agent                     MQTT MCP Server            Physical World
"What's pump #3 temp?"  →   mqtt_query_snapshot    →   MQTT query → 87°C
"Shut down pump #3"     →   mqtt_send_command      →   MQTT command → pump stops
"Any anomalies?"        →   mqtt_get_alerts        →   Returns alert list
```

**The MCP ecosystem is exploding**: Claude Desktop, Cursor, Windsurf, and Continue.dev all have native MCP support. Build one MCP Server, dozens of AI clients can use it. Like building an HTTP website in the 90s — new protocol, new opportunity.

---

## Architecture at a Glance

```
Your Laptop                      Factory Floor Raspberry Pi
┌─────────────────┐              ┌──────────────────────┐
│ Claude Desktop   │   HTTP SSE  │  mqtt-mcp-server     │
│ or Cursor AI     │←──remote───→│  (6.9MB Rust binary) │
│                  │              │        ↓ MQTT        │
│ "Shut down #3"   │              │  mosquitto broker    │
│       ↓          │              │        ↓             │
│  AI calls tools  │              │  PLC / Sensors / Actuators │
└─────────────────┘              └──────────────────────┘
```

**The LLM doesn't run on the Pi** — it's on your machine or in the cloud. The Pi is just a "translator": AI commands → MQTT messages → devices, and device data → MCP format → AI.

---

## Quick Start

### 1. Download (no Rust required)

```bash
# ARM64 Raspberry Pi
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64

# x86_64 Linux server
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-x86_64
```

Or build from source:

```bash
cargo install mqtt-mcp-server
```

### 2. Run

```bash
mqtt-mcp-server \
  --mode sse \
  --listen 0.0.0.0:3000 \
  --broker tcp://localhost:1883 \
  --topics '#'
```

### 3. Connect AI Agent

Claude Desktop config:

```json
{
  "mcpServers": {
    "mqtt": {
      "transport": "sse",
      "url": "http://<pi-ip>:3000/sse"
    }
  }
}
```

Same for Cursor / Windsurf. Done — your AI can now control physical devices.

---

## Control Devices with Natural Language

```
You: "What's pump #3's temperature?"
AI:  [calls mqtt_query_snapshot] → 87°C
     "Pump #3 is at 87°C, above 85°C threshold. Analyze further?"

You: "Yes"
AI:  [calls mqtt_query_range + mqtt_analyze]
     "Rising 2°C/min — possible cooling failure.
      Recommend: reduce load, inspect coolant loop."
```

The AI auto-selects tools, constructs parameters, and interprets results. You don't need to know MQTT topics.

---

## 8 MCP Tools

| Tool | What it does |
|------|-------------|
| `mqtt_subscribe` | Subscribe to MQTT topics |
| `mqtt_publish` | Send messages to devices |
| `mqtt_list_devices` | List all connected devices |
| `mqtt_query_snapshot` | Get latest device reading |
| `mqtt_query_range` | Query historical trends |
| `mqtt_send_command` | Send control commands |
| `mqtt_get_alerts` | Get alert list |
| `mqtt_analyze` | AI-powered health analysis |

**Auto-discovery**: Any device that publishes to MQTT is automatically registered — no config needed.

---

## Rule Engine: Alerts Without Waiting for AI

```yaml
rules:
  - name: "High Temperature"
    device: "pump/*"
    metric: "temperature"
    condition: "value > 80"
    action: "alert"
    ai_enhance: true

  - name: "Device Offline"
    device: "*"
    metric: "status"
    condition: "last_seen > 300s"
    action: "alert"
```

| Expression | Meaning |
|------------|---------|
| `value > 85` | Numeric threshold |
| `rate > 5` | Rate of change |
| `last_seen > 300s` | Offline detection |

---

## Web Dashboard

Open `http://<pi-ip>:8080`:

- Device online/offline status
- Alert list (info / warning / critical)
- Auto-refresh every 3 seconds
- Dark theme, single HTML page, zero dependencies

---

## Raspberry Pi One-Click Deploy

```bash
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64
sudo apt install -y mosquitto
chmod +x install.sh && sudo ./install.sh
```

See [deploy/README.md](deploy/README.md).

---

## Supported LLM Providers

(Customer provides API key)

| Provider | CLI flag |
|----------|----------|
| DeepSeek | `--ai-provider deepseek` |
| Qwen | `--ai-provider qwen` |
| GLM | `--ai-provider zhipu` |
| Ollama (local) | `--ai-provider ollama --ai-model qwen2.5` |
| Custom | `--ai-provider custom` |

---

## Pricing

| Tier | Price | Features |
|------|-------|----------|
| **Open Source** | Free (MIT) | Full MCP Server, Rule Engine, Dashboard, AI Bridge |

Pro tier (multi-node, cloud dashboard, OTA) in planning.

---

## Requirements

- ARM64 (Pi 3B+/4/5) or x86_64 Linux
- An MQTT broker (mosquitto works great)
- (Optional) LLM API key for AI analysis

---

## License

MIT

---

## Contact

<p align="center">
  <img src="qq-group.jpg" width="200" alt="QQ Group"><br>
  <b>Scan to join QQ Group</b>
</p>
