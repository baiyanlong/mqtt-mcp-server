// MQTT MCP Server — 让 AI Agent 通过 MCP 协议连接物理世界的 IoT 设备
//
// CLI 参数可覆盖配置文件的值，不写配置文件也能直接启动。

use clap::Parser;
use mqtt_mcp_server::{config, storage, mqtt, ai, mcp, reporter};
use tracing_subscriber::{EnvFilter, fmt};

/// MQTT MCP Server：将 AI 智能体通过 MQTT 连接到物理设备
///
/// 所有参数都可选——不提供则从 config.yaml 读取。
#[derive(Parser, Debug)]
#[command(name = "mqtt-mcp-server", version, about, long_about = None)]
struct Cli {
    // ── 配置文件 ──
    /// 配置文件路径（YAML）
    #[arg(short, long, default_value = "config.yaml")]
    config: String,

    // ── MCP 传输 ──
    /// MCP 传输模式：stdio（默认，适配 Claude Desktop）或 sse（HTTP）
    #[arg(long, default_value = "stdio")]
    mode: String,

    /// SSE 模式监听地址
    #[arg(long, default_value = "127.0.0.1:3000")]
    listen: String,

    /// 启用 Web Dashboard（默认端口 8080），--no-web 禁用
    #[arg(long, default_value_t = 8080, overrides_with = "no_web")]
    web: u16,

    /// 禁用 Web Dashboard（纯 MCP 模式）
    #[arg(long, default_value_t = false)]
    no_web: bool,

    // ── MQTT ──
    /// MQTT Broker 地址（覆盖配置文件）
    #[arg(long)]
    broker: Option<String>,

    /// MQTT 订阅主题（逗号分隔，覆盖配置文件）
    #[arg(long, value_delimiter = ',')]
    topics: Option<Vec<String>>,

    /// MQTT 用户名
    #[arg(long)]
    mqtt_user: Option<String>,

    /// MQTT 密码
    #[arg(long)]
    mqtt_pass: Option<String>,

    // ── AI ──
    /// 启用 AI 分析
    #[arg(long)]
    ai: bool,

    /// AI Provider：openai / anthropic / deepseek / qwen / ollama / custom
    #[arg(long)]
    ai_provider: Option<String>,

    /// AI 模型名称
    #[arg(long)]
    ai_model: Option<String>,

    /// AI API Key
    #[arg(long)]
    ai_key: Option<String>,

    /// AI API Base URL（Ollama 用 http://localhost:11434/v1）
    #[arg(long)]
    ai_base_url: Option<String>,

    /// AI 分析窗口大小（取最近 N 条数据）
    #[arg(long)]
    ai_window: Option<usize>,

    // ── 存储 ──
    /// SQLite 数据库路径
    #[arg(long)]
    db: Option<String>,

    // ── Pro 云服务 ──
    /// 云服务地址（如 https://dashboard.mqtt-mcp.com）
    #[arg(long)]
    cloud: Option<String>,

    /// 云服务 API Key
    #[arg(long)]
    cloud_key: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志（输出到 stderr，避免污染 stdio MCP 协议）
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();

    // 加载配置文件
    let mut cfg = config::load(&cli.config)?;

    // ── CLI 覆盖配置文件 ──
    if let Some(broker) = cli.broker {
        cfg.mqtt.broker = broker;
    }
    if let Some(topics) = cli.topics {
        cfg.mqtt.topics = topics;
    }
    if let Some(user) = cli.mqtt_user {
        cfg.mqtt.username = Some(user);
    }
    if let Some(pass) = cli.mqtt_pass {
        cfg.mqtt.password = Some(pass);
    }

    // AI 覆盖
    if cli.ai {
        cfg.ai.enabled = true;
    }
    if let Some(provider) = cli.ai_provider {
        // ollama → openai provider + localhost base_url
        if provider == "ollama" {
            cfg.ai.provider = "openai".into();
            cfg.ai.base_url.get_or_insert("http://localhost:11434/v1".into());
        } else {
            cfg.ai.provider = provider;
        }
    }
    if let Some(model) = cli.ai_model {
        cfg.ai.model = model;
    }
    if let Some(key) = cli.ai_key {
        cfg.ai.api_key = Some(key);
    }
    if let Some(url) = cli.ai_base_url {
        cfg.ai.base_url = Some(url);
    }
    if let Some(w) = cli.ai_window {
        cfg.ai.window_size = w;
    }

    // 存储覆盖
    if let Some(db_path) = cli.db {
        cfg.storage.db_path = db_path;
    }

    // ── 打印最终配置 ──
    tracing::info!("MQTT MCP Server v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("MQTT Broker: {}", cfg.mqtt.broker);
    tracing::info!("MQTT Topics: {:?}", cfg.mqtt.topics);
    tracing::info!("AI: {} (enabled={}, model={})", cfg.ai.provider, cfg.ai.enabled, cfg.ai.model);
    tracing::info!("传输模式: {}", cli.mode);

    // 初始化存储层
    let db = storage::init(&cfg).await?;

    // 初始化 AI Bridge
    let ai_bridge = ai::Bridge::new(&cfg.ai);

    // 初始化云服务上报代理（Pro 版可选）
    let cloud_reporter = match (&cli.cloud, &cli.cloud_key) {
        (Some(url), Some(key)) => {
            let storage_dir = std::path::PathBuf::from(
                std::path::Path::new(&cfg.storage.db_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
            );
            let r = reporter::Reporter::new(url.clone(), key.clone(), storage_dir);
            tracing::info!("[cloud] 已配置: {} (节点ID: {})", url, r.node_id());
            r.start();
            Some(r)
        }
        _ => None,
    };

    // 初始化 MQTT 客户端（传入 AI Bridge 和规则引擎）
    let mqtt_handle = mqtt::start(&cfg.mqtt, db.clone(), Some(ai_bridge.clone()), cfg.rules.clone(), cfg.devices.clone(), cfg.ai.window_size, cloud_reporter).await?;

    // 构建并启动 MCP 服务
    tracing::info!("启动 MCP 服务（{} 模式）...", cli.mode);

    match cli.mode.as_str() {
        // ── stdio 模式：桌面端 Agent 直连（Claude Desktop / Cursor） ──
        "stdio" => {
            // Web Dashboard 在后台并行运行（不阻塞 stdio）
            if !cli.no_web {
                let web_db = db.clone();
                let web_port = cli.web;
                tokio::spawn(async move {
                    if let Err(e) = mqtt_mcp_server::web::serve(web_db, web_port).await {
                        tracing::error!("Web Dashboard 错误: {}", e);
                    }
                });
                tracing::info!("Web Dashboard: http://localhost:{}", cli.web);
            }
            mcp::serve(mqtt_handle, ai_bridge, db.clone(), &cfg, cli.mode.as_str(), &cli.listen).await?;
        }

        // ── SSE 模式：远程 Agent 通过 HTTP SSE 连接 ──
        "sse" => {
            // 启动 SSE MCP Server（axum 在后台运行，不阻塞）
            let sse_ct = mcp::serve_sse(mqtt_handle, ai_bridge, db.clone(), &cfg, &cli.listen).await?;

            // Web Dashboard 并行运行
            if !cli.no_web {
                let web_db = db;
                let web_port = cli.web;
                tokio::spawn(async move {
                    if let Err(e) = mqtt_mcp_server::web::serve(web_db, web_port).await {
                        tracing::error!("Web Dashboard 错误: {}", e);
                    }
                });
                tracing::info!("Web Dashboard: http://localhost:{}", cli.web);
            }

            tracing::info!("═══════════════════════════════════════");
            tracing::info!("MQTT MCP Server 已就绪（SSE 模式）");
            tracing::info!("═══════════════════════════════════════");

            // 等待 Ctrl+C
            tokio::signal::ctrl_c().await?;
            sse_ct.cancel();
            tracing::info!("服务已关闭");
        }

        other => anyhow::bail!("不支持的模式: {}。可用: stdio, sse", other),
    }

    Ok(())
}
