//! 云服务数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// 边缘节点
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Node {
    pub id: Uuid,
    pub node_id: String,
    pub name: String,
    pub version: String,
    pub last_heartbeat: DateTime<Utc>,
    pub device_count: i32,
    pub alert_count: i32,
    pub mqtt_connected: bool,
    pub status: String,       // online / offline
    pub cpu_percent: Option<f64>,
    pub mem_mb: Option<f64>,
    pub uptime_secs: Option<i64>,
    pub labels: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// 节点注册请求
#[derive(Debug, Deserialize)]
pub struct RegisterNodeRequest {
    pub node_id: String,
    pub version: String,
    pub name: Option<String>,
}

/// 心跳上报请求
#[derive(Debug, Deserialize)]
pub struct HeartbeatRequest {
    pub node_id: String,
    pub version: String,
    pub uptime_secs: u64,
    pub device_count: usize,
    pub alert_count: usize,
    pub mqtt_connected: bool,
    pub cpu_percent: Option<f64>,
    pub mem_mb: Option<f64>,
}

/// 告警上报请求
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRecord {
    pub id: Uuid,
    pub node_id: String,
    pub device_id: String,
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub value: f64,
    pub metric: String,
    pub ai_analysis: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 仪表盘摘要
#[derive(Debug, Serialize)]
pub struct DashboardSummary {
    pub total_nodes: i64,
    pub online_nodes: i64,
    pub total_alerts: i64,
    pub critical_alerts: i64,
    pub nodes: Vec<NodeSummary>,
}

#[derive(Debug, Serialize)]
pub struct NodeSummary {
    pub node_id: String,
    pub name: String,
    pub status: String,
    pub device_count: i32,
    pub alert_count: i32,
    pub uptime_secs: Option<i64>,
    pub last_heartbeat: DateTime<Utc>,
}

/// 通用 API 响应
#[derive(Debug, Serialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: Option<String>,
}
