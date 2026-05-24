//! MQTT 客户端连接管理。
//!
//! 支持 TCP 连接，自动解析 tcp://host:port 格式地址。

use crate::config::MqttConfig;
use rumqttc::{AsyncClient, EventLoop, MqttOptions};

/// 根据配置创建 MQTT 客户端和事件循环
pub async fn connect(config: &MqttConfig) -> anyhow::Result<(AsyncClient, EventLoop)> {
    let client_id = config.client_id.clone().unwrap_or_else(|| {
        format!("mqtt-mcp-{}", uuid::Uuid::new_v4())
    });

    // 解析 broker 地址
    let (host, port) = parse_broker(&config.broker)?;
    let mut options = MqttOptions::new(&client_id, &host, port);
    options.set_keep_alive(std::time::Duration::from_secs(config.keep_alive));
    options.set_clean_session(config.clean_session);

    // 认证
    if let (Some(username), Some(password)) = (&config.username, &config.password) {
        options.set_credentials(username, password);
    }

    tracing::info!("连接 MQTT Broker {}:{}，客户端 ID: {}", host, port, client_id);

    let (client, eventloop) = AsyncClient::new(options, 100);

    // 订阅配置的主题
    for topic in &config.topics {
        client.subscribe(topic, rumqttc::QoS::AtLeastOnce).await?;
        tracing::info!("已订阅主题: {}", topic);
    }

    Ok((client, eventloop))
}

/// 解析 broker 字符串为 (host, port)
/// 支持格式：tcp://host:port、mqtt://host:port、host:port
fn parse_broker(broker: &str) -> anyhow::Result<(String, u16)> {
    let s = broker
        .trim_start_matches("tcp://")
        .trim_start_matches("mqtt://");

    if let Some((host, port_str)) = s.rsplit_once(':') {
        let port: u16 = port_str.parse()?;
        Ok((host.to_string(), port))
    } else {
        Ok((s.to_string(), 1883))
    }
}
