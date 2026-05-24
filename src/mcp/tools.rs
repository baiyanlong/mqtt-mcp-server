//! MCP Tools 注册表 — AI Agent 可调用的函数集合。
//!
//! 每个 Tool 对应一个 MQTT 操作或设备交互。
//! Tool 是 AI Agent 与物理设备交互的主要方式。

use serde::{Deserialize, Serialize};

/// Tool 参数定义（MCP 协议兼容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub required: bool,
    #[serde(rename = "type")]
    pub param_type: String,
}

/// Tool 定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParam>,
}

/// 返回所有可用的 MCP Tools
pub fn all_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "mqtt_subscribe".into(),
            description: "订阅一个 MQTT 主题，实时接收设备数据。支持通配符：+ 匹配单层，# 匹配多层。\n例如：'sensors/+/temperature' 或 'building/#'".into(),
            parameters: vec![
                ToolParam {
                    name: "topic".into(),
                    description: "要订阅的 MQTT 主题（支持通配符 + 和 #）".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "qos".into(),
                    description: "服务质量：0（最多一次）、1（至少一次）、2（恰好一次）".into(),
                    required: false,
                    param_type: "integer".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_publish".into(),
            description: "向 MQTT 主题发布消息。用于发送指令或数据到设备。控制类指令建议使用 mqtt_send_command。".into(),
            parameters: vec![
                ToolParam {
                    name: "topic".into(),
                    description: "目标 MQTT 主题".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "payload".into(),
                    description: "消息内容（建议用 JSON 格式）".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "qos".into(),
                    description: "QoS 级别（0、1、2）".into(),
                    required: false,
                    param_type: "integer".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_list_devices".into(),
            description: "列出所有已注册的 MQTT 设备，包含在线/离线状态和最后活跃时间。".into(),
            parameters: vec![],
        },
        Tool {
            name: "mqtt_query_snapshot".into(),
            description: "查询指定设备和指标的最新遥测值。立即返回最近一条数据，无需订阅。".into(),
            parameters: vec![
                ToolParam {
                    name: "device_id".into(),
                    description: "设备唯一标识".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "metric".into(),
                    description: "指标名（如 'temperature'、'humidity'、'status'）".into(),
                    required: true,
                    param_type: "string".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_query_range".into(),
            description: "查询设备在指定时间范围内的历史遥测数据，返回有序数据点用于趋势分析。".into(),
            parameters: vec![
                ToolParam {
                    name: "device_id".into(),
                    description: "设备唯一标识".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "metric".into(),
                    description: "指标名".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "from".into(),
                    description: "起始时间，ISO 8601 格式（如 '2026-05-23T10:00:00Z'）或相对时间（如 '1h'、'30m'）".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "to".into(),
                    description: "结束时间（默认当前时间）".into(),
                    required: false,
                    param_type: "string".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_send_command".into(),
            description: "向指定设备发送控制命令。自动将逻辑设备 ID 映射到对应的 MQTT 主题。".into(),
            parameters: vec![
                ToolParam {
                    name: "device_id".into(),
                    description: "目标设备标识".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "command".into(),
                    description: "要执行的命令（如 'reboot'、'set_config'、'start'、'stop'）".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "params".into(),
                    description: "可选命令参数，JSON 字符串".into(),
                    required: false,
                    param_type: "string".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_get_alerts".into(),
            description: "获取规则引擎产生的告警列表，按严重程度和时间排序。用于监控面板和状态检查。".into(),
            parameters: vec![
                ToolParam {
                    name: "severity".into(),
                    description: "按严重程度过滤：critical、warning、info，或 all".into(),
                    required: false,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "limit".into(),
                    description: "最大返回条数（默认 20）".into(),
                    required: false,
                    param_type: "integer".into(),
                },
            ],
        },
        Tool {
            name: "mqtt_analyze".into(),
            description: "用 LLM 分析设备当前状态。聚合近期遥测数据发送给 AI 模型，返回人类可读的分析报告，包含异常评估和建议。\n这是核心差异化功能——把原始 IoT 数据转化为可执行的洞察。".into(),
            parameters: vec![
                ToolParam {
                    name: "device_id".into(),
                    description: "要分析的设备".into(),
                    required: true,
                    param_type: "string".into(),
                },
                ToolParam {
                    name: "window".into(),
                    description: "分析窗口时长（如 '5m'、'1h'、'24h'），默认 5m".into(),
                    required: false,
                    param_type: "string".into(),
                },
            ],
        },
    ]
}
