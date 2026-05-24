//! MCP Server 端到端测试 — 直接调用处理器方法，不依赖 MCP 客户端。
//!
//! 运行方式：
//!   cargo test --test mcp_e2e -- --nocapture

use std::sync::Arc;
use tokio::sync::Mutex;
use mqtt_mcp_server::config::{Config, MqttConfig, AiConfig, StorageConfig};
use mqtt_mcp_server::storage;
use mqtt_mcp_server::ai;
use mqtt_mcp_server::mqtt;
use mqtt_mcp_server::mcp::server;

async fn setup() -> (server::MqttMcpServer, rumqttc::AsyncClient) {
    let mqtt_options = rumqttc::MqttOptions::new(
        format!("test-{}", uuid::Uuid::new_v4()),
        "localhost", 1883,
    );
    let (mqtt_client, eventloop) = rumqttc::AsyncClient::new(mqtt_options, 100);

    tokio::spawn(async move {
        let mut el = eventloop;
        loop {
            if el.poll().await.is_err() { break; }
        }
    });

    let config = Config {
        mqtt: MqttConfig {
            broker: "tcp://localhost:1883".into(),
            client_id: None, username: None, password: None,
            topics: vec!["#".into()], qos: 1, keep_alive: 60, clean_session: false,
        },
        ai: AiConfig::default(),
        rules: vec![],
        devices: vec![],
        storage: StorageConfig {
            db_path: format!("data/test-{}.db", uuid::Uuid::new_v4()),
            ..Default::default()
        },
    };

    let db = storage::init(&config).await.unwrap();
    let ai_bridge = ai::Bridge::new(&config.ai);

    let srv = server::MqttMcpServer {
        mqtt: mqtt::MqttHandle {
            client: Arc::new(Mutex::new(Some(mqtt_client.clone()))),
            config: config.mqtt.clone(),
            cache: Arc::new(mqtt_mcp_server::engine::cache::SlidingWindowCache::default()),
            db: db.clone(),
            ai: None,
            rules: Arc::new(vec![]),
            devices: Arc::new(vec![]),
            ai_window_size: 100,
            auto_register: false,
        },
        ai: ai_bridge,
        db,
        config,
    };

    (srv, mqtt_client)
}

#[tokio::test]
async fn test_list_devices_empty() {
    let (srv, _) = setup().await;
    let r = server::handle_list_devices(&srv).await.unwrap();
    assert!(r.contains("暂无") || r.contains("已注册"));
}

#[tokio::test]
async fn test_query_snapshot_no_data() {
    let (srv, _) = setup().await;
    let r = server::handle_query_snapshot(&srv, server::QuerySnapshotParams {
        device_id: "test".into(), metric: "temp".into(),
    }).await.unwrap();
    assert!(r.contains("暂无数据"));
}

#[tokio::test]
async fn test_get_alerts_empty() {
    let (srv, _) = setup().await;
    let r = server::handle_get_alerts(&srv, server::GetAlertsParams {
        severity: None, limit: Some(20),
    }).await.unwrap();
    assert!(r.contains("没有活跃告警"));
}

#[tokio::test]
async fn test_analyze_disabled() {
    let (srv, _) = setup().await;
    let r = server::handle_analyze(&srv, server::AnalyzeParams {
        device_id: "pump1".into(), window: None,
    }).await.unwrap();
    assert!(r.contains("未启用") || r.contains("API Key"));
}
