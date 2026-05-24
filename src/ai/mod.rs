//! AI Bridge 层：LLM API 调用、本地异常检测、Prompt 模板。
//!
//! 支持多 Provider：OpenAI、Anthropic、DeepSeek、通义千问、智谱、自定义。

pub mod bridge;
pub mod anomaly;
pub mod templates;

pub use bridge::Bridge;
