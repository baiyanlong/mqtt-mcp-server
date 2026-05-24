//! MQTT 接入层：rumqtt 封装、事件循环、消息发布。
//!
//! 对外暴露 MqttHandle 共享句柄和 start() 启动函数。

pub mod client;
pub mod subscriber;
pub mod publisher;

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::config::MqttConfig;
use crate::engine::cache::SlidingWindowCache;
use crate::storage::Store;
use crate::ai::Bridge;

/// MQTT 子系统句柄 — 跨所有 MCP 处理器共享
#[derive(Clone)]
pub struct MqttHandle {
    pub client: Arc<Mutex<Option<rumqttc::AsyncClient>>>,
    pub config: MqttConfig,
    /// 滑动窗口缓存，事件循环用
    pub cache: Arc<SlidingWindowCache>,
    /// 数据库句柄
    pub db: Store,
    /// AI Bridge（可选，用于深度分析）
    pub ai: Option<Bridge>,
    /// 规则配置
    pub rules: Arc<Vec<crate::config::RuleConfig>>,
    /// 设备注册回调：收到新设备时自动注册到数据库
    pub auto_register: bool,
}

/// 启动 MQTT 客户端，返回共享句柄
pub async fn start(
    config: &MqttConfig,
    db: Store,
    ai: Option<Bridge>,
    rules: Vec<crate::config::RuleConfig>,
) -> anyhow::Result<MqttHandle> {
    let handle = MqttHandle {
        client: Arc::new(Mutex::new(None)),
        config: config.clone(),
        cache: Arc::new(SlidingWindowCache::default()),
        db,
        ai,
        rules: Arc::new(rules),
        auto_register: true,
    };

    let (client, mut eventloop) = client::connect(config).await?;

    // 保存客户端引用
    *handle.client.lock().await = Some(client);

    // 启动事件循环（传入完整 handle）
    let h = handle.clone();
    tokio::spawn(async move {
        subscriber::run_event_loop(&mut eventloop, h).await;
    });

    Ok(handle)
}
