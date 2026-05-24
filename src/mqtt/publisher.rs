//! MQTT 消息发布工具。
//!
//! 支持 QoS 0/1/2 三级服务质量。

use rumqttc::QoS;

/// 向 MQTT 主题发布消息
pub async fn publish(
    client: &rumqttc::AsyncClient,
    topic: &str,
    payload: &str,
    qos: u8,
) -> anyhow::Result<()> {
    let qos = match qos {
        0 => QoS::AtMostOnce,
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtLeastOnce,
    };

    client.publish(topic, qos, false, payload.as_bytes()).await?;
    tracing::debug!("已发布到 {} (qos={:?}): {}", topic, qos, payload);
    Ok(())
}
