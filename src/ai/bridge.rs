//! AI Bridge — 将 MQTT 数据接入 LLM API 进行智能分析。
//!
//! 支持多 Provider，客户自备 API Key，我们不承担 token 费用。
//! 支持的 Provider：openai / anthropic / deepseek / qwen / zhipu / custom

use crate::config::AiConfig;
use serde::{Deserialize, Serialize};

/// AI Bridge 客户端
#[derive(Clone)]
pub struct Bridge {
    config: AiConfig,
    client: reqwest::Client,
}

/// LLM 返回的分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// 是否为异常
    pub is_anomaly: bool,
    /// 严重程度：normal / warning / critical
    pub severity: String,
    /// 摘要
    pub summary: String,
    /// 建议
    pub recommendation: String,
    /// 置信度 0.0 - 1.0
    pub confidence: f64,
}

/// 发送给 LLM 的遥测上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryContext {
    pub device_id: String,
    pub metric: String,
    pub current_value: f64,
    pub recent_values: Vec<(String, f64)>,  // (时间戳, 值)
    pub rule_triggered: Option<String>,
}

impl Bridge {
    pub fn new(config: &AiConfig) -> Self {
        Self {
            config: config.clone(),
            client: reqwest::Client::builder()
                .no_proxy()  // 不走系统代理（Ollama 等本地模型用）
                .build()
                .expect("创建 HTTP 客户端失败"),
        }
    }

    /// 检查 AI 功能是否已启用
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.config.api_key.is_some()
    }

    /// 分析设备遥测数据，返回 AI 评估结果
    pub async fn analyze(&self, context: &TelemetryContext) -> anyhow::Result<AnalysisResult> {
        if !self.is_enabled() {
            anyhow::bail!("AI Bridge 未启用。请设置 ai.enabled=true 并提供 API Key。");
        }

        let prompt = build_analysis_prompt(context);
        let response = self.call_llm(&prompt).await?;
        let result = parse_analysis_response(&response)?;

        Ok(result)
    }

    /// 调用 LLM API
    async fn call_llm(&self, prompt: &str) -> anyhow::Result<String> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("未配置 API Key"))?;

        let base_url = self.config.base_url.as_deref()
            .unwrap_or_else(|| default_base_url(&self.config.provider));

        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {
                    "role": "system",
                    "content": SYSTEM_PROMPT
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": self.config.max_tokens,
            "temperature": 0.3,
            "response_format": { "type": "json_object" }
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await.unwrap_or_default();
            anyhow::bail!("LLM API 错误 ({}): {}", status, error_body);
        }

        let json: serde_json::Value = response.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("{}")
            .to_string();

        Ok(content)
    }
}

/// 系统提示词 — 指示 LLM 返回结构化 JSON
const SYSTEM_PROMPT: &str = r#"你是一个工业 IoT 分析助手。
分析设备遥测数据并检测异常。

请以 JSON 格式返回分析结果，包含以下字段：
{
  "is_anomaly": true/false,
  "severity": "normal" | "warning" | "critical",
  "summary": "一句话摘要",
  "recommendation": "给操作员的行动建议",
  "confidence": 0.0 到 1.0
}

规则：
- 轻微超范围是 "warning"，除非是持续趋势
- 突发的孤立尖峰是 "warning"
- 持续偏离或朝危险方向快速变化是 "critical"
- 一切正常时 severity 为 "normal"，置信度应该高
- 保持简洁。操作员在值班，需要快速可执行的信息。"#;

/// 根据遥测上下文构建用户提示词
fn build_analysis_prompt(ctx: &TelemetryContext) -> String {
    let mut prompt = format!(
        "分析以下遥测数据：\n\n设备: {}\n指标: {}\n当前值: {}\n\n",
        ctx.device_id, ctx.metric, ctx.current_value
    );

    if !ctx.recent_values.is_empty() {
        prompt.push_str("近期数据（时间, 值）：\n");
        for (ts, val) in &ctx.recent_values {
            prompt.push_str(&format!("  {}: {}\n", ts, val));
        }
        prompt.push('\n');
    }

    if let Some(ref rule) = ctx.rule_triggered {
        prompt.push_str(&format!(
            "注意：本地规则引擎已触发规则 '{}'。\n\n", rule
        ));
    }

    prompt.push_str("这是异常吗？操作员应该做什么？");
    prompt
}

/// 解析 LLM 返回的 JSON（自动剥离 markdown 代码块）
fn parse_analysis_response(response: &str) -> anyhow::Result<AnalysisResult> {
    // 剥离 ```json ... ``` 包裹
    let content = response
        .trim()
        .strip_prefix("```json")
        .or_else(|| response.strip_prefix("```"))
        .unwrap_or(response)
        .trim_end_matches("```")
        .trim();

    let json: serde_json::Value = serde_json::from_str(content)?;

    Ok(AnalysisResult {
        is_anomaly: json["is_anomaly"].as_bool().unwrap_or(false),
        severity: json["severity"].as_str().unwrap_or("normal").to_string(),
        summary: json["summary"].as_str().unwrap_or("无分析结果").to_string(),
        recommendation: json["recommendation"].as_str().unwrap_or("无建议").to_string(),
        confidence: json["confidence"].as_f64().unwrap_or(0.5),
    })
}

/// 获取各 Provider 的默认 API 地址
fn default_base_url(provider: &str) -> &'static str {
    match provider.to_lowercase().as_str() {
        "openai" => "https://api.openai.com/v1",
        "anthropic" => "https://api.anthropic.com/v1",
        "deepseek" => "https://api.deepseek.com/v1",
        "qwen" => "https://dashscope.aliyuncs.com/compatible-mode/v1",
        "zhipu" => "https://open.bigmodel.cn/api/paas/v4",
        _ => "https://api.openai.com/v1", // 默认
    }
}
