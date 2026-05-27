# SSE 端点

SSE 模式下，MQTT MCP Server 提供 HTTP SSE 接口。

## 端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/sse` | GET | SSE 长连接，获取 MCP 事件流 |
| `/message` | POST | JSON-RPC 消息发送 |

## 连接流程

```
1. AI Agent → GET /sse
2. Server 返回: event: endpoint
               data: /message?sessionId=abc123
3. AI Agent → POST /message?sessionId=abc123
              {"jsonrpc":"2.0","method":"initialize",...}
4. Server → SSE: initialize response
5. AI Agent → POST /message
              {"jsonrpc":"2.0","method":"tools/list",...}
6. Server → SSE: tools list
```

## 示例

```bash
# 连接 SSE
curl --noproxy '*' -s http://localhost:3000/sse
# event: endpoint
# data: /message?sessionId=e8358944-...

# 发送 JSON-RPC
curl --noproxy '*' -s -X POST \
  "http://localhost:3000/message?sessionId=xxx" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}'
```

## MCP Inspector

```bash
npx @anthropic-ai/mcp-inspector sse http://localhost:3000/sse
```
