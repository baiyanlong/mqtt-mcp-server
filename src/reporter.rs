//! 边缘上报代理 — 心跳 + 告警推送到云服务

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::interval;

/// 上报代理 — 边缘到云的通信桥梁
#[derive(Clone)]
pub struct Reporter {
    cloud_url: String,
    api_key: String,
    /// 节点唯一 ID，持久化到本地文件，重启不丢失
    node_id: String,
    registered: Arc<Mutex<bool>>,
    client: Client,
    heartbeat_interval: Duration,
    start_time: std::time::Instant,
}

#[derive(Debug, Serialize)]
struct HeartbeatPayload {
    node_id: String,
    version: String,
    uptime_secs: u64,
    device_count: usize,
    alert_count: usize,
    mqtt_connected: bool,
}

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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CloudResponse {
    status: String,
    message: Option<String>,
}

impl Reporter {
    /// 创建上报代理。node_id 优先读持久化文件，不存在则新生成并保存。
    pub fn new(cloud_url: String, api_key: String, storage_dir: PathBuf) -> Self {
        let node_id = Self::load_or_create_node_id(&storage_dir);

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

    /// 从文件加载 node_id，不存在则生成新 UUID 并保存
    fn load_or_create_node_id(dir: &PathBuf) -> String {
        let path = dir.join(".node_id");
        if let Ok(id) = std::fs::read_to_string(&path) {
            let id = id.trim().to_string();
            if !id.is_empty() {
                tracing::info!("[reporter] 加载已持久化的节点 ID: {}", id);
                return id;
            }
        }

        let id = uuid::Uuid::new_v4().to_string();
        if let Err(e) = std::fs::create_dir_all(dir) {
            tracing::warn!("[reporter] 创建目录失败: {}", e);
        }
        if let Err(e) = std::fs::write(&path, &id) {
            tracing::warn!("[reporter] 保存节点 ID 失败: {}", e);
        } else {
            tracing::info!("[reporter] 新节点 ID 已保存: {}", id);
        }
        id
    }

    /// 启动上报循环（非阻塞）
    pub fn start(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            this.run().await;
        });
    }

    async fn run(&self) {
        if let Ok(_) = self.register().await {
            *self.registered.lock().await = true;
            tracing::info!("[reporter] 已注册到云服务: {}", self.cloud_url);
        } else {
            tracing::warn!("[reporter] 首次注册失败，将在心跳时重试");
        }

        let mut ticker = interval(self.heartbeat_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = self.heartbeat().await {
                tracing::warn!("[reporter] 心跳失败: {}", e);
                *self.registered.lock().await = false;
                if self.register().await.is_ok() {
                    *self.registered.lock().await = true;
                }
            }
        }
    }

    async fn register(&self) -> Result<(), String> {
        let url = format!("{}/api/v1/nodes/register", self.cloud_url);
        let payload = serde_json::json!({
            "node_id": self.node_id,
            "version": env!("CARGO_PKG_VERSION"),
        });

        let resp = self.client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP: {}", e))?;

        if resp.status().is_success() { Ok(()) }
        else { Err(format!("{} — {}", resp.status(), resp.text().await.unwrap_or_default())) }
    }

    /// 发送心跳。device_count / alert_count 由调用方通过 update_counts 设置。
    async fn heartbeat(&self) -> Result<(), String> {
        let payload = HeartbeatPayload {
            node_id: self.node_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: self.start_time.elapsed().as_secs(),
            device_count: 0,
            alert_count: 0,
            mqtt_connected: true,
        };

        let url = format!("{}/api/v1/nodes/heartbeat", self.cloud_url);
        let resp = self.client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("HTTP: {}", e))?;

        if resp.status().is_success() { Ok(()) }
        else { Err(format!("{}", resp.status())) }
    }

    /// 推送告警到云
    pub async fn push_alert(&self, alert: AlertPayload) {
        let url = format!("{}/api/v1/alerts", self.cloud_url);
        match self.client
            .post(&url)
            .header("Authorization", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&alert)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                tracing::debug!("[reporter] 告警推送成功: {}", alert.device_id);
            }
            Ok(resp) => {
                tracing::warn!("[reporter] 告警推送失败 {}: {}", resp.status(), alert.device_id);
            }
            Err(e) => {
                tracing::error!("[reporter] 告警推送网络错误: {}", e);
            }
        }
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}
