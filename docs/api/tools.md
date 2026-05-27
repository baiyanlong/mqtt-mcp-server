# MCP 工具参考

MQTT MCP Server 提供 8 个 MCP 工具，AI Agent 可自动调用。

## mqtt_subscribe

订阅 MQTT 主题。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| topic | string | ✅ | MQTT 主题（支持通配符 `+` `#`） |
| qos | number | ❌ | QoS 0/1/2，默认 1 |

## mqtt_publish

向 MQTT 主题发布消息。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| topic | string | ✅ | 目标主题 |
| payload | string | ✅ | 消息内容（建议 JSON） |
| qos | number | ❌ | 默认 1 |

## mqtt_list_devices

列出所有已注册设备。设备自动注册——任何发过 MQTT 消息的设备都会出现。

无参数。

## mqtt_query_snapshot

查询设备最新遥测值。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| device_id | string | ✅ | 设备 ID |
| metric | string | ✅ | 指标名（如 temperature） |

## mqtt_query_range

查询历史遥测数据。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| device_id | string | ✅ | 设备 ID |
| metric | string | ✅ | 指标名 |
| from | string | ✅ | 起始时间（ISO 8601 或 1h/30m） |
| to | string | ❌ | 结束时间，默认当前 |

## mqtt_send_command

向设备发送控制指令。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| device_id | string | ✅ | 目标设备 |
| command | string | ✅ | 命令名（如 reboot） |
| params | string | ❌ | JSON 参数 |

## mqtt_get_alerts

获取告警列表。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| severity | string | ❌ | 过滤：critical/warning/info |
| limit | number | ❌ | 默认 20 |

## mqtt_analyze

AI 驱动的设备健康分析。需要启用 AI 功能。

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| device_id | string | ✅ | 要分析的设备 |
| window | string | ❌ | 分析窗口（5m/1h/24h） |
