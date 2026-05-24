//! MCP 协议层：Tools、Resources、Prompts 的定义与路由。
//!
//! 对外暴露 serve() 入口，MCP Server 启动时注册所有工具和资源。

pub mod server;
pub mod tools;
pub mod resources;
pub mod prompts;

pub use server::serve;
