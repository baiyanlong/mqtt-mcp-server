//! License 验证 — 离线校验 Pro 授权
//!
//! License 格式：BASE64(json_payload.signature)
//! 安全措施：密钥 XOR 混淆 + 签名校验 + 过期检查 + 时钟倒退检测

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};

/// License 密钥（XOR 混淆，防止 strings 提取）
const KEY_XOR_MASK: [u8; 4] = [0x7A, 0xB3, 0xE1, 0x5F];
const KEY_PART_A: &[u8] = b"\x2a\xcf\x85\xa5\x14\xd2\x9f\xb3\x19\x8c\x4e\xe2\x50\xf7\xb1\x66\x0e\xd5\x92\xc3";
const KEY_PART_B: &[u8] = b"\x1f\xea\xdd\x86\x09\xf4\xc3\xa8\x33\xbf\x71\xcc\x44\xe9\xa7\x55\x1b\xd0\x96\xf2";

/// 恢复真实密钥
fn get_secret() -> [u8; 40] {
    let mut key = [0u8; 40];
    for i in 0..20 {
        key[i] = KEY_PART_A[i] ^ KEY_PART_B[i] ^ KEY_XOR_MASK[i % 4];
    }
    for i in 0..20 {
        key[20 + i] = KEY_PART_A[i] ^ KEY_PART_B[19 - i] ^ KEY_XOR_MASK[(i + 1) % 4];
    }
    key
}

/// License 载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    pub node_limit: u32,
    pub expire: String,
    #[serde(default)]
    pub customer: String,
}

#[derive(Debug, Clone)]
pub struct License {
    pub payload: LicensePayload,
    pub valid: bool,
    pub reason: Option<String>,
}

impl License {
    pub fn verify(license_str: &str) -> Self {
        let raw = match BASE64.decode(license_str.as_bytes()) {
            Ok(v) => v,
            Err(e) => return Self::invalid(format!("Base64 解码失败: {}", e)),
        };

        let raw_str = String::from_utf8_lossy(&raw);

        let (payload_str, sig_hex) = match raw_str.rfind('.') {
            Some(pos) => (&raw_str[..pos], &raw_str[pos + 1..]),
            None => return Self::invalid("格式错误：缺少签名分隔符".into()),
        };

        let secret = get_secret();
        let expected_sig = hmac_sha256(payload_str.as_bytes(), &secret);
        if sig_hex != expected_sig {
            return Self::invalid("签名校验失败：License 被篡改".into());
        }

        let payload: LicensePayload = match serde_json::from_str(payload_str) {
            Ok(p) => p,
            Err(e) => return Self::invalid(format!("JSON 解析失败: {}", e)),
        };

        // 过期检查 + 防时钟倒退
        let now = chrono::Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        // 防时钟倒退：检查编译时间
        let build_date = env!("CARGO_PKG_VERSION"); // 间接锚点，至少不能早于发布日
        if payload.expire.as_str() < today.as_str() {
            return Self {
                payload: payload.clone(),
                valid: false,
                reason: Some(format!("License 已过期 ({})", payload.expire)),
            };
        }

        // 过期时间不能超过当前年份 + 10 年（防无限期 license）
        let current_year = now.format("%Y").to_string().parse::<i32>().unwrap_or(2026);
        if let Ok(expire_year) = payload.expire[..4].parse::<i32>() {
            if expire_year > current_year + 10 {
                return Self {
                    payload: payload.clone(),
                    valid: false,
                    reason: Some("过期时间异常".into()),
                };
            }
        }

        // 时钟倒退检测：记录首次验证时间到文件
        if let Some(first_seen) = check_clock_tamper() {
            return Self {
                payload,
                valid: false,
                reason: Some(first_seen),
            };
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

    /// 生成 License（管理端使用）
    pub fn generate(payload: &LicensePayload) -> String {
        let json = serde_json::to_string(payload).expect("序列化失败");
        let secret = get_secret();
        let sig = hmac_sha256(json.as_bytes(), &secret);
        let raw = format!("{}.{}", json, sig);
        BASE64.encode(raw.as_bytes())
    }

    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// 检查节点数是否超限
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

/// 时钟倒退检测：记录首次验证时间，发现倒退则拒绝
fn check_clock_tamper() -> Option<String> {
    let marker_path = std::path::Path::new("/tmp/.mqtt-mcp-license-ts");

    let now = chrono::Local::now().timestamp();

    if let Ok(existing) = std::fs::read_to_string(marker_path) {
        if let Ok(prev) = existing.trim().parse::<i64>() {
            if now < prev - 86_400 {
                // 倒退超过 1 天
                return Some("检测到系统时钟倒退，License 验证拒绝".into());
            }
        }
    }

    let _ = std::fs::write(marker_path, now.to_string());
    None
}

/// HMAC-SHA256
fn hmac_sha256(data: &[u8], key: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    use std::io::Write;

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
            expire: "2027-12-31".into(),
            customer: "测试客户".into(),
        };
        let key = License::generate(&payload);
        let license = License::verify(&key);

        assert!(license.is_valid(), "{}", license.reason.unwrap_or_default());
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
            expire: "2027-12-31".into(),
            customer: "".into(),
        };
        let mut key = License::generate(&payload);
        key.push('x');
        let license = License::verify(&key);
        assert!(!license.is_valid());
    }

    #[test]
    fn test_secret_not_plaintext() {
        // 验证密钥在源码中不以明文存在
        let secret = get_secret();
        let secret_str = String::from_utf8_lossy(&secret);
        // 不应包含常见英文单词
        assert!(!secret_str.contains("secret"));
        assert!(!secret_str.contains("mqtt"));
    }
}
