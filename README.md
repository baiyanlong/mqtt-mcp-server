# MQTT MCP Server

[![Crates.io](https://img.shields.io/crates/v/mqtt-mcp-server)](https://crates.io/crates/mqtt-mcp-server)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> 🌐 [English](README.en.md) | **中文**

**让任何 AI Agent 通过 MQTT 操控物理设备。**

MQTT MCP Server 是一个 [Model Context Protocol](https://modelcontextprotocol.io/) 服务端，将 AI 智能体（Claude、GPT 等）与任何 MQTT 连接的 IoT 设备打通。5 分钟部署，你的 AI 助手就能读取传感器数据、控制执行器、分析遥测——全都通过自然语言。

作者：[byl](https://github.com/byl)

---

## 快速开始

### 安装

```bash
cargo install mqtt-mcp-server
```

### 配置

```bash
cp config.example.yaml config.yaml
# 编辑 config.yaml：设置 MQTT Broker 地址和（可选）AI 模型
```

### 启动

```bash
# Stdio 模式（对接 Claude Desktop 等 MCP 客户端）
mqtt-mcp-server --config config.yaml --mode stdio

# SSE 模式（HTTP 服务，对接远程 Agent）
mqtt-mcp-server --config config.yaml --mode sse --listen 127.0.0.1:3000
```

### 配置 Claude Desktop

在 `claude_desktop_config.json` 中添加：

```json
{
  "mcpServers": {
    "mqtt": {
      "command": "mqtt-mcp-server",
      "args": ["--config", "/你的路径/config.yaml", "--mode", "stdio"]
    }
  }
}
```

Claude 立刻就能跟你的 IoT 设备说话了。

---

## AI Agent 能做什么

接入之后，你的 AI Agent 可以：

- **订阅** MQTT 主题，实时监控设备数据
- **发布** 控制指令（比如"关掉 3 号泵"）
- **查询** 当前传感器值和历史趋势
- **分析** 设备健康状态（AI 异常检测、预测性维护）
- **管理告警** —— 设备异常时自动推送

### 对话示例

```
用户: "3号泵现在温度多少？"
AI:   [调用 mqtt_query_snapshot] → 87°C
AI:   "3号泵当前 87°C，已超过 85°C 阈值。需要我进一步分析吗？"

用户: "分析一下趋势。"
AI:   [调用 mqtt_query_range + mqtt_analyze]
AI:   "过去 5 分钟温度以 2°C/分钟速度上升，疑似冷却系统故障。
       建议：降低负载，并检查冷却液回路。"
```

---

## 提供的 MCP 工具

| 工具名称 | 功能 |
|---------|------|
| `mqtt_subscribe` | 订阅 MQTT 主题 |
| `mqtt_publish` | 向 MQTT 主题发布消息 |
| `mqtt_list_devices` | 列出所有已注册设备 |
| `mqtt_query_snapshot` | 查询设备最新遥测数据 |
| `mqtt_query_range` | 查询历史遥测数据 |
| `mqtt_send_command` | 向设备发送控制命令 |
| `mqtt_get_alerts` | 获取近期告警列表 |
| `mqtt_analyze` | AI 驱动的设备健康分析 |

---

## 规则引擎

在 `config.yaml` 中声明规则，MQTT 消息自动评估触发告警：

```yaml
rules:
  - name: "高温告警"
    device: "pump/*"        # 匹配 device/pump/xxx
    metric: "temperature"
    condition: "value > 80"  # 温度超过 80°C 触发
    action: "alert"
    ai_enhance: true         # 触发后自动 LLM 分析

  - name: "设备离线"
    device: "*"
    metric: "status"
    condition: "last_seen > 300s"  # 5 分钟无数据
    action: "alert"
    ai_enhance: false
```

**支持的 condition 语法：**

| 表达式 | 含义 | 示例 |
|--------|------|------|
| `value > 85` | 数值阈值 | 温度超标 |
| `rate > 5` | 每分钟变化率 | 温度飙升 |
| `last_seen > 300s` | 离线检测 | 设备断连 |

**严重程度自动分级**（规则名含"温度"时）：80~88→info，88~100→warning，100+→critical。告警含 AI 分析结果，Dashboard 实时展示。

---

## 定价

| 版本 | 价格 | 包含功能 |
|------|------|---------|
| **开源版** | 免费 (MIT) | 完整 MCP Server、单 Broker、本地规则引擎、基础 AI Bridge |
| **Pro 版** | ¥149/节点/月 ($49/节点/月) | 多节点管理面板、多 Broker、行业模板、告警推送 |
| **企业版** | 定制报价 | 私有化部署、定制协议(Modbus/OPC-UA)、SSO、SLA |

> AI 调用费由客户自备 API Key，我们绝不替客户垫 token 费。

---

## 架构

```
AI Agent ←→ MCP 协议 ←→ MQTT MCP Server ←→ MQTT Broker ←→ IoT 设备
```

- **Rust 编写** —— 单二进制文件，<10MB，内存安全
- 支持 **stdio** 和 **SSE** 两种传输模式
- 内置**规则引擎**，支持自定义 DSL
- **AI Bridge**：本地预过滤 + LLM 深度分析，省 token

---

## 国内 LLM 支持

已内置支持以下国内模型（客户自备 Key）：

| 模型 | provider 配置值 | Base URL |
|------|----------------|----------|
| DeepSeek | `deepseek` | `https://api.deepseek.com/v1` |
| 通义千问 | `qwen` | `https://dashscope.aliyuncs.com/compatible-mode/v1` |
| 智谱 GLM | `zhipu` | `https://open.bigmodel.cn/api/paas/v4` |
| OpenAI 兼容 | `custom` | 自定义 endpoint |

---

## 环境要求

- Rust 1.75+（从源码编译时需要）
- 一个 MQTT Broker（如 mosquitto、EMQX、HiveMQ）
- （可选）LLM API Key，启用 AI 分析功能

---

## 许可证

MIT — 详见 [LICENSE](LICENSE)

---

用 Rust 连接 AI 与物理世界。🚀
