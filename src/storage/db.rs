//! SQLite 数据库操作。
//!
//! 管理设备注册、遥测存储、告警记录和许可证。
//! 使用 bundled SQLite — 无需外部数据库服务器。

use crate::config::Config;
use chrono::Utc;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::models::{Alert, Device, TelemetryRecord};

/// 数据库句柄 — 跨所有处理器共享
#[derive(Clone)]
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

/// 初始化数据库，创建表结构
pub async fn init(config: &Config) -> anyhow::Result<Store> {
    let db_path = &config.storage.db_path;

    // 确保父目录存在
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    create_tables(&conn)?;

    let store = Store {
        conn: Arc::new(Mutex::new(conn)),
    };

    tracing::info!("数据库已初始化: {}", db_path);
    Ok(store)
}

/// 创建数据库表结构
fn create_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            device_type TEXT,
            registered_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_seen TEXT,
            is_online INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_id TEXT NOT NULL,
            metric TEXT NOT NULL,
            value REAL NOT NULL,
            raw_payload TEXT,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (device_id) REFERENCES devices(id)
        );

        CREATE INDEX IF NOT EXISTS idx_telemetry_device_metric
            ON telemetry(device_id, metric, timestamp);

        CREATE TABLE IF NOT EXISTS alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_name TEXT NOT NULL,
            device_id TEXT NOT NULL,
            metric TEXT NOT NULL,
            value REAL NOT NULL,
            severity TEXT NOT NULL DEFAULT 'warning',
            message TEXT NOT NULL,
            ai_analysis TEXT,
            acknowledged INTEGER NOT NULL DEFAULT 0,
            resolved INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            resolved_at TEXT,
            FOREIGN KEY (device_id) REFERENCES devices(id)
        );

        CREATE INDEX IF NOT EXISTS idx_alerts_device_time
            ON alerts(device_id, created_at);

        CREATE TABLE IF NOT EXISTS licenses (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            license_key TEXT NOT NULL UNIQUE,
            tier TEXT NOT NULL,
            node_limit INTEGER NOT NULL DEFAULT 1,
            activated_at TEXT NOT NULL DEFAULT (datetime('now')),
            expires_at TEXT,
            is_active INTEGER NOT NULL DEFAULT 1
        );
        ",
    )?;

    Ok(())
}

impl Store {
    // ── 设备操作 ──

    /// 注册或更新设备
    pub async fn register_device(&self, device: &Device) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO devices (id, name, device_type, registered_at, last_seen, is_online)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                device.id,
                device.name,
                device.device_type,
                device.registered_at.to_rfc3339(),
                device.last_seen.map(|t| t.to_rfc3339()),
                device.is_online as i32,
            ],
        )?;
        Ok(())
    }

    /// 列出所有设备
    pub async fn list_devices(&self) -> anyhow::Result<Vec<Device>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, name, device_type, registered_at, last_seen, is_online FROM devices"
        )?;

        let devices = stmt.query_map([], |row| {
            Ok(Device {
                id: row.get(0)?,
                name: row.get(1)?,
                device_type: row.get(2)?,
                registered_at: parse_datetime(&row.get::<_, String>(3)?),
                last_seen: row.get::<_, Option<String>>(4)?.map(|s| parse_datetime(&s)),
                is_online: row.get::<_, i32>(5)? != 0,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(devices)
    }

    /// 更新设备在线状态
    pub async fn update_device_status(&self, device_id: &str, is_online: bool) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE devices SET is_online = ?1, last_seen = datetime('now') WHERE id = ?2",
            rusqlite::params![is_online as i32, device_id],
        )?;
        Ok(())
    }

    // ── 遥测操作 ──

    /// 插入遥测记录
    pub async fn insert_telemetry(&self, record: &TelemetryRecord) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO telemetry (device_id, metric, value, raw_payload, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                record.device_id,
                record.metric,
                record.value,
                record.raw_payload,
                record.timestamp.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// 查询历史遥测数据
    pub async fn query_telemetry(
        &self,
        device_id: &str,
        metric: &str,
        from: &str,
        to: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<TelemetryRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT device_id, metric, value, raw_payload, timestamp
             FROM telemetry
             WHERE device_id = ?1 AND metric = ?2 AND timestamp >= ?3 AND timestamp <= ?4
             ORDER BY timestamp DESC
             LIMIT ?5"
        )?;

        let records = stmt.query_map(
            rusqlite::params![device_id, metric, from, to, limit as i64],
            |row| {
                Ok(TelemetryRecord {
                    id: None,
                    device_id: row.get(0)?,
                    metric: row.get(1)?,
                    value: row.get(2)?,
                    raw_payload: row.get(3)?,
                    timestamp: parse_datetime(&row.get::<_, String>(4)?),
                })
            },
        )?.filter_map(|r| r.ok()).collect();

        Ok(records)
    }

    // ── 告警操作 ──

    /// 插入告警记录
    pub async fn insert_alert(&self, alert: &Alert) -> anyhow::Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO alerts (rule_name, device_id, metric, value, severity, message, ai_analysis)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                alert.rule_name,
                alert.device_id,
                alert.metric,
                alert.value,
                alert.severity,
                alert.message,
                alert.ai_analysis,
            ],
        )?;
        Ok(())
    }

    /// 获取告警列表（可按严重程度过滤）
    pub async fn get_alerts(
        &self,
        severity: Option<&str>,
        limit: usize,
    ) -> anyhow::Result<Vec<Alert>> {
        let conn = self.conn.lock().await;

        let query = if let Some(sev) = severity {
            format!(
                "SELECT id, rule_name, device_id, metric, value, severity, message, ai_analysis,
                        acknowledged, resolved, created_at, resolved_at
                 FROM alerts WHERE severity = '{}' AND resolved = 0
                 ORDER BY created_at DESC LIMIT {}",
                sev, limit
            )
        } else {
            format!(
                "SELECT id, rule_name, device_id, metric, value, severity, message, ai_analysis,
                        acknowledged, resolved, created_at, resolved_at
                 FROM alerts WHERE resolved = 0
                 ORDER BY created_at DESC LIMIT {}",
                limit
            )
        };

        let mut stmt = conn.prepare(&query)?;
        let alerts = stmt.query_map([], |row| {
            Ok(Alert {
                id: Some(row.get(0)?),
                rule_name: row.get(1)?,
                device_id: row.get(2)?,
                metric: row.get(3)?,
                value: row.get(4)?,
                severity: row.get(5)?,
                message: row.get(6)?,
                ai_analysis: row.get(7)?,
                acknowledged: row.get::<_, i32>(8)? != 0,
                resolved: row.get::<_, i32>(9)? != 0,
                created_at: parse_datetime(&row.get::<_, String>(10)?),
                resolved_at: row.get::<_, Option<String>>(11)?.map(|s| parse_datetime(&s)),
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(alerts)
    }
}

/// 将 RFC3339 字符串解析为 DateTime<Utc>
fn parse_datetime(s: &str) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
