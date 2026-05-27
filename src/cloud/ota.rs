//! OTA 远程升级 — Cloud 端

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// OTA 发布版本
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct OtaRelease {
    pub id: i64,
    pub version: String,
    pub platform: String,          // arm64 / x86_64
    pub file_path: String,         // 存储路径
    pub sha256: String,
    pub size_bytes: i64,
    pub release_notes: Option<String>,
    pub min_version: Option<String>, // 最低可升级版本
    pub created_at: DateTime<Utc>,
}

/// 版本检查请求
#[derive(Debug, Deserialize)]
pub struct OtaCheckParams {
    pub platform: String,          // 客户端平台
    pub current_version: String,   // 客户端当前版本
}

/// 版本检查响应
#[derive(Debug, Serialize)]
pub struct OtaCheckResponse {
    pub update_available: bool,
    pub latest_version: Option<String>,
    pub download_url: Option<String>,
    pub sha256: Option<String>,
    pub size_bytes: Option<i64>,
    pub release_notes: Option<String>,
}

/// 上传新版本请求
#[derive(Debug, Deserialize)]
pub struct OtaUploadRequest {
    pub version: String,
    pub platform: String,
    pub sha256: String,
    pub release_notes: Option<String>,
    pub min_version: Option<String>,
}
