//! License 验证 — 离线校验 Pro 授权
//!
//! License 格式：BASE64(json_payload.signature)
//!   - payload: {"node_limit":20,"expire":"2027-06-01","customer":"xxx"}
//!   - signature: HMAC-SHA256(payload, SECRET)

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

/// License 密钥（编译时嵌入，不对外公开）
const LICENSE_SECRET: &[u8] = b"mqtt-mcp-pro-secret-2026-change-in-production";

/// License 载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// 节点数上限
    pub node_limit: u32,
    /// 过期日期 (YYYY-MM-DD)
    pub expire: String,
    /// 客户名称（可选）
    #[serde(default)]
    pub customer: String,
}

/// 解析后的 License
#[derive(Debug, Clone)]
pub struct License {
    pub payload: LicensePayload,
    pub valid: bool,
    pub reason: Option<String>,
}

impl License {
    /// 验证 License 字符串并解析
    pub fn verify(license_str: &str) -> Self {
        // Base64 解码
        let raw = match BASE64.decode(license_str.as_bytes()) {
            Ok(v) => v,
            Err(e) => return Self::invalid(format!("Base64 解码失败: {}", e)),
        };

        let raw_str = String::from_utf8_lossy(&raw);

        // 按最后一个 '.' 分割 payload 和 signature
        let (payload_str, sig_hex) = match raw_str.rfind('.') {
            Some(pos) => (&raw_str[..pos], &raw_str[pos + 1..]),
            None => return Self::invalid("格式错误：缺少签名分隔符".into()),
        };

        // 验证签名
        let expected_sig = hmac_sha256(payload_str.as_bytes(), LICENSE_SECRET);
        if sig_hex != expected_sig {
            return Self::invalid("签名校验失败：License 被篡改".into());
        }

        // 解析 JSON
        let payload: LicensePayload = match serde_json::from_str(payload_str) {
            Ok(p) => p,
            Err(e) => return Self::invalid(format!("JSON 解析失败: {}", e)),
        };

        // 检查过期
        if let Ok(today) = chrono::Local::now().format("%Y-%m-%d").to_string().as_str().parse::<String>() {
            // 简单字符串比较，YYYY-MM-DD 格式天然支持
            if payload.expire.as_str() < today.as_str() {
                return Self {
                    payload: payload.clone(),
                    valid: false,
                    reason: Some(format!("License 已过期 ({})", payload.expire)),
                };
            }
        }

        Self {
            payload,
            valid: true,
            reason: None,
        }
    }

    fn invalid(reason: String) -> Self {
        tracing::warn!("[license] {}", reason);
        Self {
            payload: LicensePayload {
                node_limit: 0,
                expire: String::new(),
                customer: String::new(),
            },
            valid: false,
            reason: Some(reason),
        }
    }

    /// 生成 License（仅供管理端使用）
    pub fn generate(payload: &LicensePayload) -> String {
        let json = serde_json::to_string(payload).expect("序列化失败");
        let sig = hmac_sha256(json.as_bytes(), LICENSE_SECRET);
        let raw = format!("{}.{}", json, sig);
        BASE64.encode(raw.as_bytes())
    }

    /// 是否在有效期内
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// 是否超过节点数限制
    pub fn check_node_count(&self, current_count: usize) -> Result<(), String> {
        if !self.valid {
            return Err("License 无效".into());
        }
        if current_count as u32 > self.payload.node_limit {
            return Err(format!(
                "超过节点数限制: {}/{}",
                current_count, self.payload.node_limit
            ));
        }
        Ok(())
    }
}

/// HMAC-SHA256 计算，返回 hex 字符串
fn hmac_sha256(data: &[u8], key: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    use std::io::Write;

    // 手动实现 HMAC-SHA256
    const BLOCK_SIZE: usize = 64;

    let mut key_padded = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let h = Sha256::digest(key);
        key_padded[..h.len()].copy_from_slice(&h);
    } else {
        key_padded[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= key_padded[i];
        opad[i] ^= key_padded[i];
    }

    let mut inner = Sha256::new();
    inner.write_all(&ipad).unwrap();
    inner.write_all(data).unwrap();
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.write_all(&opad).unwrap();
    outer.write_all(&inner_hash).unwrap();

    format!("{:x}", outer.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let payload = LicensePayload {
            node_limit: 20,
            expire: "2099-12-31".into(),
            customer: "测试客户".into(),
        };

        let key = License::generate(&payload);
        let license = License::verify(&key);

        assert!(license.is_valid());
        assert_eq!(license.payload.node_limit, 20);
        assert!(license.check_node_count(15).is_ok());
        assert!(license.check_node_count(25).is_err());
    }

    #[test]
    fn test_expired() {
        let payload = LicensePayload {
            node_limit: 20,
            expire: "2020-01-01".into(),
            customer: "".into(),
        };

        let key = License::generate(&payload);
        let license = License::verify(&key);

        assert!(!license.is_valid());
        assert!(license.reason.unwrap().contains("过期"));
    }

    #[test]
    fn test_tampered() {
        let payload = LicensePayload {
            node_limit: 20,
            expire: "2099-12-31".into(),
            customer: "".into(),
        };

        let mut key = License::generate(&payload);
        // 篡改 Base64 字符串
        key.push('x');

        let license = License::verify(&key);
        assert!(!license.is_valid());
    }
}
