# 什么是 MCP

## MCP = AI 的 USB 协议

**MCP（Model Context Protocol）** 是 Anthropic（Claude 母公司）2025 年推出的开放标准。类比：

| 时代 | 协议 | 作用 |
|------|------|------|
| 90 年代 | HTTP | 浏览器 ↔ 服务器 |
| 2010 年代 | REST API | APP ↔ 后端 |
| **2025 年** | **MCP** | **AI ↔ 任何工具** |

USB 让电脑即插即用任何硬件。MCP 让 AI 即插即用任何工具——数据库、文件系统、GitHub、MQTT 设备。

## 没有 MCP vs 有 MCP

```
没有 MCP：
  AI 只能聊天。想查数据库？你写 Python 调 API → 告诉 AI 结果。
  AI 没有"手"，所有外部操作都要你当中间人。

有 MCP：
  AI 自动发现工具 → 自动构造参数 → 自动调用 → 自动解读结果。
  你只需要说话，AI 自己干活。
```

## MQTT MCP Server 做了什么

它在 MCP 和 MQTT 之间架桥：

```
Claude Desktop                  MQTT MCP Server              工厂设备
─────────                       ───────────────              ────────
"3号泵温度？"            →      mqtt_query_snapshot   →      87°C
"关掉3号泵"              →      mqtt_send_command     →      泵停机
"有异常吗？"             →      mqtt_get_alerts       →      告警列表
```

AI 看到的是 **8 个工具函数**，每个函数背后是真实的物理设备操作。

## 生态正在爆发

| 客户端 | 状态 |
|--------|------|
| Claude Desktop | ✅ 内置 |
| Cursor | ✅ 内置 |
| Windsurf | ✅ 内置 |
| Continue.dev | ✅ 内置 |
| MCP Inspector | ✅ 调试工具 |

你写一个 MCP Server，几十个 AI 客户端都能用。
