//! MQTT 事件循环 — 完整的消息处理管线。
//!
//! 收到消息后依次经过：数据解析 → 缓存更新 → 规则评估 → 告警持久化 → AI 分析

use rumqttc::{Event, EventLoop, Incoming};
use crate::mqtt::MqttHandle;
use crate::engine::{filter, rules};
use chrono::Utc;

/// 运行 MQTT 事件循环
pub async fn run_event_loop(eventloop: &mut EventLoop, handle: MqttHandle) {
    tracing::info!("MQTT 事件循环已启动");

    loop {
        match eventloop.poll().await {
            Ok(Event::Incoming(Incoming::Publish(publish))) => {
                let topic = publish.topic.clone();
                let payload = String::from_utf8_lossy(&publish.payload).to_string();

                let device_id = filter::extract_device_id(&topic)
                    .unwrap_or_else(|| "unknown".into());
                let metric = filter::extract_metric(&topic)
                    .unwrap_or_else(|| "value".into());

                // 步骤 1：数据解析（优先使用设备映射的 JSON 路径）
                let value = {
                    let mapping = handle.devices.iter()
                        .find(|d| d.id == device_id)
                        .and_then(|d| d.mappings.iter().find(|m| m.metric == metric));

                    match mapping {
                        Some(m) => filter::parse_value_by_path(&payload, &m.json_path),
                        None => filter::parse_value(&payload),
                    }
                };

                let value = match value {
                    Some(v) => v,
                    None => {
                        tracing::debug!("无法解析载荷: topic={}", topic);
                        continue;
                    }
                };

                tracing::debug!(
                    "解析: device={}, metric={}, value={}",
                    device_id, metric, value
                );

                // 步骤 2：更新滑动窗口缓存
                let now = Utc::now();
                handle.cache.insert(&device_id, &metric, crate::engine::cache::DataPoint {
                    timestamp: now,
                    value,
                    raw_payload: Some(payload.clone()),
                });

                // 自动注册设备
                if handle.auto_register {
                    let _ = handle.db.register_device(&crate::storage::models::Device {
                        id: device_id.clone(),
                        name: device_id.clone(),
                        device_type: None,
                        registered_at: now,
                        last_seen: Some(now),
                        is_online: true,
                    }).await;
                }

                // 步骤 3：规则引擎评估
                let window = handle.cache.get_window(&device_id, &metric);
                let results = rules::evaluate(&handle.rules, &device_id, &metric, value, &window);

                for result in results {
                    if !result.triggered {
                        continue;
                    }

                    tracing::warn!(
                        "规则触发: {} ({}: value={}, severity={:?})",
                        result.rule_name, device_id, value, result.severity
                    );

                    // 步骤 4：写入告警
                    let mut ai_analysis = None;

                    // 步骤 5：AI 深度分析（如果配置了且已启用）
                    if result.should_ai_analyze {
                        if let Some(ref ai) = handle.ai {
                            if ai.is_enabled() {
                                let recent_values: Vec<(String, f64)> = window.iter()
                                    .map(|(t, v)| (t.format("%H:%M:%S").to_string(), *v))
                                    .collect();

                                let ctx = crate::ai::bridge::TelemetryContext {
                                    device_id: device_id.clone(),
                                    metric: metric.clone(),
                                    current_value: value,
                                    recent_values,
                                    rule_triggered: Some(result.rule_name.clone()),
                                };

                                match ai.analyze(&ctx).await {
                                    Ok(analysis) => {
                                        ai_analysis = Some(format!(
                                            "[{}] {} — 建议: {} (置信度: {:.0}%)",
                                            analysis.severity,
                                            analysis.summary,
                                            analysis.recommendation,
                                            analysis.confidence * 100.0
                                        ));
                                        tracing::info!("AI 分析完成: {}", ai_analysis.as_ref().unwrap());
                                    }
                                    Err(e) => {
                                        tracing::error!("AI 分析失败: {}", e);
                                    }
                                }
                            }
                        }
                    }

                    let severity_str = match result.severity {
                        rules::AlertSeverity::Info => "info",
                        rules::AlertSeverity::Warning => "warning",
                        rules::AlertSeverity::Critical => "critical",
                    };

                    let _ = handle.db.insert_alert(&crate::storage::models::Alert {
                        id: None,
                        rule_name: result.rule_name,
                        device_id: result.device_id,
                        metric: result.metric,
                        value: result.current_value,
                        severity: severity_str.into(),
                        message: result.message,
                        ai_analysis,
                        acknowledged: false,
                        resolved: false,
                        created_at: result.timestamp,
                        resolved_at: None,
                    }).await;
                }
            }
            Ok(Event::Incoming(Incoming::ConnAck(ack))) => {
                tracing::info!("MQTT 已连接: session_present={}", ack.session_present);
            }
            Ok(Event::Incoming(Incoming::Disconnect)) => {
                tracing::warn!("MQTT 连接断开，将自动重连");
            }
            Err(e) => {
                tracing::error!("MQTT 事件循环错误: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
            _ => {}
        }
    }
}
