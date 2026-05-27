//! Cloud API 集成测试
//!
//! 需要 PostgreSQL。默认连接 TEST_DATABASE_URL 或 postgres://mqttmcp:mqttmcp@localhost:5432/mqttmcp_test
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

    async fn setup() -> PgPool {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://mqttmcp:mqttmcp@localhost:5432/mqttmcp_test?sslmode=disable".into());

        let pool = PgPool::connect(&db_url).await.expect("连接测试数据库失败");

        // 清空旧数据
        let _ = sqlx::query("DROP TABLE IF EXISTS alerts CASCADE").execute(&pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS nodes CASCADE").execute(&pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS api_keys CASCADE").execute(&pool).await;
        let _ = sqlx::query("DROP TABLE IF EXISTS ota_releases CASCADE").execute(&pool).await;

        cloud::db::init(&pool).await.expect("初始化表失败");

        pool
    }

    fn app(pool: PgPool) -> axum::Router {
        cloud::build_router(pool)
    }

    #[tokio::test]
    async fn test_health_check() {
        let pool = setup().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dashboard_empty() {
        let pool = setup().await;
        let response = app(pool)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_register_and_list_nodes() {
        let pool = setup().await;

        let resp = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-1","version":"0.3.0","name":"test"}"#))
                    .unwrap(),
            ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("GET").uri("/api/v1/nodes")
                    .header("Authorization", "test-key")
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let nodes: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(nodes.len() >= 1);
        assert_eq!(nodes[0]["node_id"], "node-1");
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let pool = setup().await;

        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-hb","version":"0.3.0"}"#))
                    .unwrap(),
            ).await;

        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/nodes/heartbeat")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-hb","version":"0.3.0","uptime_secs":3600,"device_count":5,"alert_count":2,"mqtt_connected":true}"#))
                    .unwrap(),
            ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_push_and_query_alerts() {
        let pool = setup().await;

        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/alerts")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-1","device_id":"pump/3","rule_name":"high","severity":"warning","message":"88C","value":88.0,"metric":"temperature","timestamp":"2026-05-27T10:00:00Z"}"#))
                    .unwrap(),
            ).await;

        let _ = app(pool.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/alerts")
                    .header("Content-Type", "application/json")
                    .header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"node-2","device_id":"pump/1","rule_name":"critical","severity":"critical","message":"105C","value":105.0,"metric":"temperature","timestamp":"2026-05-27T10:05:00Z"}"#))
                    .unwrap(),
            ).await;

        let resp = app(pool)
            .oneshot(
                Request::builder()
                    .method("GET").uri("/api/v1/alerts?limit=10")
                    .header("Authorization", "test-key")
                    .body(Body::empty()).unwrap(),
            ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let alerts: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(alerts.len() >= 2);

        // 按严重程度过滤 — 需要新请求
        let pool2 = setup().await;
        // 重新插入测试数据
        let _ = app(pool2.clone()).oneshot(
            Request::builder().method("POST").uri("/api/v1/alerts")
                .header("Content-Type", "application/json").header("Authorization", "test-key")
                .body(Body::from(r#"{"node_id":"n","device_id":"d","rule_name":"r","severity":"critical","message":"m","value":1.0,"metric":"m","timestamp":"2026-05-27T10:00:00Z"}"#)).unwrap(),
        ).await;

        let resp = app(pool2)
            .oneshot(
                Request::builder().method("GET").uri("/api/v1/alerts?severity=critical&limit=10")
                    .header("Authorization", "test-key").body(Body::empty()).unwrap(),
            ).await.unwrap();
        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let critical: Vec<Value> = serde_json::from_slice(&body).unwrap();
        assert!(critical.len() >= 1);
        assert_eq!(critical[0]["severity"], "critical");
    }

    #[tokio::test]
    async fn test_dashboard_summary() {
        let pool = setup().await;

        let _ = app(pool.clone())
            .oneshot(
                Request::builder().method("POST").uri("/api/v1/nodes/register")
                    .header("Content-Type", "application/json").header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"dash-1","version":"0.3.0"}"#)).unwrap(),
            ).await;
        let _ = app(pool.clone())
            .oneshot(
                Request::builder().method("POST").uri("/api/v1/alerts")
                    .header("Content-Type", "application/json").header("Authorization", "test-key")
                    .body(Body::from(r#"{"node_id":"dash-1","device_id":"p/1","rule_name":"t","severity":"critical","message":"x","value":99.0,"metric":"t","timestamp":"2026-05-27T10:00:00Z"}"#)).unwrap(),
            ).await;

        let resp = app(pool)
            .oneshot(
                Request::builder().method("GET").uri("/api/v1/dashboard")
                    .header("Authorization", "test-key").body(Body::empty()).unwrap(),
            ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), 10240).await.unwrap();
        let summary: Value = serde_json::from_slice(&body).unwrap();
        assert!(summary["total_nodes"].as_i64().unwrap() >= 1);
        assert!(summary["online_nodes"].as_i64().unwrap() >= 1);
        assert!(summary["critical_alerts"].as_i64().unwrap() >= 1);
    }
}
