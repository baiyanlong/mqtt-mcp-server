# Cloud API (Pro)

Pro 版云服务提供 REST API，用于接收边缘节点心跳和告警。

## 端点一览

| 端点 | 方法 | 认证 | 说明 |
|------|------|------|------|
| `/api/v1/nodes/register` | POST | API Key | 节点注册 |
| `/api/v1/nodes/heartbeat` | POST | API Key | 心跳上报 |
| `/api/v1/nodes` | GET | API Key | 节点列表 |
| `/api/v1/alerts` | POST | API Key | 上报告警 |
| `/api/v1/alerts` | GET | API Key | 查询告警 |
| `/api/v1/dashboard` | GET | API Key | 仪表盘摘要 |
| `/health` | GET | 无 | 健康检查 |

## 认证

所有 `/api/*` 路由需 `Authorization` header：

```
Authorization: your-api-key
```

## 心跳上报

```bash
curl -X POST https://dashboard.example.com/api/v1/nodes/heartbeat \
  -H "Authorization: key-xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "factory-1-pi",
    "version": "0.3.0",
    "uptime_secs": 86400,
    "device_count": 12,
    "alert_count": 3,
    "mqtt_connected": true
  }'
```

## 上报告警

```bash
curl -X POST https://dashboard.example.com/api/v1/alerts \
  -H "Authorization: key-xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "node_id": "factory-1-pi",
    "device_id": "pump/3",
    "rule_name": "高温告警",
    "severity": "warning",
    "message": "温度 88°C 超过阈值",
    "value": 88.0,
    "metric": "temperature",
    "timestamp": "2026-05-25T10:00:00Z"
  }'
```

## 启动

```bash
mqtt-mcp-cloud --listen 0.0.0.0:8080 --db postgres://user:pass@localhost/mqttmcp
```
