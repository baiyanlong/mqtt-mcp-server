//! MCP Resources — AI Agent 可"读取"的结构化数据资源。
//!
//! Resource 提供对设备数据、遥测和告警的只读访问。
//! 使用 URI 模式：devices://、telemetry://、alerts://

/// Resource 定义
pub struct Resource {
    pub uri_pattern: String,
    pub name: String,
    pub description: String,
}

/// 所有可用的 MCP Resources
pub fn all_resources() -> Vec<Resource> {
    vec![
        Resource {
            uri_pattern: "devices://list".into(),
            name: "设备列表".into(),
            description: "所有已注册设备的 JSON 列表，含在线/离线状态和元数据".into(),
        },
        Resource {
            uri_pattern: "devices://{id}/info".into(),
            name: "设备详情".into(),
            description: "指定设备的详细信息，包括类型、最后活跃时间和配置".into(),
        },
        Resource {
            uri_pattern: "telemetry://{device}/{metric}/latest".into(),
            name: "最新遥测值".into(),
            description: "指定设备和指标的最新遥测值".into(),
        },
        Resource {
            uri_pattern: "telemetry://{device}/{metric}/history?from=&to=".into(),
            name: "遥测历史".into(),
            description: "指定时间范围内的历史遥测数据。查询参数：from、to（ISO 8601）".into(),
        },
        Resource {
            uri_pattern: "alerts://active".into(),
            name: "活跃告警".into(),
            description: "所有当前活跃（未解决）的规则引擎告警".into(),
        },
        Resource {
            uri_pattern: "alerts://history?days=7".into(),
            name: "告警历史".into(),
            description: "历史告警数据用于趋势分析。查询参数：days（默认 7）".into(),
        },
    ]
}
