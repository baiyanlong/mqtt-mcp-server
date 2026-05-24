//! 本地异常检测 — 轻量级统计检查，不调用 LLM。
//!
//! 用作 AI 分析的前置过滤器，拦截约 99% 的明显正常数据，
//! 只把可疑模式转发给 LLM，大幅节省 token 消耗。

use chrono::{DateTime, Utc};

/// Z-score 异常检测
/// 当值偏离均值超过 threshold 个标准差时返回 true
pub fn z_score(value: f64, window: &[(DateTime<Utc>, f64)], threshold: f64) -> bool {
    if window.len() < 3 {
        return false; // 数据不足
    }

    let values: Vec<f64> = window.iter().map(|(_, v)| *v).collect();
    let n = values.len() as f64;
    let mean: f64 = values.iter().sum::<f64>() / n;
    let variance: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    if std_dev < f64::EPSILON {
        return false; // 无波动，无法检测异常
    }

    let z = (value - mean).abs() / std_dev;
    z > threshold
}

/// 变化率检测
/// 当窗口内每分钟变化超过 max_rate 时返回 true
pub fn rate_of_change(window: &[(DateTime<Utc>, f64)], max_rate: f64) -> bool {
    if window.len() < 2 {
        return false;
    }

    let first = window.first().unwrap();
    let last = window.last().unwrap();

    let time_diff_minutes = (last.0 - first.0).num_seconds() as f64 / 60.0;
    if time_diff_minutes <= 0.0 {
        return false;
    }

    let value_change = (last.1 - first.1).abs();
    let rate = value_change / time_diff_minutes;

    rate > max_rate
}

/// 死区检测
/// 传感器长时间返回完全相同的值 — 可能卡死或断连
pub fn dead_band(window: &[(DateTime<Utc>, f64)], max_flat_minutes: f64) -> bool {
    if window.len() < 5 {
        return false;
    }

    let first = window.first().unwrap();
    let last = window.last().unwrap();
    let time_diff_minutes = (last.0 - first.0).num_seconds() as f64 / 60.0;

    if time_diff_minutes < max_flat_minutes {
        return false;
    }

    let reference = window[0].1;
    window
        .iter()
        .all(|(_, v)| (v - reference).abs() < f64::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z_score_anomaly() {
        let now = Utc::now();
        let window = vec![
            (now, 20.0), (now, 21.0), (now, 20.0), (now, 19.0), (now, 20.0),
        ];
        // 50 远远超出正常范围
        assert!(z_score(50.0, &window, 3.0));
        // 20 正常
        assert!(!z_score(20.0, &window, 3.0));
    }

    #[test]
    fn test_rate_of_change() {
        let now = Utc::now();
        let one_min_ago = now - chrono::Duration::minutes(1);
        let window = vec![
            (one_min_ago, 20.0),
            (now, 30.0), // 每分钟变化 10
        ];
        assert!(rate_of_change(&window, 5.0));
        assert!(!rate_of_change(&window, 20.0));
    }
}
