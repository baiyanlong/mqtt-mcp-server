//! 数据过滤与清洗工具。
//!
//! 预处理原始 MQTT 载荷，处理 IoT 数据常见的质量问题：
//! 缺失值、时间戳不齐、异常量程。

/// 将原始 MQTT 载荷解析为数值
/// 支持纯数字和 JSON 格式（自动提取常见字段）
pub fn parse_value(payload: &str) -> Option<f64> {
    // 先尝试直接解析数字
    if let Ok(v) = payload.trim().parse::<f64>() {
        return Some(clamp(v));
    }

    // JSON 格式：查找常见数值字段
    if let Ok(obj) = serde_json::from_str::<serde_json::Value>(payload) {
        for field in &[
            "value", "val", "temperature", "humidity",
            "pressure", "power", "speed", "status",
        ] {
            if let Some(v) = obj.get(field).and_then(|v| v.as_f64()) {
                return Some(clamp(v));
            }
        }
        // 如果 JSON 只有一个数值字段，直接取它
        if let Some(obj) = obj.as_object() {
            let numeric_values: Vec<f64> = obj.values()
                .filter_map(|v| v.as_f64())
                .collect();
            if numeric_values.len() == 1 {
                return Some(clamp(numeric_values[0]));
            }
        }
    }

    None
}

/// 限制值到合理范围（过滤明显异常的传感器读数）
fn clamp(value: f64) -> f64 {
    if value.is_nan() || value.is_infinite() {
        return 0.0;
    }
    // 物理合理范围（后续可针对不同设备类型覆盖）
    if value < -273.15 {
        -273.15 // 低于绝对零度 — 传感器故障
    } else if value > 1_000_000.0 {
        1_000_000.0 // 不合理的高值
    } else {
        value
    }
}

/// 从 MQTT 主题中提取设备 ID
/// 支持模式：device/{id}/metric、devices/{id}/...、{prefix}/{id}/...
pub fn extract_device_id(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() < 2 {
        return None;
    }

    // 常见模式：device/xxx/metric
    for i in 0..parts.len() {
        if parts[i] == "device" || parts[i] == "devices" {
            if i + 1 < parts.len() {
                return Some(parts[i + 1].to_string());
            }
        }
    }

    // 兜底：用第二段作为设备 ID
    Some(parts[1].to_string())
}

/// 从 MQTT 主题中提取指标名
pub fn extract_metric(topic: &str) -> Option<String> {
    let parts: Vec<&str> = topic.split('/').collect();
    if parts.len() < 2 {
        return Some("value".to_string());
    }

    // 用最后一段作为指标名
    parts.last().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_number() {
        assert_eq!(parse_value("25.5"), Some(25.5));
        assert_eq!(parse_value("100"), Some(100.0));
    }

    #[test]
    fn test_parse_json_value() {
        let json = r#"{"value": 72.3, "unit": "celsius"}"#;
        assert_eq!(parse_value(json), Some(72.3));
    }

    #[test]
    fn test_clamp_nan() {
        assert_eq!(clamp(f64::NAN), 0.0);
    }

    #[test]
    fn test_extract_device_id() {
        assert_eq!(
            extract_device_id("device/pump3/temperature"),
            Some("pump3".into())
        );
    }

    #[test]
    fn test_extract_metric() {
        assert_eq!(
            extract_metric("device/pump3/temperature"),
            Some("temperature".into())
        );
    }
}
