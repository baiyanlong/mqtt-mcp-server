# AI 配置

启用 AI 分析后，规则引擎触发告警时可自动调用 LLM 深度分析。

## 快速启用

```bash
# Ollama 本地模型（免费，离线）
./mqtt-mcp-server --ai --ai-provider ollama --ai-model qwen2.5:7b

# DeepSeek
./mqtt-mcp-server --ai --ai-provider deepseek --ai-model deepseek-chat --ai-key sk-xxx

# 通义千问
./mqtt-mcp-server --ai --ai-provider qwen --ai-model qwen-turbo --ai-key sk-xxx
```

## 支持的 Provider

| Provider | 配置值 | Base URL | 计费 |
|----------|--------|----------|------|
| Ollama | `ollama` | `localhost:11434/v1` | 免费 |
| DeepSeek | `deepseek` | `api.deepseek.com/v1` | 按 token |
| 通义千问 | `qwen` | `dashscope.aliyuncs.com` | 按 token |
| 智谱 GLM | `zhipu` | `open.bigmodel.cn` | 按 token |
| OpenAI | `openai` | `api.openai.com/v1` | 按 token |
| 自定义 | `custom` | 任意 | 自定 |

## YAML 配置

```yaml
ai:
  enabled: true
  provider: "deepseek"
  model: "deepseek-chat"
  api_key: "${DEEPSEEK_KEY}"   # 支持环境变量
  base_url: ""                  # 留空用默认
  window_size: 100              # AI 分析取最近 N 条数据
```

## 工作原理

1. 规则引擎触发告警（如温度 > 80°C）
2. 收集该设备最近 `window_size` 条历史数据
3. 构造 prompt 发给 LLM
4. LLM 返回：异常判断、严重程度、原因分析、建议措施

> AI 调用费由客户自备 API Key，我们不垫 token 费。
