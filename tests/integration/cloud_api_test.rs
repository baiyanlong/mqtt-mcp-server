//! Cloud API 集成测试
//!
//! 需要 PostgreSQL。默认连接 postgres://mqttmcp:mqttmcp@localhost:5432/mqttmcp_test
//! 可通过 TEST_DATABASE_URL 环境变量覆盖。
//!
//! 运行： cargo test --test cloud_api --features cloud -- --nocapture

#[cfg(feature = "cloud")]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use mqtt_mcp_server::cloud;
    use serde_json::Value;
    use sqlx::PgPool;
    use tower::ServiceExt;

    async fn setup() -> (PgPool, String) {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://mqttmcp:mqttmcp@localhost:5432/mqttmcp_test".into());

        // 连接到 postgres 先创建测试库
        let admin_url = db_url.replace("mqttmcp_test", "postgres");
        if let Ok(admin_pool) = PgPool::connect(&admin_url).await {
            let _ = sqlx::query("DROP DATABASE IF EXISTS mqttmcp_test")
                .execute(&admin_pool)
                .await;
            let _ = sqlx::query("CREATE DATABASE mqttmcp_test")
                .execute(&admin_pool)
                .await;
            admin_pool.close().await;
        }

        let pool = PgPool::connect(&db_url).await.expect("连接测试数据库失败");
        cloud::db::init(&pool).await.expect("初始化表失败");
        cloud::ota_db::init_ota(&pool).await.expect("初始化 OTA 表失败");

        (pool, db_url)
    }

    fn app(pool: PgPool) -> axum::Router {
        cloud::build_router(pool)
    }

    // ═══════════════════════════════════════════
    // 基础
    // ═══════════════════════════════════════════

    #[sqlx::test]
    async fn test_health_check() {
        let (pool, _) = setup().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[sqlx::test]
    async fn test_dashboard_empty() {
        let (pool, _) = setup().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ═══════════════════════════════════════════
    // 节点管理
    // ═══════════════════════════════════════════

    #[sqlx::test]
    async fn test_register_and_list_nodes() {
        let (pool, _) = setup().await;

        // 注册节点
        let resp = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(
                        r#"{"node_id":"node-1","version":"0.3.0","name":"测试节点"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 列出节点
        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/nodes")
                    .header("Authorization", "test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let nodes: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(nodes.len() >= 1);
        assert_eq!(nodes[0]["node_id"], "node-1");
    }

    #[sqlx::test]
    async fn test_heartbeat() {
        let (pool, _) = setup().await;

        // 先注册
        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-hb","version":"0.3.0"}"#))
                    .unwrap(),
            )
            .await;

        // 发心跳
        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nodes/heartbeat")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(
                        r#"{"node_id":"node-hb","version":"0.3.0","uptime_secs":3600,"device_count":5,"alert_count":2,"mqtt_connected":true}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 1024).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    // ═══════════════════════════════════════════
    // 告警
    // ═══════════════════════════════════════════

    #[sqlx::test]
    async fn test_push_and_query_alerts() {
        let (pool, _) = setup().await;

        // 上报告警
        let resp = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/alerts")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(
                        r#"{"node_id":"node-1","device_id":"pump/3","rule_name":"高温","severity":"warning","message":"温度 88°C","value":88.0,"metric":"temperature","timestamp":"2026-05-27T10:00:00Z"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // 上报告警2
        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/alerts")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(
                        r#"{"node_id":"node-2","device_id":"pump/1","rule_name":"严重超温","severity":"critical","message":"温度 105°C","value":105.0,"metric":"temperature","timestamp":"2026-05-27T10:05:00Z"}"#,
                    ))
                    .unwrap(),
            )
            .await;

        // 查询所有告警
        let resp = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/alerts?limit=10")
                    .header("Authorization", "test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let alerts: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(alerts.len() >= 2);

        // 按严重程度过滤
        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/alerts?severity=critical&limit=10")
                    .header("Authorization", "test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let critical_alerts: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(critical_alerts.len(), 1);
        assert_eq!(critical_alerts[0]["severity"], "critical");
    }

    // ═══════════════════════════════════════════
    // 仪表盘
    // ═══════════════════════════════════════════

    #[sqlx::test]
    async fn test_dashboard_summary() {
        let (pool, _) = setup().await;

        // 注册节点 + 上报告警
        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"dash-1","version":"0.3.0"}"#))
                    .unwrap(),
            )
            .await;
        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/alerts")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(
                        r#"{"node_id":"dash-1","device_id":"p/1","rule_name":"t","severity":"critical","message":"x","value":99.0,"metric":"t","timestamp":"2026-05-27T10:00:00Z"}"#,
                    ))
                    .unwrap(),
            )
            .await;

        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/dashboard")
                    .header("Authorization", "test-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let summary: Value = serde_json::from_slice(&body).unwrap();
        assert!(summary["total_nodes"].as_i64().unwrap() >= 1);
        assert!(summary["online_nodes"].as_i64().unwrap() >= 1);
        assert!(summary["critical_alerts"].as_i64().unwrap() >= 1);
    }
}
