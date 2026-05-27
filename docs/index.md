---
layout: home

hero:
  name: "MQTT MCP Server"
  text: "让 AI 操控物理设备"
  tagline: 云端 AI + 树莓派边缘网关。Claude/GPT 通过自然语言控制 MQTT 设备
  actions:
    - theme: brand
      text: 快速开始
      link: /guide/getting-started
    - theme: alt
      text: GitHub
      link: https://github.com/baiyanlong/mqtt-mcp-server

features:
  - icon: 🔌
    title: MCP 标准协议
    details: 基于 Anthropic 2025 年推出的 Model Context Protocol，AI 自动发现工具、调用设备。Claude Desktop / Cursor 即插即用。
  - icon: 🥧
    title: 树莓派边缘网关
    details: 6.9MB Rust 二进制。大模型跑在云端，小盒子在现场做翻译官——把 AI 指令转成 MQTT 消息。
  - icon: 🤖
    title: AI 异常分析
    details: 本地规则引擎自动检测异常，可选 LLM 深度分析。支持 DeepSeek、通义千问、智谱、Ollama 等。
  - icon: 📊
    title: Web Dashboard
    details: 深色主题，设备状态 + 告警面板，3 秒自动刷新。一个 HTML 页面，零外部依赖。
  - icon: 🚀
    title: 5 分钟部署
    details: wget 二进制 + ./install.sh，systemd 自动管理。GitHub Actions 自动构建 ARM64。
  - icon: 🛡️
    title: 开源 MIT
    details: 完全免费开源。Rust 编写，内存安全。23 个自动化测试全绿。
---

## 一句话理解

```
你的电脑 (Claude/Cursor)                 工厂现场 (树莓派)
     │                                      │
     │  "3号泵温度多少？"                    │
     ├────────── HTTP SSE ──────────────────→│
     │                                      ├── MQTT → 查询传感器
     │  "87°C，建议检查冷却液"               │
     │←───────── HTTP SSE ───────────────────┤
```

**AI 不跑在树莓派上。** 小盒子只管协议转换。

## 赞助

<p align="center">
  <img src="/qq-group.jpg" width="240" alt="QQ群"><br>
  <b>IoT + AI 交流群</b>
</p>
