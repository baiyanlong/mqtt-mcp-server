//! mqtt-mcp-cloud — 多节点管理云服务
//!
//! 启动方式：
//!   mqtt-mcp-cloud --listen 0.0.0.0:8080 --db postgres://user:pass@localhost/mqttmcp
//!
//! 数据库密码支持环境变量：--db "postgres://user:${DB_PASSWORD}@localhost/mqttmcp"

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

/// MQTT MCP Cloud — 多节点边缘网关管理平台
#[derive(Parser, Debug)]
#[command(name = "mqtt-mcp-cloud", version, about)]
struct Cli {
    /// 监听地址
    #[arg(long, default_value = "0.0.0.0:8080")]
    listen: String,

    /// PostgreSQL 连接串（也可通过 DATABASE_URL 环境变量设置）
    #[arg(long)]
    db: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    tracing::info!("MQTT MCP Cloud v{}", env!("CARGO_PKG_VERSION"));

    // 隐藏密码打印
    let db_display = if let Some(at) = cli.db.find('@') {
        if let Some(colon) = cli.db[..at].rfind(':') {
            format!("{}:***{}", &cli.db[..colon], &cli.db[at..])
        } else {
            cli.db.clone()
        }
    } else {
        cli.db.clone()
    };
    tracing::info!("数据库: {}", db_display);

    // 连接 PostgreSQL
    let pool = sqlx::PgPool::connect(&cli.db).await?;
    tracing::info!("PostgreSQL 已连接");

    // 初始化表结构
    mqtt_mcp_server::cloud::db::init(&pool).await?;

    // 构建路由
    let router = mqtt_mcp_server::cloud::build_router(pool);

    // 启动 HTTP 服务（优雅关闭）
    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    tracing::info!("═══════════════════════════════════════");
    tracing::info!("Cloud Dashboard:  http://{}", cli.listen);
    tracing::info!("API:             http://{}/api/v1/", cli.listen);
    tracing::info!("健康检查:        http://{}/health", cli.listen);
    tracing::info!("═══════════════════════════════════════");

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("收到关闭信号，正在退出...");
        })
        .await?;

    Ok(())
}
