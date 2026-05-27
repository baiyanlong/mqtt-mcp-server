use std::time::Duration;
use mqtt_mcp_server::config::{Config, MqttConfig, AiConfig, RuleConfig, StorageConfig};
use mqtt_mcp_server::storage;
use mqtt_mcp_server::mqtt;

#[tokio::test(flavor = "multi_thread")]
async fn test_rule_engine_triggers_alert() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let prefix = format!("test/pipe-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let topic = format!("{}/sensor/temperature", prefix);
    let db_path = format!("data/pipe-{}.db", uuid::Uuid::new_v4());

    let rules = vec![RuleConfig {
        name: "高温".into(),
        device: "*".into(),
        metric: "temperature".into(),
        condition: "value > 80".into(),
        action: "alert".into(),
        ai_enhance: false,
    }];

    let config = Config {
        mqtt: MqttConfig {
            broker: "tcp://localhost:1883".into(),
            client_id: None, username: None, password: None,
            topics: vec![format!("{}/#", prefix)],
            qos: 1, keep_alive: 60, clean_session: true,
        },
        ai: AiConfig::default(),
        rules: rules.clone(),
        devices: vec![],
        storage: StorageConfig { db_path, ..Default::default() },
    };

    let db = storage::init(&config).await.unwrap();
    let handle = mqtt::start(&config.mqtt, db.clone(), None, rules, vec![], 100, None)
        .await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    {
        let guard = handle.client.lock().await;
        let c = guard.as_ref().unwrap();
        c.publish(&topic, rumqttc::QoS::AtLeastOnce, false, b"25.0").await.unwrap();
        tokio::time::sleep(Duration::from_millis(500)).await;
        c.publish(&topic, rumqttc::QoS::AtLeastOnce, false, b"95.0").await.unwrap();
    }

    tokio::time::sleep(Duration::from_secs(3)).await;

    let device_id = topic.split('/').nth(1).unwrap_or("unknown");
    let window = handle.cache.get_window(device_id, "temperature");
    let alerts = db.get_alerts(None, 10).await.unwrap();

    tracing::info!("缓存: {} 条, 告警: {} 条", window.len(), alerts.len());

    assert!(window.len() >= 1, "至少 1 条缓存");
    assert!(!alerts.is_empty(), "应该有告警");
    assert!(alerts.iter().any(|a| a.value == 95.0));
    tracing::info!("✅ 管线测试通过");
}
