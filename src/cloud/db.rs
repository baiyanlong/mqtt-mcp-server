//! PostgreSQL 数据库操作

use sqlx::PgPool;

use super::models::*;

/// 初始化数据库：创建表结构
pub async fn init(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS nodes (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            node_id VARCHAR(255) UNIQUE NOT NULL,
            name VARCHAR(255) NOT NULL DEFAULT '',
            version VARCHAR(50) NOT NULL DEFAULT '',
            last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            device_count INT NOT NULL DEFAULT 0,
            alert_count INT NOT NULL DEFAULT 0,
            mqtt_connected BOOLEAN NOT NULL DEFAULT false,
            status VARCHAR(20) NOT NULL DEFAULT 'online',
            cpu_percent DOUBLE PRECISION,
            mem_mb DOUBLE PRECISION,
            uptime_secs BIGINT,
            labels JSONB,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS alerts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            node_id VARCHAR(255) NOT NULL,
            device_id VARCHAR(255) NOT NULL,
            rule_name VARCHAR(255) NOT NULL DEFAULT '',
            severity VARCHAR(20) NOT NULL DEFAULT 'info',
            message TEXT NOT NULL DEFAULT '',
            value DOUBLE PRECISION NOT NULL DEFAULT 0,
            metric VARCHAR(100) NOT NULL DEFAULT '',
            ai_analysis TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE INDEX IF NOT EXISTS idx_alerts_node_id ON alerts(node_id);
        CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at);

        CREATE TABLE IF NOT EXISTS api_keys (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            node_id VARCHAR(255) UNIQUE NOT NULL,
            key_hash VARCHAR(255) NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("[cloud] 数据库表已初始化");
    Ok(())
}

/// 注册或更新节点信息
pub async fn upsert_node(pool: &PgPool, req: &RegisterNodeRequest) -> anyhow::Result<Node> {
    let node = sqlx::query_as::<_, Node>(
        r#"
        INSERT INTO nodes (node_id, name, version, last_heartbeat, status)
        VALUES ($1, COALESCE($4, ''), $2, NOW(), 'online')
        ON CONFLICT (node_id)
        DO UPDATE SET
            version = EXCLUDED.version,
            last_heartbeat = NOW(),
            status = 'online',
            name = CASE WHEN nodes.name = '' THEN COALESCE(EXCLUDED.name, '') ELSE nodes.name END
        RETURNING *
        "#,
    )
    .bind(&req.node_id)
    .bind(&req.version)
    .bind(&req.name)
    .fetch_one(pool)
    .await?;

    Ok(node)
}

/// 更新心跳
pub async fn update_heartbeat(pool: &PgPool, req: &HeartbeatRequest) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE nodes
        SET last_heartbeat = NOW(),
            version = $2,
            device_count = $3,
            alert_count = $4,
            mqtt_connected = $5,
            status = 'online',
            uptime_secs = $6,
            cpu_percent = COALESCE($7, cpu_percent),
            mem_mb = COALESCE($8, mem_mb)
        WHERE node_id = $1
        "#,
    )
    .bind(&req.node_id)
    .bind(&req.version)
    .bind(req.device_count as i32)
    .bind(req.alert_count as i32)
    .bind(req.mqtt_connected)
    .bind(req.uptime_secs as i64)
    .bind(req.cpu_percent)
    .bind(req.mem_mb)
    .execute(pool)
    .await?;

    Ok(())
}

/// 获取所有节点
pub async fn list_nodes(pool: &PgPool) -> anyhow::Result<Vec<Node>> {
    let nodes = sqlx::query_as::<_, Node>(
        "SELECT * FROM nodes ORDER BY last_heartbeat DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(nodes)
}

/// 插入告警记录
pub async fn insert_alert(pool: &PgPool, alert: &AlertRecord) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO alerts (node_id, device_id, rule_name, severity, message, value, metric, ai_analysis)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&alert.node_id)
    .bind(&alert.device_id)
    .bind(&alert.rule_name)
    .bind(&alert.severity)
    .bind(&alert.message)
    .bind(alert.value)
    .bind(&alert.metric)
    .bind(&alert.ai_analysis)
    .execute(pool)
    .await?;

    Ok(())
}

/// 查询告警（按严重程度过滤，最近 N 条）
pub async fn list_alerts(pool: &PgPool, severity: Option<&str>, limit: i64) -> anyhow::Result<Vec<AlertRecord>> {
    let alerts = match severity {
        Some(sev) => {
            sqlx::query_as::<_, AlertRecord>(
                "SELECT * FROM alerts WHERE severity = $1 ORDER BY created_at DESC LIMIT $2"
            )
            .bind(sev)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, AlertRecord>(
                "SELECT * FROM alerts ORDER BY created_at DESC LIMIT $1"
            )
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(alerts)
}

/// 仪表盘摘要统计
pub async fn dashboard_summary(pool: &PgPool) -> anyhow::Result<DashboardSummary> {
    let total_nodes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes")
        .fetch_one(pool).await?;
    let online_nodes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM nodes WHERE status = 'online'")
        .fetch_one(pool).await?;
    let total_alerts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alerts")
        .fetch_one(pool).await?;
    let critical_alerts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM alerts WHERE severity = 'critical'")
        .fetch_one(pool).await?;

    let nodes = list_nodes(pool).await?;
    let node_summaries = nodes.into_iter().map(|n| NodeSummary {
        node_id: n.node_id,
        name: n.name,
        status: n.status,
        device_count: n.device_count,
        alert_count: n.alert_count,
        uptime_secs: n.uptime_secs,
        last_heartbeat: n.last_heartbeat,
    }).collect();

    Ok(DashboardSummary {
        total_nodes: total_nodes.0,
        online_nodes: online_nodes.0,
        total_alerts: total_alerts.0,
        critical_alerts: critical_alerts.0,
        nodes: node_summaries,
    })
}
