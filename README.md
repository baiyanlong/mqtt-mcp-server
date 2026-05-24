# 🔌 MQTT MCP Server

> 云端 AI（Claude/GPT/Cursor）← 远程操控 → 树莓派边缘网关 ← MQTT → 你的工厂设备

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 🌐 [English](README.en.md) | **中文**

**把树莓派插到你的工厂机柜里，Claude 就能直接读取传感器、控制 PLC、分析设备健康——用自然语言。** 云端大模型不必部署在现场，小盒子只负责 MQTT ↔ AI 协议转换，轻量、安全、6.9MB 单文件。

作者：[byl](https://github.com/byl)

---

## 什么是 MCP？

**MCP（Model Context Protocol）是 Anthropic（Claude 母公司）2025 年推出的开放标准**——简单说，它就是"AI 的 USB 协议"。USB 让电脑能即插即用任何外设，MCP 让 AI 能即插即用任何外部工具。

```
没有 MCP 的时代：
  AI 只能聊天，想查个数据库？对不起，你得自己写代码调 API

有 MCP 的时代：
  AI ← MCP 协议 → 数据库、文件系统、MQTT设备、GitHub...
  任何一个实现了 MCP 的服务，AI 都能直接操作
```

**MQTT MCP Server 做了什么？** 它在 MCP 协议和 MQTT 协议之间架了一座桥。AI 看到的是"8 个工具函数"，实际上每个函数都在操控真实的物理设备。

```
AI Agent                     MQTT MCP Server            物理世界
"3号泵温度？"          →     mqtt_query_snapshot    →    MQTT 查询 → 87°C
"关掉 3 号泵"          →     mqtt_send_command      →    MQTT 指令 → 泵停机
"最近有什么异常？"     →     mqtt_get_alerts        →    返回告警列表
```

**MCP 的生态正在爆发**：Claude Desktop、Cursor、Windsurf、Continue.dev 都已经内置 MCP 支持。你写一个 MCP Server，几十个 AI 客户端都能用。这就像 90 年代写一个 HTTP 网站——协议是新的，机会是新的。

---

## 一句话理解：它怎么跑

```
你桌上的电脑                      工厂现场的树莓派
┌─────────────────┐              ┌──────────────────────┐
│ Claude Desktop   │   HTTP SSE  │  mqtt-mcp-server     │
│ 或 Cursor AI     │←────远程────→│  (6.9MB Rust 二进制) │
│                  │              │        ↓ MQTT        │
│ "关掉3号泵"      │              │  mosquitto broker    │
│       ↓          │              │        ↓             │
│  AI 调用工具     │              │  PLC / 传感器 / 执行器│
└─────────────────┘              └──────────────────────┘
```

**大模型不跑在树莓派上**——它在你电脑上或云端。树莓派只是个"翻译官"，把 AI 的指令翻译成 MQTT 消息发给设备，把设备数据翻译成 MCP 格式回给 AI。

---

## 快速开始

### 1. 下载（不需要装 Rust）

```bash
# ARM64 树莓派
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64

# x86_64 Linux 服务器
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-x86_64
```

或者源码安装：

```bash
cargo install mqtt-mcp-server
```

### 2. 启动

```bash
# 一行命令启动（不写配置文件也行）
mqtt-mcp-server \
  --mode sse \
  --listen 0.0.0.0:3000 \
  --broker tcp://localhost:1883 \
  --topics '#'
```

### 3. 连接 AI Agent

Claude Desktop 配置：

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

Cursor / Windsurf 同理。配完之后，你的 AI 就能操控物理设备了。

---

## 用自然语言操控设备

```
你: "3号泵现在温度多少？"
AI: [自动调用 mqtt_query_snapshot] → 87°C
    "3号泵当前 87°C，已超过 85°C 阈值。要我分析趋势吗？"

你: "分析一下"
AI: [调用 mqtt_query_range + mqtt_analyze]
    "过去5分钟以 2°C/分钟上升，疑似冷却系统故障。
     建议：降低负载，检查冷却液回路。"
```

AI 自动选择工具、自动构造参数、自动解读结果——你不需要知道 MQTT topic 是什么。

---

## 8 个 MCP 工具

| 工具 | 做什么 | 谁调用 |
|------|--------|--------|
| `mqtt_subscribe` | 订阅 MQTT 主题，开始监听设备 | AI |
| `mqtt_publish` | 向设备发消息（JSON 指令） | AI |
| `mqtt_list_devices` | 列出所有已注册的设备 | AI |
| `mqtt_query_snapshot` | 查某设备最新数据（"温度多少"） | AI |
| `mqtt_query_range` | 查历史趋势（"过去1小时温度曲线"） | AI |
| `mqtt_send_command` | 发控制指令（"关掉3号泵"） | AI |
| `mqtt_get_alerts` | 获取告警列表 | AI |
| `mqtt_analyze` | 调大模型深度分析设备健康 | AI |

**设备自动注册**：任何向 MQTT 发了消息的设备，不需要手动配置，自动出现在设备列表里。

---

## 规则引擎：不用等 AI，自动触发告警

AI 不是 24 小时盯着的，规则引擎替你站岗：

```yaml
rules:
  - name: "高温告警"
    device: "pump/*"
    metric: "temperature"
    condition: "value > 80"    # 超过 80°C 立刻告警
    action: "alert"
    ai_enhance: true           # 触发后自动让 LLM 分析

  - name: "设备离线"
    device: "*"
    metric: "status"
    condition: "last_seen > 300s"  # 5 分钟没数据
    action: "alert"
```

| 表达式 | 含义 | 场景 |
|--------|------|------|
| `value > 85` | 数值阈值 | 温度/压力/电流超标 |
| `rate > 5` | 变化速率 | 温度飙升、压力骤降 |
| `last_seen > 300s` | 离线检测 | 设备断连/断电 |

---

## Web Dashboard：实时看板

启动后打开 `http://树莓派IP:8080`：

- 设备在线/离线状态
- 告警列表（info / warning / critical 分级）
- 3 秒自动刷新
- 深色主题，单 HTML 页面，零依赖

---

## 树莓派一键部署

```bash
# 1. 下载 ARM64 二进制
wget https://github.com/baiyanlong/mqtt-mcp-server/releases/latest/download/mqtt-mcp-server-arm64

# 2. 装 Mosquitto（MQTT Broker）
sudo apt install -y mosquitto

# 3. 一键部署 systemd 服务
chmod +x install.sh && sudo ./install.sh

# 搞定。重启自动启动，崩溃自动重启。
```

详见 [deploy/README.md](deploy/README.md)。

---

## 支持的国内 AI 模型

客户自备 API Key，我们不替客户垫 token 费。

| 模型 | 命令行参数 | Base URL |
|------|-----------|----------|
| DeepSeek | `--ai-provider deepseek` | `api.deepseek.com` |
| 通义千问 | `--ai-provider qwen` | `dashscope.aliyuncs.com` |
| 智谱 GLM | `--ai-provider zhipu` | `open.bigmodel.cn` |
| Ollama 本地 | `--ai-provider ollama --ai-model qwen2.5` | `localhost:11434` |
| OpenAI 兼容 | `--ai-provider custom` | 自定义 |

```bash
# 示例：接 DeepSeek
mqtt-mcp-server --mode sse --listen 0.0.0.0:3000 \
  --ai --ai-provider deepseek --ai-model deepseek-chat \
  --ai-key sk-xxx
```

---

## 定价

| 版本 | 价格 | 功能 |
|------|------|------|
| **开源版** | 免费 (MIT) | 完整 MCP Server、规则引擎、Dashboard、AI Bridge |

Pro 版（多节点管理、云 Dashboard、OTA 升级）规划中。

---

## 环境要求

- ARM64（树莓派 3B+/4/5）或 x86_64 Linux
- 一个 MQTT Broker（mosquitto 够用）
- （可选）LLM API Key，启用 AI 分析

---

## 许可证

MIT

---

## 联系

<p align="center">
  <img src="qq-group.jpg" width="200" alt="QQ群"><br>
  <b>扫码加入 QQ 群</b>
</p>
