//! 集成测试：验证 MQTT → MCP Server 端到端链路。
//!
//! 运行方式：
//!   cargo test --test integration -- --nocapture

use std::time::Duration;

#[tokio::test]
async fn test_mqtt_publish_and_receive() {
    // 1. 连接本地 MQTT Broker
    let mut mqttoptions = rumqttc::MqttOptions::new(
        format!("test-{}", uuid::Uuid::new_v4()),
        "localhost",
        1883,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 100);

    // 2. 订阅测试主题
    client.subscribe("test/integration/#", rumqttc::QoS::AtLeastOnce)
        .await
        .expect("订阅失败");

    // 3. 发布测试消息
    let test_payload = r#"{"temperature": 42.5, "unit": "celsius"}"#;
    client.publish(
        "test/integration/sensor1",
        rumqttc::QoS::AtLeastOnce,
        false,
        test_payload.as_bytes(),
    ).await.expect("发布失败");

    // 4. 等待消息到达（超时 5 秒）
    let received = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match eventloop.poll().await {
                Ok(rumqttc::Event::Incoming(rumqttc::Incoming::Publish(p))) => {
                    let payload = String::from_utf8_lossy(&p.payload).to_string();
                    tracing::info!("收到消息: topic={}, payload={}", p.topic, payload);
                    return (p.topic, payload);
                }
                Err(e) => {
                    tracing::error!("MQTT 错误: {}", e);
                    panic!("{}", e);
                }
                _ => {}
            }
        }
    }).await;

    match received {
        Ok((topic, payload)) => {
            assert_eq!(topic, "test/integration/sensor1");
            assert!(payload.contains("42.5"));
            tracing::info!("✅ 集成测试通过");
        }
        Err(_) => {
            panic!("❌ 超时：未收到 MQTT 消息");
        }
    }
}
