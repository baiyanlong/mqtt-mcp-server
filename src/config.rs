//! 配置层：加载 YAML 配置文件，支持环境变量替换。
//!
//! 配置包含四大部分：MQTT 连接、AI 模型、规则引擎、存储。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 顶层应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// MQTT Broker 连接配置
    pub mqtt: MqttConfig,
    /// AI Bridge 配置（LLM API Key、模型、Provider）
    #[serde(default)]
    pub ai: AiConfig,
    /// 规则引擎配置
    #[serde(default)]
    pub rules: Vec<RuleConfig>,
    /// 设备注册表
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
    /// 存储配置
    #[serde(default)]
    pub storage: StorageConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    /// Broker 地址（如 tcp://localhost:1883）
    pub broker: String,
    /// 客户端 ID（不填则自动生成）
    #[serde(default)]
    pub client_id: Option<String>,
    /// 认证用户名
    #[serde(default)]
    pub username: Option<String>,
    /// 认证密码
    #[serde(default)]
    pub password: Option<String>,
    /// 启动时订阅的主题列表
    #[serde(default)]
    pub topics: Vec<String>,
    /// 默认 QoS 级别
    #[serde(default = "default_qos")]
    pub qos: u8,
    /// 心跳间隔（秒）
    #[serde(default = "default_keep_alive")]
    pub keep_alive: u64,
    /// 清理会话标志
    #[serde(default = "default_clean_session")]
    pub clean_session: bool,
}

fn default_qos() -> u8 { 1 }
fn default_keep_alive() -> u64 { 60 }
fn default_clean_session() -> bool { false }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// 是否启用 AI 功能
    #[serde(default)]
    pub enabled: bool,
    /// LLM Provider：openai / anthropic / deepseek / qwen / zhipu / custom
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API Key（支持 ${ENV_VAR} 语法从环境变量读取）
    #[serde(default)]
    pub api_key: Option<String>,
    /// 模型名称
    #[serde(default = "default_model")]
    pub model: String,
    /// 自定义 Base URL（代理 / 自部署模型用）
    #[serde(default)]
    pub base_url: Option<String>,
    /// 每次请求最大 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_provider() -> String { "openai".into() }
fn default_model() -> String { "gpt-4o-mini".into() }
fn default_max_tokens() -> u32 { 1024 }

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_provider(),
            api_key: None,
            model: default_model(),
            base_url: None,
            max_tokens: default_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// 规则名称（用于告警标识）
    pub name: String,
    /// 设备匹配模式（如 "pump/*"、"*"）
    pub device: String,
    /// 监听的指标名
    pub metric: String,
    /// 触发条件表达式
    pub condition: String,
    /// 触发动作：alert / log / command
    #[serde(default = "default_action")]
    pub action: String,
    /// 是否启用 AI 增强分析
    #[serde(default)]
    pub ai_enhance: bool,
}

fn default_action() -> String { "alert".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    /// 设备唯一 ID
    pub id: String,
    /// 设备名称
    pub name: String,
    /// 设备类型标签
    #[serde(default)]
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// SQLite 数据库文件路径
    #[serde(default = "default_db_path")]
    pub db_path: String,
    /// 每设备最大遥测记录数（LRU 淘汰）
    #[serde(default = "default_max_records")]
    pub max_records_per_device: usize,
    /// 遥测数据保留天数
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_db_path() -> String { "data/mqtt-mcp.db".into() }
fn default_max_records() -> usize { 100_000 }
fn default_retention_days() -> u32 { 30 }

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            max_records_per_device: default_max_records(),
            retention_days: default_retention_days(),
        }
    }
}

/// 从 YAML 文件加载配置
pub fn load(path: &str) -> anyhow::Result<Config> {
    let path = Path::new(path);
    if !path.exists() {
        tracing::warn!("配置文件不存在 {}，使用默认配置", path.display());
        return Ok(Config::default());
    }

    let content = std::fs::read_to_string(path)?;
    let mut config: Config = serde_yaml::from_str(&content)?;

    // 解析环境变量引用（${VAR_NAME} 语法）
    let api_key_resolved = config.ai.api_key.as_ref().and_then(|key| {
        if key.starts_with("${") && key.ends_with("}") {
            let var_name = &key[2..key.len() - 1];
            let val = std::env::var(var_name).ok();
            if val.is_none() {
                tracing::warn!(
                    "环境变量 {} 未设置，AI 功能已禁用",
                    var_name
                );
            }
            val
        } else {
            Some(key.clone())
        }
    });
    config.ai.api_key = api_key_resolved;
    if config.ai.api_key.is_none() {
        config.ai.enabled = false;
    }

    Ok(config)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mqtt: MqttConfig {
                broker: "tcp://localhost:1883".into(),
                client_id: None,
                username: None,
                password: None,
                topics: vec!["#".into()],
                qos: default_qos(),
                keep_alive: default_keep_alive(),
                clean_session: default_clean_session(),
            },
            ai: AiConfig::default(),
            rules: vec![],
            devices: vec![],
            storage: StorageConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.mqtt.broker, "tcp://localhost:1883");
        assert!(!cfg.ai.enabled);
    }

    #[test]
    fn test_parse_yaml_config() {
        let yaml = r#"
mqtt:
  broker: tcp://broker.example.com:1883
  topics:
    - "sensors/#"
    - "actuators/#"
ai:
  enabled: true
  provider: openai
  api_key: sk-test123
  model: gpt-4o-mini
rules:
  - name: "高温告警"
    device: "pump/*"
    metric: "temperature"
    condition: "value > 85"
    action: "alert"
    ai_enhance: true
"#;
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.mqtt.broker, "tcp://broker.example.com:1883");
        assert_eq!(cfg.mqtt.topics.len(), 2);
        assert!(cfg.ai.enabled);
        assert_eq!(cfg.ai.provider, "openai");
        assert_eq!(cfg.rules.len(), 1);
    }
}
