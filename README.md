# MQTT MCP Server

[![Crates.io](https://img.shields.io/crates/v/mqtt-mcp-server)](https://crates.io/crates/mqtt-mcp-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**Let any AI Agent talk to physical devices via MQTT.**

MQTT MCP Server is a [Model Context Protocol](https://modelcontextprotocol.io/) server that bridges AI agents (Claude, GPT, etc.) to any MQTT-connected IoT device. Deploy it in 5 minutes, and your AI assistant can read sensor data, control actuators, and analyze telemetry — all through natural language.

## Quick Start

### Install

```bash
cargo install mqtt-mcp-server
```

### Configure

```bash
cp config.example.yaml config.yaml
# Edit config.yaml: set your MQTT broker and (optionally) AI provider
```

### Run

```bash
# Stdio mode (for Claude Desktop, etc.)
mqtt-mcp-server --config config.yaml --mode stdio

# SSE mode (HTTP server)
mqtt-mcp-server --config config.yaml --mode sse --listen 127.0.0.1:3000
```

### Configure Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "mqtt": {
      "command": "mqtt-mcp-server",
      "args": ["--config", "/path/to/config.yaml", "--mode", "stdio"]
    }
  }
}
```

Now Claude can interact with your IoT devices.

## AI Agent Capabilities

Once connected, your AI agent can:

- **Subscribe** to MQTT topics to monitor device data in real-time
- **Publish** commands to control devices (e.g., "turn off pump #3")
- **Query** current sensor values and historical trends
- **Analyze** device health using AI (anomaly detection, predictive maintenance)
- **Manage alerts** — get notified when something goes wrong

### Example Conversation

```
User: "What's the temperature of pump #3?"
AI:   [calls mqtt_query_snapshot] → 87°C
AI:   "Pump #3 is at 87°C, which is above the 85°C threshold. Shall I analyze further?"

User: "Yes, analyze the trend."
AI:   [calls mqtt_query_range + mqtt_analyze]
AI:   "The temperature has been rising 2°C/min for the last 5 minutes.
       This suggests a cooling system issue. Recommendation: reduce load
       and inspect the coolant circuit."
```

## Tools (MCP)

| Tool | Description |
|------|-------------|
| `mqtt_subscribe` | Subscribe to MQTT topics |
| `mqtt_publish` | Publish messages to MQTT topics |
| `mqtt_list_devices` | List all connected devices |
| `mqtt_query_snapshot` | Get latest value for a device/metric |
| `mqtt_query_range` | Query historical telemetry |
| `mqtt_send_command` | Send commands to devices |
| `mqtt_get_alerts` | Get recent alerts |
| `mqtt_analyze` | AI-powered device health analysis |

## Pricing

| Tier | Price | Features |
|------|-------|----------|
| **Open Source** | Free (MIT) | Full MCP Server, single broker, local rules, basic AI Bridge |
| **Pro** | $49/node/month | Multi-node dashboard, multi-broker, industry templates, alert push |
| **Enterprise** | Custom | Private deployment, custom protocols (Modbus/OPC-UA), SSO, SLA |

## Architecture

```
AI Agent (Claude/GPT) ←→ MCP Protocol ←→ MQTT MCP Server ←→ MQTT Broker ←→ IoT Devices
```

- Written in Rust — single binary, <10MB, memory-safe
- Supports stdio and SSE transports
- Built-in rule engine with configurable DSL
- AI Bridge: local pre-filtering + LLM analysis for anomaly detection

## Requirements

- Rust 1.75+ (if building from source)
- An MQTT broker (e.g., mosquitto, EMQX, HiveMQ)
- (Optional) LLM API key for AI analysis features

## License

MIT — see [LICENSE](LICENSE) for details.

---

Built with ❤️ for the intersection of AI and physical computing.
