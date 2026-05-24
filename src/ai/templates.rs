//! Prompt 模板库 — 预置 IoT 场景分析 Prompt。
//!
//! 随产品发布，用户可自定义扩展。
//! Pro 用户获得扩展模板库（行业专用模板）。

/// 模板变量 — 运行时替换的占位符
#[derive(Debug, Clone)]
pub struct TemplateVar {
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// 命名 Prompt 模板
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<TemplateVar>,
    pub category: TemplateCategory,
}

/// 模板分类
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateCategory {
    General,
    Industrial,
    Building,
    Energy,
    ProFeature,
}

/// 所有内置 Prompt 模板
pub fn builtin_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate {
            name: "device_health_check".into(),
            description: "单设备快速健康评估".into(),
            category: TemplateCategory::General,
            variables: vec![
                TemplateVar {
                    name: "device_id".into(),
                    description: "设备标识".into(),
                    required: true,
                },
                TemplateVar {
                    name: "telemetry".into(),
                    description: "近期遥测数据（自动填充）".into(),
                    required: true,
                },
            ],
            template: r#"分析 {device_id} 的当前健康状态：

近期遥测：
{telemetry}

评估标准：
1. 是否有指标超出正常工作范围？
2. 是否有趋势预示潜在故障？
3. 评定整体健康度：健康 / 降级 / 有风险 / 严重
4. 操作员应执行什么操作？

保持简洁——操作员在值班。"#.into(),
        },
        PromptTemplate {
            name: "pump_analysis".into(),
            description: "泵系统专项分析（振动、流量、温度）".into(),
            category: TemplateCategory::Industrial,
            variables: vec![
                TemplateVar {
                    name: "device_id".into(),
                    description: "泵设备 ID".into(),
                    required: true,
                },
                TemplateVar {
                    name: "telemetry".into(),
                    description: "近期遥测数据".into(),
                    required: true,
                },
            ],
            template: r#"分析泵 {device_id}：

遥测数据：
{telemetry}

泵专项检查：
1. 振动是否在允许范围内？振动上升 = 轴承磨损。
2. 流量是否稳定？下降可能表示堵塞或叶轮损坏。
3. 温度是否正常？过热 = 润滑失效或过载。
4. NPSH（净正吸入压头）是否足够？气蚀风险。

给出泵健康评分（0-100）和具体维护建议。"#.into(),
        },
        PromptTemplate {
            name: "energy_report".into(),
            description: "能耗分析与优化建议".into(),
            category: TemplateCategory::Energy,
            variables: vec![
                TemplateVar {
                    name: "device_ids".into(),
                    description: "设备 ID 列表（逗号分隔）".into(),
                    required: true,
                },
                TemplateVar {
                    name: "time_range".into(),
                    description: "分析窗口".into(),
                    required: true,
                },
                TemplateVar {
                    name: "energy_data".into(),
                    description: "能耗数据".into(),
                    required: true,
                },
            ],
            template: r#"分析设备 {device_ids} 的能耗
时间范围：{time_range}

数据：
{energy_data}

分析：
1. 识别峰值用电时段
2. 与基线对比（上周均值）
3. 标记异常（>30% 偏差）
4. 提出具体优化方案及预估节省
5. 建议排程调整方案

以要点报告形式输出。"#.into(),
        },
    ]
}
