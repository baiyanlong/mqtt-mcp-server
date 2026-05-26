//! API Key 认证

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use sha2::{Sha256, Digest};

/// 对 API Key 做 SHA256 哈希（存储用，不存明文）
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// 提取 HTTP Header 中的 API Key
pub fn extract_key(req: &Request) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
}

/// axum 中间件：校验 API Key（简单实现——开发阶段直接比对）
///
/// 生产环境应查询数据库中的 key_hash。
pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 开发阶段：允许跳过认证（可通过环境变量开关）
    if std::env::var("CLOUD_DEV_MODE").is_ok() {
        return Ok(next.run(req).await);
    }

    let key = req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string());

    match key {
        Some(_k) => {
            // TODO: 查询 api_keys 表验证 key_hash
            // 开发阶段：有 key 就放行
            Ok(next.run(req).await)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
