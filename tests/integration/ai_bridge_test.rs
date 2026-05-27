//! AI Bridge 端到端测试：MQTT 触发 → AI 分析 → 告警含 AI 内容。
//! 需要 Ollama 运行 qwen-coder 模型。

use std::time::Duration;
use mqtt_mcp_server::config::{Config, MqttConfig, AiConfig, RuleConfig, StorageConfig};
use mqtt_mcp_server::storage;
use mqtt_mcp_server::mqtt;

#[tokio::test(flavor = "multi_thread")]
async fn test_ai_bridge_with_ollama() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let prefix = format!("test/ai-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
    let topic = format!("{}/pump/temperature", prefix);

    let rules = vec![RuleConfig {
        name: "高温告警".into(),
        device: "*".into(),
        metric: "temperature".into(),
        condition: "value > 80".into(),
        action: "alert".into(),
        ai_enhance: true,  // ★ 启用 AI 分析
    }];

    let config = Config {
        mqtt: MqttConfig {
            broker: "tcp://localhost:1883".into(),
            client_id: None, username: None, password: None,
            topics: vec![format!("{}/#", prefix)],
            qos: 1, keep_alive: 60, clean_session: true,
        },
        ai: AiConfig {
            enabled: true,
            provider: "openai".into(),
            api_key: Some("ollama".into()),
            model: "qwen-coder".into(),
            base_url: Some("http://localhost:11434/v1".into()),
            max_tokens: 200,
            window_size: 100,
        },
        rules: rules.clone(),
        devices: vec![],
        storage: StorageConfig {
            db_path: format!("data/ai-test-{}.db", uuid::Uuid::new_v4()),
            ..Default::default()
        },
    };

    let db = storage::init(&config).await.unwrap();

    // AI Bridge
    let ai = mqtt_mcp_server::ai::Bridge::new(&config.ai);
    assert!(ai.is_enabled());

    let handle = mqtt::start(&config.mqtt, db.clone(), Some(ai), rules, config.devices.clone(), 100, None)
        .await.unwrap();

    tokio::time::sleep(Duration::from_secs(2)).await;

    // 发布高温数据 — 应触发 AI 分析
    {
        let guard = handle.client.lock().await;
        let c = guard.as_ref().unwrap();
        c.publish(&topic, rumqttc::QoS::AtLeastOnce, false, b"96.0").await.unwrap();
        tracing::info!("发布高温 96°C，等待 AI 分析...");
    }

    // AI 调用需要时间
    tokio::time::sleep(Duration::from_secs(15)).await;

    let alerts = db.get_alerts(None, 10).await.unwrap();
    tracing::info!("告警总数: {}", alerts.len());

    for a in &alerts {
        tracing::info!("  [{}] {}: {}", a.severity, a.rule_name, a.message);
        if let Some(ref ai) = a.ai_analysis {
            tracing::info!("  AI 分析: {}", ai);
        }
    }

    // 验证
    assert!(!alerts.is_empty(), "应该有告警");
    let ai_alert = alerts.iter().find(|a| a.ai_analysis.is_some());
    assert!(ai_alert.is_some(), "应该有 AI 分析内容");
    tracing::info!("✅ AI Bridge 端到端测试通过");
}
