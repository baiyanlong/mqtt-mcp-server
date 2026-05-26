//! 边缘上报代理 — 心跳 + 告警推送到云服务
//!
//! Reporter 运行在边缘节点上（树莓派/香橙派），
//! 定期向 mqtt-mcp-cloud 发送心跳和系统状态，
//! 并在规则引擎触发告警时实时推送。

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// 上报代理 — 边缘到云的通信桥梁
#[derive(Clone)]
pub struct Reporter {
    /// 云服务地址（如 https://dashboard.mqtt-mcp.com）
    cloud_url: String,
    /// 节点认证 API Key
    api_key: String,
    /// 节点唯一标识（首次启动自动生成，持久化到 SQLite）
    node_id: String,
    /// 是否已向云端注册
    registered: Arc<Mutex<bool>>,
    /// HTTP 客户端（复用连接池）
    client: Client,
    /// 心跳间隔
    heartbeat_interval: Duration,
    /// 系统启动时间（用于统计 uptime）
    start_time: std::time::Instant,
}

/// 心跳上报的 JSON 结构
#[derive(Debug, Serialize)]
struct HeartbeatPayload {
    node_id: String,
    version: String,
    uptime_secs: u64,
    device_count: usize,
    alert_count: usize,
    mqtt_connected: bool,
}

/// 告警推送的 JSON 结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertPayload {
    pub node_id: String,
    pub device_id: String,
    pub rule_name: String,
    pub severity: String,
    pub message: String,
    pub value: f64,
    pub metric: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_analysis: Option<String>,
}

/// 云服务注册/心跳的响应结构
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CloudResponse {
    status: String,
    message: Option<String>,
}

impl Reporter {
    /// 创建上报代理实例
    pub fn new(cloud_url: String, api_key: String, node_id: String) -> Self {
        Self {
            cloud_url: cloud_url.trim_end_matches('/').to_string(),
            api_key,
            node_id,
            registered: Arc::new(Mutex::new(false)),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("创建 HTTP 客户端失败"),
            heartbeat_interval: Duration::from_secs(30),
            start_time: std::time::Instant::now(),
        }
    }

    /// 启动上报循环（非阻塞，spawn 后台任务）
    pub fn start(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            this.run().await;
        });
    }

    /// 主循环：先注册，再周期性心跳
    async fn run(&self) {
        // 首次注册
        match self.register().await {
            Ok(_) => {
                *self.registered.lock().await = true;
                tracing::info!("[reporter] 已注册到云服务: {}", self.cloud_url);
            }
            Err(e) => {
                tracing::warn!("[reporter] 注册失败（下次心跳重试）: {}", e);
            }
        }

        // 周期性心跳
        let mut ticker = interval(self.heartbeat_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = self.heartbeat().await {
                // 心跳连续失败 3 次 → 尝试重新注册
                tracing::warn!("[reporter] 心跳发送失败: {}", e);
                *self.registered.lock().await = false;
                // 重试注册
                if let Ok(_) = self.register().await {
                    *self.registered.lock().await = true;
                }
            }
        }
    }

    /// 向云服务注册节点
    async fn register(&self) -> Result<(), String> {
        let url = format!("{}/api/v1/nodes/register", self.cloud_url);
        let payload = serde_json::json!({
            "node_id": self.node_id,
            "version": env!("CARGO_PKG_VERSION"),
        });

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("注册失败: {} — {}", resp.status(), resp.text().await.unwrap_or_default()))
        }
    }

    /// 发送心跳（包含系统状态摘要）
    async fn heartbeat(&self) -> Result<(), String> {
        let url = format!("{}/api/v1/nodes/heartbeat", self.cloud_url);
        let payload = HeartbeatPayload {
            node_id: self.node_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            device_count: 0,   // 后续从 cache 获取
            alert_count: 0,    // 后续从 storage 获取
            mqtt_connected: true,
        };

        let resp = self
            .client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("心跳失败: {} — {}", resp.status(), resp.text().await.unwrap_or_default()))
        }
    }

    /// 推送告警到云服务（实时，不等待心跳周期）
    pub async fn push_alert(&self, alert: AlertPayload) {
        let url = format!("{}/api/v1/alerts", self.cloud_url);

        match self
            .client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&alert)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!("[reporter] 告警已推送: {}", alert.device_id);
            }
            Ok(resp) => {
                tracing::warn!("[reporter] 告警推送失败 {}: {}", resp.status(), alert.device_id);
            }
            Err(e) => {
                tracing::error!("[reporter] 告警推送网络错误: {}", e);
            }
        }
    }

    /// 返回节点 ID
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}
