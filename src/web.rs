//! 轻量 Web Dashboard — 零外部依赖，内嵌在二进制中。
//!
//! 提供三个 REST API + 一个 HTML 页面：
//!   GET /              → Dashboard 页面
//!   GET /api/devices   → 设备列表 JSON
//!   GET /api/alerts    → 告警列表 JSON
//!   GET /api/telemetry?device=X&metric=Y → 遥测数据 JSON

use crate::storage::Store;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// 启动 Web Dashboard，监听指定端口
pub async fn serve(db: Store, port: u16) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Web Dashboard 已启动: http://localhost:{}", port);

    let db = Arc::new(db);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let db = db.clone();

        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            let n = match socket.read(&mut buf).await {
                Ok(n) if n > 0 => n,
                _ => return,
            };

            let request = String::from_utf8_lossy(&buf[..n]);
            let first_line = request.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            if parts.len() < 2 {
                return;
            }

            let method = parts[0];
            let path = parts[1];

            let (status, content_type, body) = match (method, path) {
                ("GET", "/") => ("200 OK", "text/html; charset=utf-8", DASHBOARD_HTML.to_string()),
                ("GET", "/api/devices") => {
                    match db.list_devices().await {
                        Ok(devices) => ("200 OK", "application/json",
                            serde_json::to_string_pretty(&devices).unwrap_or_default()),
                        Err(e) => ("500", "text/plain", e.to_string()),
                    }
                }
                ("GET", "/api/alerts") => {
                    match db.get_alerts(None, 50).await {
                        Ok(alerts) => ("200 OK", "application/json",
                            serde_json::to_string_pretty(&alerts).unwrap_or_default()),
                        Err(e) => ("500", "text/plain", e.to_string()),
                    }
                }
                ("GET", p) if p.starts_with("/api/telemetry") => {
                    let query = p.split('?').nth(1).unwrap_or("");
                    let device = get_param(query, "device").unwrap_or("unknown");
                    let metric = get_param(query, "metric").unwrap_or("temperature");
                    match db.query_telemetry(device, metric, "1970-01-01T00:00:00Z", "2099-01-01T00:00:00Z", 100).await {
                        Ok(records) => ("200 OK", "application/json",
                            serde_json::to_string_pretty(&records).unwrap_or_default()),
                        Err(e) => ("500", "text/plain", e.to_string()),
                    }
                }
                _ => ("404", "text/plain", "Not Found".to_string()),
            };

            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status, content_type, body.len(), body
            );
            let _ = socket.write_all(response.as_bytes()).await;
        });
    }
}

fn get_param<'a>(query: &'a str, key: &str) -> Option<&'a str> {
    query.split('&')
        .find(|p| p.starts_with(&format!("{}=", key)))
        .and_then(|p| p.split('=').nth(1))
}

/// 内嵌 Dashboard HTML——自动刷新，轮询 API
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>MQTT MCP Server - Dashboard</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0d1117; color: #c9d1d9; padding: 20px; }
h1 { font-size: 20px; color: #58a6ff; margin-bottom: 20px; }
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 800px) { .grid { grid-template-columns: 1fr; } }
.card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; }
.card h2 { font-size: 14px; color: #8b949e; margin-bottom: 12px; text-transform: uppercase; letter-spacing: 1px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { padding: 6px 10px; text-align: left; border-bottom: 1px solid #21262d; }
th { color: #8b949e; font-weight: 500; }
.online { color: #3fb950; } .offline { color: #f85149; }
.sev-info { color: #58a6ff; } .sev-warning { color: #d29922; } .sev-critical { color: #f85149; }
.mono { font-family: 'SF Mono', monospace; font-size: 12px; }
.badge { display: inline-block; padding: 2px 8px; border-radius: 12px; font-size: 11px; font-weight: 600; }
.badge-info { background: #1f6feb33; color: #58a6ff; }
.badge-warn { background: #9e6a0333; color: #d29922; }
.badge-crit { background: #da363333; color: #f85149; }
.timestamp { color: #484f58; font-size: 11px; }
.ai-note { color: #8b949e; font-size: 12px; font-style: italic; margin-top: 4px; }
</style>
</head>
<body>
<h1>🔌 MQTT MCP Server Dashboard</h1>
<div class="grid">
    <div class="card">
        <h2>📡 设备列表</h2>
        <table><thead><tr><th>名称</th><th>ID</th><th>状态</th><th>最后活跃</th></tr></thead>
        <tbody id="devices"></tbody></table>
    </div>
    <div class="card">
        <h2>🚨 告警</h2>
        <table><thead><tr><th>级别</th><th>设备</th><th>数值</th><th>时间</th></tr></thead>
        <tbody id="alerts"></tbody></table>
    </div>
</div>
<script>
async function refresh() {
    try {
        let d = await fetch('/api/devices').then(r => r.json());
        document.getElementById('devices').innerHTML = d.map(dev =>
            `<tr><td>${dev.name}</td><td class="mono">${dev.id}</td>
            <td class="${dev.is_online ? 'online' : 'offline'}">${dev.is_online ? '在线' : '离线'}</td>
            <td class="timestamp">${dev.last_seen || '-'}</td></tr>`
        ).join('') || '<tr><td colspan="4">暂无设备</td></tr>';

        let a = await fetch('/api/alerts').then(r => r.json());
        document.getElementById('alerts').innerHTML = a.map(al => {
            let cls = al.severity === 'critical' ? 'crit' : al.severity === 'warning' ? 'warn' : 'info';
            return `<tr><td><span class="badge badge-${cls}">${al.severity}</span></td>
            <td class="mono">${al.device_id}</td><td>${al.value}</td>
            <td class="timestamp">${al.created_at?.substring(11,19) || ''}</td></tr>
            ${al.ai_analysis ? `<tr><td colspan="4" class="ai-note">🤖 ${al.ai_analysis.substring(0,200)}</td></tr>` : ''}`;
        }).join('') || '<tr><td colspan="4">无告警 ✅</td></tr>';
    } catch(e) { console.error(e); }
}
refresh();
setInterval(refresh, 3000);
</script>
</body>
</html>"#;
