//! MCP Server 生命周期 — 基于 rmcp 的完整 MCP 协议实现。
//!
//! stdio 模式：标准输入输出，适配 Claude Desktop 等本地客户端。
//! SSE 模式：HTTP Server-Sent Events，适配远程 Agent。

use crate::config::Config;
use crate::ai::Bridge;
use crate::storage::Store;
use super::tools;
use super::resources;
use super::prompts;
use std::sync::Arc;
use rmcp::{
    ServerHandler,
    model::{
        ServerInfo, ServerCapabilities,
        CallToolResult, Content,
        ListToolsResult, Tool as McpTool,
        ListResourcesResult,
        ListPromptsResult, Prompt as McpPrompt,
        RawResource, ErrorCode, AnnotateAble,
    },
    service::{ServiceExt, RequestContext, RoleServer},
    ErrorData as McpError,
    schemars,
};
use serde::Deserialize;

// ── 请求参数类型 ──

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct SubscribeParams {
    #[schemars(description = "要订阅的 MQTT 主题（支持通配符 + 和 #）")]
    pub topic: String,
    #[schemars(description = "QoS 级别：0/1/2")]
    #[serde(default = "default_qos")]
    pub qos: u8,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct PublishParams {
    #[schemars(description = "目标 MQTT 主题")]
    pub topic: String,
    #[schemars(description = "消息内容（建议 JSON 格式）")]
    pub payload: String,
    #[schemars(description = "QoS 级别")]
    #[serde(default = "default_qos")]
    pub qos: u8,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct QuerySnapshotParams {
    #[schemars(description = "设备唯一标识")]
    pub device_id: String,
    #[schemars(description = "指标名（如 temperature、humidity）")]
    pub metric: String,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct QueryRangeParams {
    #[schemars(description = "设备唯一标识")]
    pub device_id: String,
    #[schemars(description = "指标名")]
    pub metric: String,
    #[schemars(description = "起始时间（ISO 8601 或 '1h'/'30m'）")]
    pub from: String,
    #[schemars(description = "结束时间（默认当前）")]
    pub to: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct SendCommandParams {
    #[schemars(description = "目标设备标识")]
    pub device_id: String,
    #[schemars(description = "命令名（如 reboot、set_config）")]
    pub command: String,
    #[schemars(description = "命令参数（JSON 字符串）")]
    pub params: Option<String>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct GetAlertsParams {
    #[schemars(description = "过滤严重程度：critical/warning/info")]
    pub severity: Option<String>,
    #[schemars(description = "最大返回条数")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct AnalyzeParams {
    #[schemars(description = "要分析的设备")]
    pub device_id: String,
    #[schemars(description = "分析窗口：5m/1h/24h")]
    pub window: Option<String>,
}

fn default_qos() -> u8 { 1 }

// ── MCP Server 结构体 ──

/// MQTT MCP Server —— 实现 ServerHandler trait
#[derive(Clone)]
pub struct MqttMcpServer {
    pub mqtt: crate::mqtt::MqttHandle,
    pub ai: Bridge,
    pub db: Store,
    pub config: Config,
}

impl ServerHandler for MqttMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "MQTT MCP Server — 让 AI Agent 操控 MQTT 连接的 IoT 设备。\n\
                 支持订阅主题、查询遥测、发送控制指令、AI 异常分析。".into()
            ),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(build_tool_list())))
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, McpError>> + Send + '_ {
        let this = self.clone();
        async move {
            handle_tool_call(&this, &request).await
        }
    }

    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListResourcesResult::with_all_items(build_resource_list())))
    }

    fn list_prompts(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListPromptsResult, McpError>> + Send + '_ {
        std::future::ready(Ok(ListPromptsResult::with_all_items(build_prompt_list())))
    }
}

// ── Tool 列表构建 ──

fn build_tool_list() -> Vec<McpTool> {
    tools::all_tools().into_iter().map(|t| {
        let schema = build_input_schema(&t.parameters);
        McpTool::new(t.name, t.description, schema)
    }).collect()
}

fn build_input_schema(params: &[tools::ToolParam]) -> Arc<serde_json::Map<String, serde_json::Value>> {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for p in params {
        properties.insert(p.name.clone(), serde_json::json!({
            "type": p.param_type,
            "description": p.description,
        }));
        if p.required {
            required.push(p.name.clone());
        }
    }

    let schema: serde_json::Map<String, serde_json::Value> = [
        ("type".to_string(), serde_json::Value::String("object".to_string())),
        ("properties".to_string(), serde_json::Value::Object(properties)),
        ("required".to_string(), serde_json::Value::Array(
            required.into_iter().map(serde_json::Value::String).collect()
        )),
    ].into_iter().collect();

    Arc::new(schema)
}

// ── Resource 列表构建 ──

fn build_resource_list() -> Vec<rmcp::model::Resource> {
    resources::all_resources().into_iter().map(|r| {
        let mut res = RawResource::new(r.uri_pattern, r.name);
        res.description = Some(r.description);
        res.no_annotation()
    }).collect()
}

// ── Prompt 列表构建 ──

fn build_prompt_list() -> Vec<McpPrompt> {
    prompts::all_prompts().into_iter().map(|p| {
        McpPrompt::new(p.name, Some(p.description), None)
    }).collect()
}

// ── Tool 调用分发 ──

async fn handle_tool_call(
    server: &MqttMcpServer,
    request: &rmcp::model::CallToolRequestParam,
) -> Result<CallToolResult, McpError> {
    let result = match request.name.as_ref() {
        "mqtt_subscribe" => {
            let params = parse_params::<SubscribeParams>(&request.arguments)?;
            handle_subscribe(server, params).await
        }
        "mqtt_publish" => {
            let params = parse_params::<PublishParams>(&request.arguments)?;
            handle_publish(server, params).await
        }
        "mqtt_list_devices" => {
            handle_list_devices(server).await
        }
        "mqtt_query_snapshot" => {
            let params = parse_params::<QuerySnapshotParams>(&request.arguments)?;
            handle_query_snapshot(server, params).await
        }
        "mqtt_query_range" => {
            let params = parse_params::<QueryRangeParams>(&request.arguments)?;
            handle_query_range(server, params).await
        }
        "mqtt_send_command" => {
            let params = parse_params::<SendCommandParams>(&request.arguments)?;
            handle_send_command(server, params).await
        }
        "mqtt_get_alerts" => {
            let params = parse_params::<GetAlertsParams>(&request.arguments)?;
            handle_get_alerts(server, params).await
        }
        "mqtt_analyze" => {
            let params = parse_params::<AnalyzeParams>(&request.arguments)?;
            handle_analyze(server, params).await
        }
        name => Err(McpError::new(ErrorCode::METHOD_NOT_FOUND, format!("未知工具: {}", name), None)),
    }?;

    Ok(CallToolResult::success(vec![Content::text(result)]))
}

fn parse_params<T: for<'de> Deserialize<'de>>(
    args: &Option<serde_json::Map<String, serde_json::Value>>
) -> Result<T, McpError> {
    let value = match args {
        Some(map) => serde_json::Value::Object(map.clone()),
        None => serde_json::Value::Object(Default::default()),
    };
    serde_json::from_value(value)
        .map_err(|e| McpError::invalid_params(e.to_string(), None))
}

// ── Tool 处理函数实现 ──

/// 公开：供集成测试使用
pub async fn handle_subscribe(server: &MqttMcpServer, params: SubscribeParams) -> Result<String, McpError> {
    let client_guard = server.mqtt.client.lock().await;
    let client = client_guard.as_ref()
        .ok_or_else(|| McpError::internal_error("MQTT 客户端未连接", None))?;

    client.subscribe(&params.topic, rumqttc::QoS::AtLeastOnce).await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(format!("已订阅主题: {}", params.topic))
}

/// 公开：供集成测试使用
pub async fn handle_publish(server: &MqttMcpServer, params: PublishParams) -> Result<String, McpError> {
    let client_guard = server.mqtt.client.lock().await;
    let client = client_guard.as_ref()
        .ok_or_else(|| McpError::internal_error("MQTT 客户端未连接", None))?;

    let qos = match params.qos {
        0 => rumqttc::QoS::AtMostOnce,
        2 => rumqttc::QoS::ExactlyOnce,
        _ => rumqttc::QoS::AtLeastOnce,
    };

    client.publish(&params.topic, qos, false, params.payload.as_bytes()).await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(format!("已发布到 {}", params.topic))
}

/// 公开：供集成测试使用
pub async fn handle_list_devices(server: &MqttMcpServer) -> Result<String, McpError> {
    let devices = server.db.list_devices().await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if devices.is_empty() {
        return Ok("暂无已注册设备。设备会在收到 MQTT 消息后自动注册。".into());
    }

    let lines: Vec<String> = devices.iter().map(|d| {
        format!(
            "- {} ({}) — {}",
            d.name, d.id,
            if d.is_online { "在线" } else { "离线" }
        )
    }).collect();

    Ok(format!("已注册设备 ({}):\n{}", devices.len(), lines.join("\n")))
}

/// 公开：供集成测试使用
pub async fn handle_query_snapshot(server: &MqttMcpServer, params: QuerySnapshotParams) -> Result<String, McpError> {
    // 先从缓存查（数据在事件循环中实时写入缓存）
    let latest = server.mqtt.cache.get_latest(&params.device_id, &params.metric);

    match latest {
        Some(dp) => Ok(format!(
            "{} / {}: {} (时间: {})",
            params.device_id, params.metric, dp.value,
            dp.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        )),
        None => Ok(format!("{} / {}: 暂无数据", params.device_id, params.metric)),
    }
}

/// 公开：供集成测试使用
pub async fn handle_query_range(server: &MqttMcpServer, params: QueryRangeParams) -> Result<String, McpError> {
    // 从缓存获取窗口数据
    let window = server.mqtt.cache.get_window(&params.device_id, &params.metric);

    if window.is_empty() {
        return Ok(format!("{} / {}: 暂无数据", params.device_id, params.metric));
    }

    let lines: Vec<String> = window.iter().map(|(ts, v)| {
        format!("  {}: {}", ts.format("%H:%M:%S"), v)
    }).collect();

    Ok(format!(
        "{} / {} ({} 条记录):\n{}",
        params.device_id, params.metric, window.len(), lines.join("\n")
    ))
}

/// 公开：供集成测试使用
pub async fn handle_send_command(server: &MqttMcpServer, params: SendCommandParams) -> Result<String, McpError> {
    let topic = format!("devices/{}/command", params.device_id);
    let payload = params.params.unwrap_or_else(|| "{}".into());

    let client_guard = server.mqtt.client.lock().await;
    let client = client_guard.as_ref()
        .ok_or_else(|| McpError::internal_error("MQTT 客户端未连接", None))?;

    client.publish(&topic, rumqttc::QoS::AtLeastOnce, false, payload.as_bytes()).await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    Ok(format!("已向 {} 发送命令: {} (topic: {})", params.device_id, params.command, topic))
}

/// 公开：供集成测试使用
pub async fn handle_get_alerts(server: &MqttMcpServer, params: GetAlertsParams) -> Result<String, McpError> {
    let limit = params.limit.unwrap_or(20);
    let alerts = server.db.get_alerts(params.severity.as_deref(), limit).await
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

    if alerts.is_empty() {
        return Ok("没有活跃告警。".into());
    }

    let lines: Vec<String> = alerts.iter().map(|a| {
        format!(
            "  [{}] {} — {} ({}: {} @ {})",
            a.severity, a.rule_name, a.message,
            a.device_id, a.metric,
            a.created_at.format("%H:%M:%S"),
        )
    }).collect();

    Ok(format!("告警 ({}):\n{}", alerts.len(), lines.join("\n")))
}

/// 公开：供集成测试使用
pub async fn handle_analyze(server: &MqttMcpServer, params: AnalyzeParams) -> Result<String, McpError> {
    if !server.ai.is_enabled() {
        return Ok("AI 分析未启用。请配置 ai.enabled=true 并提供 API Key。".into());
    }

    // 从缓存读取近期数据（事件循环实时写入）
    let metric = "temperature"; // 默认分析温度，后续可从配置扩展
    let window = server.mqtt.cache.get_window(&params.device_id, metric);

    let recent_values: Vec<(String, f64)> = window.iter().map(|(ts, v)| {
        (ts.format("%H:%M:%S").to_string(), *v)
    }).collect();

    let current_value = recent_values.last().map(|(_, v)| *v).unwrap_or(0.0);

    let context = crate::ai::bridge::TelemetryContext {
        device_id: params.device_id.clone(),
        metric: metric.into(),
        current_value,
        recent_values,
        rule_triggered: None,
    };

    match server.ai.analyze(&context).await {
        Ok(analysis) => Ok(format!(
            "设备: {}\n异常: {}\n严重程度: {}\n置信度: {:.0}%\n摘要: {}\n建议: {}",
            params.device_id,
            if analysis.is_anomaly { "是" } else { "否" },
            analysis.severity,
            analysis.confidence * 100.0,
            analysis.summary,
            analysis.recommendation,
        )),
        Err(e) => Ok(format!("AI 分析失败: {}", e)),
    }
}

// ── 启动入口 ──

pub async fn serve(
    mqtt: crate::mqtt::MqttHandle,
    ai: Bridge,
    db: Store,
    config: &Config,
    mode: &str,
    listen_addr: &str,
) -> anyhow::Result<()> {
    let server = MqttMcpServer {
        mqtt,
        ai,
        db,
        config: config.clone(),
    };

    match mode {
        "stdio" => {
            tracing::info!("启动 MCP Server（stdio 模式）...");
            let service = server.serve(
                (tokio::io::stdin(), tokio::io::stdout())
            ).await?;
            tracing::info!("MCP Server 已就绪，等待客户端连接...");

            tokio::signal::ctrl_c().await?;
            service.cancel().await?;
        }
        "sse" => {
            tracing::info!("SSE 模式启动，监听: {}", listen_addr);
            tracing::warn!("SSE 模式尚未完整实现，请使用 stdio 模式");
            tokio::signal::ctrl_c().await?;
        }
        other => {
            anyhow::bail!("不支持的传输模式: {}。可用: stdio, sse", other)
        }
    }

    Ok(())
}
