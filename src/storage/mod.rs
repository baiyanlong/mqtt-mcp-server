//! 存储层：SQLite 数据库操作和设备/遥测/告警数据模型。
//!
//! 使用 bundled SQLite，无需外部数据库服务器。

pub mod models;
pub mod db;

pub use db::{init, Store};
