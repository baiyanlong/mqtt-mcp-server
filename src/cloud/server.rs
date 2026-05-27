//! 云服务 HTTP API

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::*;
use super::ota::*;
use super::ota_db;

use super::db;
#[allow(unused_imports)]
use super::auth::auth_middleware;

/// OTA 路由（使用 CloudState）
pub fn ota_routes(state: CloudState) -> Router<CloudState> {
    use axum::routing::get;
    Router::new()
        .route("/api/v1/ota/check", get(ota_check_update))
        .route("/api/v1/ota/download/{version}/{platform}", get(ota_download))
        .with_state(state)
}

/// 共享应用状态
#[derive(Clone)]
pub struct CloudState {
    pub db: PgPool,
}

/// 构建 axum Router
pub fn build_router(pool: PgPool) -> Router {
    let state = CloudState { db: pool };

    // 无需认证的路由
    let public = Router::new()
        .route("/", get(serve_dashboard))
        .route("/health", get(health_check));

    // 需要 API Key 的路由
    let api = Router::new()
        .route("/api/v1/nodes/register", post(register_node))
        .route("/api/v1/nodes/heartbeat", post(node_heartbeat))
        .route("/api/v1/nodes", get(list_nodes))
        .route("/api/v1/alerts", post(push_alert).get(get_alerts))
        .route("/api/v1/dashboard", get(dashboard))
        .layer(axum::middleware::from_fn(super::auth::auth_middleware));

    Router::new()
        .merge(public)
        .merge(api)
        .merge(ota_routes(state.clone()))
        .with_state(state)
}

// ═══════════════════════════════════════════
// API Handlers
// ═══════════════════════════════════════════

/// POST /api/v1/nodes/register — 节点注册
async fn register_node(
    State(state): State<CloudState>,
    Json(req): Json<RegisterNodeRequest>,
) -> Result<Json<Node>, StatusCode> {
    db::upsert_node(&state.db, &req)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("注册节点失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// POST /api/v1/nodes/heartbeat — 心跳上报
async fn node_heartbeat(
    State(state): State<CloudState>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    db::update_heartbeat(&state.db, &req)
        .await
        .map_err(|e| {
            tracing::error!("心跳处理失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse { status: "ok".into(), message: None }))
}

/// GET /api/v1/nodes — 节点列表
async fn list_nodes(
    State(state): State<CloudState>,
) -> Result<Json<Vec<Node>>, StatusCode> {
    db::list_nodes(&state.db)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("查询节点失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// POST /api/v1/alerts — 边缘节点上报告警
async fn push_alert(
    State(state): State<CloudState>,
    Json(req): Json<crate::reporter::AlertPayload>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let alert = AlertRecord {
        id: Uuid::new_v4(),
        node_id: req.node_id,
        device_id: req.device_id,
        rule_name: req.rule_name,
        severity: req.severity,
        message: req.message,
        value: req.value,
        metric: req.metric,
        ai_analysis: req.ai_analysis,
        created_at: Utc::now(),
    };

    db::insert_alert(&state.db, &alert)
        .await
        .map_err(|e| {
            tracing::error!("告警存储失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(ApiResponse { status: "ok".into(), message: None }))
}

/// GET /api/v1/alerts?severity=warning&limit=50
async fn get_alerts(
    State(state): State<CloudState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<AlertRecord>>, StatusCode> {
    let severity = params.get("severity").map(|s| s.as_str());
    let limit: i64 = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);

    db::list_alerts(&state.db, severity, limit)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("查询告警失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// GET /api/v1/dashboard — 仪表盘摘要
async fn dashboard(
    State(state): State<CloudState>,
) -> Result<Json<DashboardSummary>, StatusCode> {
    db::dashboard_summary(&state.db)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!("查询仪表盘失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// GET /health — 健康检查
async fn health_check() -> Json<ApiResponse> {
    Json(ApiResponse { status: "ok".into(), message: Some("MQTT MCP Cloud is running".into()) })
}

/// GET / — 内嵌 HTML 多节点面板
async fn serve_dashboard() -> axum::response::Html<&'static str> {
    axum::response::Html(CLOUD_DASHBOARD_HTML)
}

/// GET /api/v1/ota/check?platform=arm64&current_version=0.3.0
async fn ota_check_update(
    State(state): State<CloudState>,
    axum::extract::Query(params): axum::extract::Query<OtaCheckParams>,
) -> Result<Json<OtaCheckResponse>, StatusCode> {
    let latest = ota_db::get_latest(&state.db, &params.platform)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match latest {
        Some(release) if release.version != params.current_version => {
            let allowed = match &release.min_version {
                Some(min_v) if params.current_version.as_str() < min_v.as_str() => true,
                _ => release.min_version.is_none(),
            };
            if allowed {
                Ok(Json(OtaCheckResponse {
                    update_available: true,
                    latest_version: Some(release.version.clone()),
                    download_url: Some(format!("/api/v1/ota/download/{}/{}", release.version, release.platform)),
                    sha256: Some(release.sha256),
                    size_bytes: Some(release.size_bytes),
                    release_notes: release.release_notes,
                }))
            } else {
                Ok(Json(OtaCheckResponse {
                    update_available: false, latest_version: None, download_url: None,
                    sha256: None, size_bytes: None,
                    release_notes: Some("受 min_version 限制".into()),
                }))
            }
        }
        _ => Ok(Json(OtaCheckResponse {
            update_available: false, latest_version: None, download_url: None,
            sha256: None, size_bytes: None, release_notes: None,
        })),
    }
}

/// GET /api/v1/ota/download/{version}/{platform}
async fn ota_download(
    State(state): State<CloudState>,
    Path((version, platform)): Path<(String, String)>,
) -> Result<axum::body::Body, StatusCode> {
    let release = ota_db::get_version(&state.db, &version, &platform)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let data = tokio::fs::read(&release.file_path).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(axum::body::Body::from(data))
}

/// 内嵌的多节点 Dashboard HTML
const CLOUD_DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>MQTT MCP Cloud - 多节点管理</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0d1117; color: #c9d1d9; padding: 20px; }
h1 { font-size: 20px; color: #58a6ff; margin-bottom: 8px; }
.subtitle { color: #8b949e; font-size: 13px; margin-bottom: 20px; }
.stats { display: flex; gap: 16px; margin-bottom: 24px; }
.stat { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px 24px; flex: 1; text-align: center; }
.stat .num { font-size: 28px; font-weight: 700; color: #58a6ff; }
.stat .label { font-size: 12px; color: #8b949e; margin-top: 4px; }
.critical .num { color: #f85149; }
.card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; margin-bottom: 16px; }
.card h2 { font-size: 14px; color: #8b949e; margin-bottom: 12px; text-transform: uppercase; letter-spacing: 1px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { padding: 8px 12px; text-align: left; border-bottom: 1px solid #21262d; }
th { color: #8b949e; font-weight: 500; }
.online { color: #3fb950; } .offline { color: #f85149; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 11px; font-weight: 600; }
.badge-info { background: #1f6feb33; color: #58a6ff; }
.badge-warn { background: #9e6a0333; color: #d29922; }
.badge-crit { background: #da363333; color: #f85149; }
.mono { font-family: 'SF Mono', monospace; font-size: 12px; }
.timestamp { color: #484f58; font-size: 11px; }
/* 节点卡片地图 */
.node-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 16px; margin-bottom: 24px; }
.node-card { background: #161b22; border: 1px solid #30363d; border-radius: 12px; padding: 20px; transition: border-color 0.2s, transform 0.2s; position: relative; }
.node-card:hover { border-color: #58a6ff; transform: translateY(-2px); }
.node-card.online { border-left: 3px solid #3fb950; }
.node-card.offline { border-left: 3px solid #f85149; opacity: 0.7; }
.node-card .nc-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
.node-card .nc-name { font-size: 16px; font-weight: 600; color: #e6edf3; word-break: break-all; }
.node-card .nc-badge { display: inline-block; padding: 2px 10px; border-radius: 12px; font-size: 11px; font-weight: 600; }
.node-card .nc-badge.online-badge { background: #3fb95022; color: #3fb950; }
.node-card .nc-badge.offline-badge { background: #f8514922; color: #f85149; }
.node-card .nc-stats { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.node-card .nc-stat { text-align: center; padding: 8px; background: #0d1117; border-radius: 6px; }
.node-card .nc-stat .nc-val { font-size: 20px; font-weight: 700; color: #58a6ff; }
.node-card .nc-stat .nc-label { font-size: 11px; color: #8b949e; margin-top: 2px; }
.node-card .nc-footer { margin-top: 14px; padding-top: 12px; border-top: 1px solid #21262d; display: flex; justify-content: space-between; font-size: 11px; color: #8b949e; }
.node-card .nc-version { font-family: 'SF Mono', monospace; }
.node-card .nc-alert-count { color: #f85149; font-weight: 600; }
</style>
</head>
<body>
<h1>MQTT MCP Cloud Dashboard</h1>
<p class="subtitle">多节点边缘网关管理</p>

<div class="stats" id="stats">
  <div class="stat"><div class="num" id="stat-nodes">0</div><div class="label">总节点</div></div>
  <div class="stat"><div class="num" id="stat-online">0</div><div class="label">在线</div></div>
  <div class="stat"><div class="num" id="stat-alerts">0</div><div class="label">总告警</div></div>
  <div class="stat critical"><div class="num" id="stat-critical">0</div><div class="label">严重告警</div></div>
</div>

<div class="card"><h2>节点地图</h2>
<div class="node-grid" id="nodes"></div></div>

<div class="card"><h2>最近告警</h2>
<table><thead><tr><th>节点</th><th>级别</th><th>设备</th><th>数值</th><th>时间</th></tr></thead>
<tbody id="alerts"></tbody></table></div>

<script>
const API = '/api/v1';
async function refresh() {
    try {
        let d = await fetch(API + '/dashboard').then(r => r.json());
        document.getElementById('stat-nodes').textContent = d.total_nodes;
        document.getElementById('stat-online').textContent = d.online_nodes;
        document.getElementById('stat-alerts').textContent = d.total_alerts;
        document.getElementById('stat-critical').textContent = d.critical_alerts;

        document.getElementById('nodes').innerHTML = d.nodes.map(n => {
            let uptime = n.uptime_secs ? Math.floor(n.uptime_secs / 3600) + 'h' : '?';
            let alertClass = n.alert_count > 0 ? ' nc-alert-count' : '';
            return `<div class="node-card ${n.status === 'online' ? 'online' : 'offline'}">
                <div class="nc-header">
                    <span class="nc-name">${n.name || n.node_id}</span>
                    <span class="nc-badge ${n.status === 'online' ? 'online-badge' : 'offline-badge'}">${n.status === 'online' ? '在线' : '离线'}</span>
                </div>
                <div class="nc-stats">
                    <div class="nc-stat"><div class="nc-val">${n.device_count}</div><div class="nc-label">设备数</div></div>
                    <div class="nc-stat"><div class="nc-val${alertClass}">${n.alert_count}</div><div class="nc-label">告警数</div></div>
                </div>
                <div class="nc-footer">
                    <span>uptime ${uptime}</span>
                    <span class="nc-version">${n.last_heartbeat?.substring(11,19) || '-'}</span>
                </div>
            </div>`;
        }).join('') || '<div style="color:#8b949e;text-align:center;padding:40px">暂无节点</div>';

        let a = await fetch(API + '/alerts?limit=20').then(r => r.json());
        document.getElementById('alerts').innerHTML = a.map(al => {
            let cls = al.severity === 'critical' ? 'crit' : al.severity === 'warning' ? 'warn' : 'info';
            return `<tr>
                <td class="mono">${al.node_id}</td>
                <td><span class="badge badge-${cls}">${al.severity}</span></td>
                <td>${al.device_id}</td>
                <td>${al.value}</td>
                <td class="timestamp">${al.created_at?.substring(11,19) || ''}</td>
            </tr>`;
        }).join('') || '<tr><td colspan="5">无告警</td></tr>';
    } catch(e) { console.error(e); }
}
refresh(); setInterval(refresh, 5000);
</script>
</body>
</html>"#;
