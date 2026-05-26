//! mqtt-mcp-cloud — 多节点管理云服务
//!
//! 接收边缘节点心跳和告警上报，提供多节点 Dashboard。

pub mod auth;
pub mod db;
pub mod models;
pub mod server;

pub use server::build_router;
