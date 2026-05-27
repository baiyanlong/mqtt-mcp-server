# 架构

## 部署拓扑

```
你的电脑 (Claude/Cursor/任何 MCP 客户端)
       ↕ HTTP SSE (端口 3000)
   mqtt-mcp-server (树莓派 / 边缘盒子 / Linux 服务器)
       ↕ MQTT (端口 1883)
   MQTT Broker (mosquitto / EMQX / HiveMQ)
       ↕
   IoT 设备群 (传感器 / 执行器 / PLC / 充电桩)
```

## 关键设计

### 大模型不跑在盒子上

树莓派只做**协议翻译**：

- AI 指令 → JSON-RPC → MQTT 消息 → 设备
- 设备数据 → MQTT → MCP 格式 → AI

因此盒子不需要 GPU，500 块钱的香橙派就能管一个车间。

### 两种传输模式

| 模式 | 适用场景 | 连接方式 |
|------|---------|---------|
| `stdio` | 本地 AI Agent | 标准输入输出 |
| `sse` | 远程 AI Agent | HTTP Server-Sent Events |

### 模块结构

```
src/
├── mcp/          MCP 协议层 (ServerHandler, tools, resources)
├── mqtt/         MQTT 接入 (rumqttc 封装, 事件循环)
├── engine/       引擎层 (规则引擎, 滑动窗口缓存, 数据过滤)
├── ai/           AI Bridge (多 Provider 支持)
├── storage/      SQLite 存储 (设备注册, 告警)
├── web/          Web Dashboard (内嵌 HTML)
├── reporter.rs   Pro 上报代理 (心跳 + 告警推送)
├── cloud/        Pro 云服务 (多节点管理)
└── bin/cloud.rs  Pro 云服务入口
```

## 数据流

```
MQTT 消息到达
    ↓
filter::parse_value_by_path  (JSON 路径映射)
    ↓
cache::insert               (写入滑动窗口)
    ↓
rules::evaluate             (规则引擎评估)
    ↓ 触发告警?
    ├── 是 → storage::insert_alert + reporter::push_alert
    └── 否 → 结束
```
