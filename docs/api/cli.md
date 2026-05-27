# CLI 参数

## 完整参数列表

```
mqtt-mcp-server [OPTIONS]
```

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `-c, --config` | `config.yaml` | 配置文件路径 |
| `--mode` | `stdio` | 传输模式：stdio / sse |
| `--listen` | `127.0.0.1:3000` | SSE 监听地址 |
| `--web` | `8080` | Web Dashboard 端口 |
| `--no-web` | false | 禁用 Dashboard |
| `--broker` | — | MQTT Broker（覆盖配置） |
| `--topics` | — | 订阅主题（逗号分隔） |
| `--mqtt-user` | — | MQTT 用户名 |
| `--mqtt-pass` | — | MQTT 密码 |
| `--ai` | — | 启用 AI 分析 |
| `--ai-provider` | — | openai / deepseek / qwen / zhipu / ollama / custom |
| `--ai-model` | — | 模型名称 |
| `--ai-key` | — | API Key |
| `--ai-base-url` | — | 自定义 Base URL |
| `--ai-window` | `100` | AI 分析窗口大小 |
| `--db` | — | SQLite 路径 |
| `--cloud` | — | 云服务地址（Pro） |
| `--cloud-key` | — | 云服务 API Key（Pro） |

## 常用启动命令

```bash
# 开发调试：SSE + Dashboard
./mqtt-mcp-server --mode sse --listen 0.0.0.0:3000 --broker tcp://localhost:1883 --topics '#'

# 生产部署：SSE + AI + Dashboard + 云上报
./mqtt-mcp-server \
  --mode sse --listen 0.0.0.0:3000 \
  --broker tcp://localhost:1883 --topics '#' \
  --ai --ai-provider deepseek --ai-model deepseek-chat --ai-key sk-xxx \
  --cloud https://dashboard.example.com --cloud-key node-xxx
```
