//! OTA HTTP API 处理器 — Cloud 端

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use super::models::ApiResponse;
use super::ota::*;
use super::ota_db;

/// OTA 路由构建
pub fn ota_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/api/v1/ota/check", get(check_update))
        .route("/api/v1/ota/download/{version}/{platform}", get(download_binary))
        .with_state(pool)
}

/// GET /api/v1/ota/check?platform=arm64&current_version=0.3.0
async fn check_update(
    State(pool): State<PgPool>,
    Query(params): Query<OtaCheckParams>,
) -> Result<Json<OtaCheckResponse>, StatusCode> {
    let latest = ota_db::get_latest(&pool, &params.platform)
        .await
        .map_err(|e| {
            tracing::error!("[ota] 查询失败: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    match latest {
        Some(release) if release.version != params.current_version => {
            let allowed = match &release.min_version {
                Some(min_v) if params.current_version.as_str() < min_v.as_str() => true,
                Some(_) => false,
                None => true,
            };

            if allowed {
                Ok(Json(OtaCheckResponse {
                    update_available: true,
                    latest_version: Some(release.version.clone()),
                    download_url: Some(format!(
                        "/api/v1/ota/download/{}/{}",
                        release.version, release.platform
                    )),
                    sha256: Some(release.sha256),
                    size_bytes: Some(release.size_bytes),
                    release_notes: release.release_notes,
                }))
            } else {
                Ok(Json(OtaCheckResponse {
                    update_available: false,
                    latest_version: None,
                    download_url: None,
                    sha256: None,
                    size_bytes: None,
                    release_notes: Some("受 min_version 限制".into()),
                }))
            }
        }
        _ => Ok(Json(OtaCheckResponse {
            update_available: false,
            latest_version: None,
            download_url: None,
            sha256: None,
            size_bytes: None,
            release_notes: None,
        })),
    }
}

/// GET /api/v1/ota/download/{version}/{platform}
async fn download_binary(
    State(pool): State<PgPool>,
    Path((version, platform)): Path<(String, String)>,
) -> Result<axum::body::Body, StatusCode> {
    let release = ota_db::get_version(&pool, &version, &platform)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let data = tokio::fs::read(&release.file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(axum::body::Body::from(data))
}

/// 上传新版本元数据（管理员 API）
pub async fn upload_binary(
    State(pool): State<PgPool>,
    Json(req): Json<OtaUploadRequest>,
) -> Result<Json<ApiResponse>, StatusCode> {
    let dir = std::path::Path::new("ota_binaries");
    tokio::fs::create_dir_all(dir).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let file_path = dir
        .join(format!("mqtt-mcp-{}-{}", req.version, req.platform))
        .to_string_lossy()
        .to_string();

    ota_db::insert_release(
        &pool,
        &req.version,
        &req.platform,
        &file_path,
        &req.sha256,
        0,
        req.release_notes.as_deref(),
        req.min_version.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("[ota] 插入失败: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(ApiResponse {
        status: "ok".into(),
        message: Some(format!("v{} ({}) 已发布", req.version, req.platform)),
    }))
}
