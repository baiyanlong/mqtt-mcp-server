# FAQ

## AI 需要部署在树莓派上吗？

**不需要。** 大模型跑在你的电脑或云端。树莓派只负责协议翻译——把 AI 指令转成 MQTT 消息，把设备数据转成 MCP 格式。

## 支持哪些 AI 模型？

DeepSeek、通义千问、智谱 GLM、OpenAI、Ollama 本地模型，以及任何 OpenAI 兼容接口。

## AI 分析的准确性怎么来的？

两层：规则引擎做数学判断（`value > 80` 是精确的），LLM 做语义解释（"为什么温度高？可能是什么原因？"）。

## 设备怎么注册？

**自动注册。** 任何向 MQTT 发了消息的设备，会自动出现在设备列表。不需要手动添加。

## 能同时控制多少个设备？

理论上无限。MQTT Broker 的吞吐量是瓶颈，一个 mosquitto 实例通常能支撑数千设备。

## 支持哪些 MQTT Broker？

mosquitto、EMQX、HiveMQ、RabbitMQ MQTT 插件——任何兼容 MQTT 3.1.1/5.0 的 Broker。

## 嵌套 JSON 数据怎么解析？

在 `config.yaml` 里配置设备映射：

```yaml
devices:
  - id: "pump1"
    mappings:
      - metric: "temperature"
        json_path: "sensor.temperature.value"
```

支持任意深度嵌套，用点号分隔路径。

## Pro 版和开源版有什么区别？

| | 开源版 | Pro |
|------|--------|-----|
| MCP Server | ✅ | ✅ |
| 规则引擎 | ✅ | ✅ |
| Dashboard | 本地 | 云多节点 |
| 多节点管理 | ❌ | ✅ |
| OTA 升级 | ❌ | 🚧 规划中 |
| 历史数据 | SQLite | PostgreSQL |

## 怎么收费？

开源版 MIT 免费。Pro 版 ¥2,980 永久授权，≤20 节点。AI 调用费客户自备 Key。
