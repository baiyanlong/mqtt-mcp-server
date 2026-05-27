//! OTA 数据库操作

use sqlx::PgPool;

use super::ota::*;

/// 初始化 OTA 表
pub async fn init_ota(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ota_releases (
            id BIGSERIAL PRIMARY KEY,
            version VARCHAR(50) NOT NULL,
            platform VARCHAR(20) NOT NULL,
            file_path VARCHAR(500) NOT NULL,
            sha256 VARCHAR(64) NOT NULL,
            size_bytes BIGINT NOT NULL DEFAULT 0,
            release_notes TEXT,
            min_version VARCHAR(50),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE(version, platform)
        );

        CREATE INDEX IF NOT EXISTS idx_ota_platform ON ota_releases(platform);
        CREATE INDEX IF NOT EXISTS idx_ota_version ON ota_releases(version);
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("[ota] OTA 表已初始化");
    Ok(())
}

/// 插入新发布版本
pub async fn insert_release(
    pool: &PgPool,
    version: &str,
    platform: &str,
    file_path: &str,
    sha256: &str,
    size_bytes: i64,
    release_notes: Option<&str>,
    min_version: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO ota_releases (version, platform, file_path, sha256, size_bytes, release_notes, min_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (version, platform) DO UPDATE SET
            file_path = EXCLUDED.file_path,
            sha256 = EXCLUDED.sha256,
            size_bytes = EXCLUDED.size_bytes,
            release_notes = COALESCE(EXCLUDED.release_notes, ota_releases.release_notes),
            min_version = COALESCE(EXCLUDED.min_version, ota_releases.min_version)
        "#,
    )
    .bind(version)
    .bind(platform)
    .bind(file_path)
    .bind(sha256)
    .bind(size_bytes)
    .bind(release_notes)
    .bind(min_version)
    .execute(pool)
    .await?;

    Ok(())
}

/// 获取指定平台的最新版本
pub async fn get_latest(pool: &PgPool, platform: &str) -> anyhow::Result<Option<OtaRelease>> {
    let release = sqlx::query_as::<_, OtaRelease>(
        "SELECT * FROM ota_releases WHERE platform = $1 ORDER BY created_at DESC LIMIT 1"
    )
    .bind(platform)
    .fetch_optional(pool)
    .await?;

    Ok(release)
}

/// 获取指定版本
pub async fn get_version(
    pool: &PgPool,
    version: &str,
    platform: &str,
) -> anyhow::Result<Option<OtaRelease>> {
    let release = sqlx::query_as::<_, OtaRelease>(
        "SELECT * FROM ota_releases WHERE version = $1 AND platform = $2"
    )
    .bind(version)
    .bind(platform)
    .fetch_optional(pool)
    .await?;

    Ok(release)
}
