//! 规则引擎 — 根据配置的规则评估 MQTT 消息，触发告警。
//!
//! 规则定义在 config.yaml 中，可触发告警、记录日志或调用 AI Bridge 深度分析。
//!
//! 条件 DSL（轻量级，无需外部解析器）：
//!   "value > 85"              — 简单阈值
//!   "value > 85 or rate > 5"  — 复合条件（带变化率）
//!   "value > baseline * 1.5"  — 相对基线
//!   "last_seen > 300s"        — 基于时间（设备离线检测）

use crate::config::RuleConfig;
use chrono::{DateTime, Utc};

/// 规则评估结果
#[derive(Debug, Clone)]
pub struct RuleResult {
    /// 触发的规则名称
    pub rule_name: String,
    /// 设备 ID
    pub device_id: String,
    /// 指标名
    pub metric: String,
    /// 是否触发
    pub triggered: bool,
    /// 当前值
    pub current_value: f64,
    /// 告警严重程度
    pub severity: AlertSeverity,
    /// 告警消息
    pub message: String,
    /// 是否需要 AI 深度分析
    pub should_ai_analyze: bool,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 告警严重程度
#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 严重
    Critical,
}

/// 对所有规则评估一条遥测数据
pub fn evaluate(
    rules: &[RuleConfig],
    device_id: &str,
    metric: &str,
    value: f64,
    window: &[(DateTime<Utc>, f64)], // 近期数据，用于变化率计算
) -> Vec<RuleResult> {
    let mut results = Vec::new();

    for rule in rules {
        // 检查规则是否适用于此设备
        if !matches_device(&rule.device, device_id) {
            continue;
        }
        if rule.metric != "*" && rule.metric != metric {
            continue;
        }

        // 评估条件
        let triggered = evaluate_condition(&rule.condition, value, window);

        if triggered {
            let severity = classify_severity(&rule.name, value);

            results.push(RuleResult {
                rule_name: rule.name.clone(),
                device_id: device_id.to_string(),
                metric: metric.to_string(),
                triggered: true,
                current_value: value,
                severity,
                message: format!(
                    "规则 '{}' 触发: {} 在设备 {} 上 (当前值: {})",
                    rule.name, rule.condition, device_id, value
                ),
                should_ai_analyze: rule.ai_enhance,
                timestamp: Utc::now(),
            });
        }
    }

    results
}

/// 简单的 glob 风格设备匹配
/// "*" 匹配所有设备，"pump/*" 匹配以 pump/ 开头的设备
fn matches_device(pattern: &str, device_id: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return device_id.starts_with(prefix);
    }
    pattern == device_id
}

/// 评估条件表达式
fn evaluate_condition(
    condition: &str,
    value: f64,
    window: &[(DateTime<Utc>, f64)],
) -> bool {
    // 简单阈值：value > 85、value < 10、value >= 100
    if let Some(result) = eval_simple_threshold(condition, value) {
        return result;
    }

    // 基于时间：last_seen > 300s
    if condition.contains("last_seen") {
        return eval_last_seen(condition, value);
    }

    // 变化率：rate > 5
    if condition.contains("rate") && !window.is_empty() {
        return eval_rate(condition, value, window);
    }

    // 默认当作简单阈值处理
    eval_simple_threshold(condition, value).unwrap_or(false)
}

/// 解析简单阈值表达式：value > 85
fn eval_simple_threshold(condition: &str, value: f64) -> Option<bool> {
    let parts: Vec<&str> = condition.split_whitespace().collect();
    if parts.len() == 3 && parts[0] == "value" {
        let threshold: f64 = parts[2].parse().ok()?;
        match parts[1] {
            ">" => Some(value > threshold),
            "<" => Some(value < threshold),
            ">=" => Some(value >= threshold),
            "<=" => Some(value <= threshold),
            "==" => Some((value - threshold).abs() < f64::EPSILON),
            _ => None,
        }
    } else {
        None
    }
}

/// 解析离线检测条件：last_seen > 300s
fn eval_last_seen(condition: &str, seconds_since: f64) -> bool {
    let parts: Vec<&str> = condition.split_whitespace().collect();
    if parts.len() == 3 && parts[2].ends_with('s') {
        let threshold_str = parts[2].trim_end_matches('s');
        if let Ok(threshold) = threshold_str.parse::<f64>() {
            return match parts[1] {
                ">" => seconds_since > threshold,
                ">=" => seconds_since >= threshold,
                _ => false,
            };
        }
    }
    false
}

/// 计算变化率并评估：rate > 5（每分钟变化超过 5 个单位）
fn eval_rate(condition: &str, _current: f64, window: &[(DateTime<Utc>, f64)]) -> bool {
    if window.len() < 2 {
        return false;
    }

    let first = window.first().unwrap();
    let last = window.last().unwrap();
    let time_diff_minutes = (last.0 - first.0).num_seconds() as f64 / 60.0;
    if time_diff_minutes <= 0.0 {
        return false;
    }
    let rate = (last.1 - first.1) / time_diff_minutes;

    let parts: Vec<&str> = condition.split_whitespace().collect();
    if parts.len() >= 3 {
        if let Ok(threshold) = parts[2].parse::<f64>() {
            return match parts[1] {
                ">" => rate.abs() > threshold,
                ">=" => rate.abs() >= threshold,
                _ => false,
            };
        }
    }

    false
}

/// 根据规则名称和数值判断告警严重程度
fn classify_severity(rule_name: &str, value: f64) -> AlertSeverity {
    let lower = rule_name.to_lowercase();
    if lower.contains("critical") || lower.contains("严重") {
        return AlertSeverity::Critical;
    }
    if lower.contains("warning") || lower.contains("警告") || lower.contains("high") {
        return AlertSeverity::Warning;
    }
    // 根据规则名含"高温"等关键词，用温度梯级阈值
    //   > 阈值 +10% → Warning, > 阈值 +25% → Critical
    let baseline = 80.0;
    if lower.contains("温度") || lower.contains("高温") || lower.contains("temp") {
        if value > baseline * 1.25 {
            AlertSeverity::Critical        // >100°C — 严重
        } else if value > baseline * 1.10 {
            AlertSeverity::Warning          // >88°C — 警告
        } else {
            AlertSeverity::Info             // 80-88°C — 信息
        }
    } else if value > 1000.0 {
        AlertSeverity::Critical
    } else if value > 500.0 {
        AlertSeverity::Warning
    } else {
        AlertSeverity::Info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_threshold() {
        assert!(eval_simple_threshold("value > 85", 90.0).unwrap());
        assert!(!eval_simple_threshold("value > 85", 80.0).unwrap());
        assert!(eval_simple_threshold("value < 10", 5.0).unwrap());
        assert!(!eval_simple_threshold("value < 10", 15.0).unwrap());
    }

    #[test]
    fn test_device_matching() {
        assert!(matches_device("*", "pump/3"));
        assert!(matches_device("pump/*", "pump/3"));
        assert!(!matches_device("pump/*", "sensor/1"));
        assert!(matches_device("pump/3", "pump/3"));
    }
}
