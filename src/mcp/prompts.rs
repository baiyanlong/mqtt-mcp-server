//! MCP Prompts — 预置分析 Prompt 模板。
//!
//! 给 AI Agent 提供开箱即用的 IoT 场景分析模板。
//! 用户也可以在配置文件中自定义 Prompt。

/// Prompt 模板
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub template: String,
}

/// 所有内置 Prompt 模板
pub fn all_prompts() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "analyze_device_health".into(),
            description: "基于近期遥测数据分析设备整体健康状态".into(),
            template: r#"分析设备 {device_id} 的健康状态。

近期遥测数据：
{telemetry}

考虑以下几点：
1. 是否有指标超出正常范围？
2. 是否有趋势预示即将发生故障？
3. 建议采取什么行动？

请给出简明评估，标注严重程度（正常/警告/严重）和可行的后续步骤。"#.into(),
        },
        Prompt {
            name: "summarize_alerts".into(),
            description: "汇总近期告警，生成日报".into(),
            template: r#"汇总过去 {time_range} 内的告警：

{alerts}

按以下分组：
1. 需要立即处理的严重问题
2. 需要关注的警告
3. 已解决的问题

保持简洁，适合交接班报告。"#.into(),
        },
        Prompt {
            name: "energy_optimization".into(),
            description: "分析能耗数据，提出优化建议".into(),
            template: r#"分析设备 {device_ids} 在 {time_range} 内的能耗：

{energy_data}

识别：
1. 峰值用电时段
2. 相对基线的异常
3. 优化机会及预估节省
4. 建议的排程调整

以要点报告形式输出。"#.into(),
        },
    ]
}
