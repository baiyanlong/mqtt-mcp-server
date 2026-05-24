//! MCP 协议端到端测试：启动 Server 进程 → MCP 客户端连接 → 调用 Tools。

use rmcp::{ServiceExt, transport::TokioChildProcess, model::CallToolRequestParam};
use tokio::process::Command;

#[tokio::test(flavor = "multi_thread")]
async fn test_mcp_client_list_and_call() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .ok();

    let mut cmd = Command::new(
        "/Users/byl/ai-project/mqtt-mcp-server/target/release/mqtt-mcp-server"
    );
    cmd.arg("--config")
       .arg("/Users/byl/ai-project/mqtt-mcp-server/test-config.yaml")
       .arg("--mode")
       .arg("stdio");

    tracing::info!("启动 MCP Server 进程...");
    let transport = TokioChildProcess::new(cmd).unwrap();
    let service = ().serve(transport).await.unwrap();

    let info = service.peer_info();
    tracing::info!("已连接！{:?}", info);

    // 列出 Tools
    let tools = service.list_tools(Default::default()).await.unwrap();
    tracing::info!("Tools: {} 个", tools.tools.len());
    for t in &tools.tools {
        tracing::info!("  - {}", t.name);
    }
    assert!(!tools.tools.is_empty());
    assert!(tools.tools.iter().any(|t| t.name == "mqtt_list_devices"));

    // 调用 mqtt_list_devices
    let result = service.call_tool(CallToolRequestParam {
        name: "mqtt_list_devices".into(),
        arguments: None,
    }).await.unwrap();
    tracing::info!("list_devices: {:?}", result);

    tracing::info!("✅ MCP 客户端端到端测试通过");
    service.cancel().await.unwrap();
}
