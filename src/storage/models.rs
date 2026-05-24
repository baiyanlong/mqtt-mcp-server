//! 持久化数据模型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 注册设备
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub device_type: Option<String>,
    pub registered_at: DateTime<Utc>,
    pub last_seen: Option<DateTime<Utc>>,
    pub is_online: bool,
}

/// 遥测记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryRecord {
    pub id: Option<i64>,
    pub device_id: String,
    pub metric: String,
    pub value: f64,
    pub raw_payload: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 告警记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: Option<i64>,
    pub rule_name: String,
    pub device_id: String,
    pub metric: String,
    pub value: f64,
    pub severity: String,
    pub message: String,
    pub ai_analysis: Option<String>,
    pub acknowledged: bool,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

/// 许可证记录（Pro/Enterprise 授权）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub id: Option<i64>,
    pub license_key: String,
    pub tier: String, // "pro"、"enterprise"
    pub node_limit: i32,
    pub activated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}
