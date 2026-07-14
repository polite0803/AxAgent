// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

use super::settings_chat::AppSettings;
use crate::constants;

/// Deserialize `Option<Option<T>>` so that a JSON `null` becomes `Some(None)`
/// while a missing field (via `#[serde(default)]`) stays `None`.
pub(crate) fn deserialize_double_option<'de, T, D>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

// === Provider System ===

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_host: String,
    pub api_path: Option<String>,
    pub enabled: bool,
    pub models: Vec<Model>,
    pub keys: Vec<ProviderKey>,
    pub proxy_config: Option<ProviderProxyConfig>,
    /// 工具调用模式：None=按 provider_type 推断；"native"=模型原生 function calling；
    /// "managed"=由 AxAgent 通过提示词注入 + 文本解析模拟（用于 Chat2API 等无原生 tool 接口的网关）
    pub tool_adaptation: Option<String>,
    /// 托管模式下的 marker 前缀（仅 tool_adaptation="managed" 时生效）。
    /// None 或空字符串 = 使用默认值 "CHAT2API"。
    pub tool_adaptation_marker_prefix: Option<String>,
    pub custom_headers: Option<String>,
    pub icon: Option<String>,
    pub builtin_id: Option<String>,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    #[serde(rename = "openai_responses")]
    OpenAIResponses,
    Anthropic,
    Gemini,
    OpenClaw,
    Hermes,
    Ollama,
}

impl ProviderType {
    // Business methods extracted to free functions below.
}

/// ProviderType → ProviderRegistry 内部 key 的唯一权威映射。
///
/// 注册表 key 由 `axagent_providers::registry::ProviderRegistry::create_default`
/// 决定；新增 ProviderType 变体时必须同步在此实现，否则编译期立刻暴露
/// (match 必须穷尽所有变体)。所有调用方统一用 `provider_registry_key(pt)`，
/// 不允许再定义私有副本。
pub fn provider_registry_key(pt: &ProviderType) -> &'static str {
    match pt {
        ProviderType::OpenAI => "openai",
        ProviderType::OpenAIResponses => "openai_responses",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Gemini => "gemini",
        ProviderType::OpenClaw => "openclaw",
        ProviderType::Hermes => "hermes",
        ProviderType::Ollama => "ollama",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProviderKey {
    pub id: String,
    pub provider_id: String,
    pub key_encrypted: String,
    pub key_prefix: String,
    pub enabled: bool,
    pub last_validated_at: Option<i64>,
    pub last_error: Option<String>,
    pub rotation_index: u32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProviderProxyConfig {
    pub proxy_type: Option<String>,
    pub proxy_address: Option<String>,
    pub proxy_port: Option<u16>,
}

impl ProviderProxyConfig {
    // Business methods extracted to free functions below.
}

/// Resolve effective proxy: provider-level overrides global.
/// If provider has explicit proxy_type, use it (even "none" to disable).
/// Otherwise fall back to global settings.
pub fn resolve_provider_proxy(
    provider: &Option<ProviderProxyConfig>,
    global_settings: &AppSettings,
) -> Option<ProviderProxyConfig> {
    if let Some(config) = provider
        && config.proxy_type.is_some()
    {
        if config.proxy_type.as_deref() == Some("none") {
            return None;
        }
        return Some(config.clone());
    }
    // Fall back to global proxy
    match global_settings.proxy_type.as_deref() {
        Some("none") | None => None,
        Some("system") => Some(ProviderProxyConfig {
            proxy_type: Some(constants::role::SYSTEM.to_string()),
            proxy_address: None,
            proxy_port: None,
        }),
        _ => Some(ProviderProxyConfig {
            proxy_type: global_settings.proxy_type.clone(),
            proxy_address: global_settings.proxy_address.clone(),
            proxy_port: global_settings.proxy_port,
        }),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProviderInput {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_host: String,
    pub api_path: Option<String>,
    pub enabled: bool,
    #[serde(default)]
    pub builtin_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProviderInput {
    pub name: Option<String>,
    pub provider_type: Option<ProviderType>,
    pub api_host: Option<String>,
    pub api_path: Option<Option<String>>,
    pub enabled: Option<bool>,
    pub proxy_config: Option<ProviderProxyConfig>,
    #[serde(default)]
    pub tool_adaptation: Option<Option<String>>,
    #[serde(default)]
    pub tool_adaptation_marker_prefix: Option<Option<String>>,
    pub custom_headers: Option<Option<String>>,
    pub icon: Option<Option<String>>,
    pub sort_order: Option<i32>,
}

// === Model System ===

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct Model {
    pub provider_id: String,
    pub model_id: String,
    pub name: String,
    pub group_name: Option<String>,
    pub model_type: ModelType,
    pub capabilities: Vec<ModelCapability>,
    pub max_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub enabled: bool,
    pub param_overrides: Option<ModelParamOverrides>,
    /// Input price per million tokens (USD). When set, used for accurate cost calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_price_per_mtok: Option<f64>,
    /// Output price per million tokens (USD). When set, used for accurate cost calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_price_per_mtok: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, TS)]
pub enum ModelType {
    #[default]
    Chat,
    Voice,
    Embedding,
}

impl ModelType {
    // Business methods extracted to free functions below.
}

/// Auto-detect model type from model_id string.
///
/// 使用更精确的匹配策略避免误判：
/// - Embedding：text-embedding-* 或 embedding-* 前缀
/// - Voice：tts-*, whisper-*, realtime 等明确语音模型标识
/// - 其余为 Chat 类型
pub fn detect_model_type(model_id: &str) -> ModelType {
    let id = model_id.to_lowercase();
    if id.contains("text-embedding") || id.starts_with("embedding") || id.contains("-embedding") {
        ModelType::Embedding
    } else if id.contains("tts-") || id.contains("whisper-") || id.contains("realtime") {
        ModelType::Voice
    } else {
        ModelType::Chat
    }
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Chat => write!(f, "chat"),
            ModelType::Voice => write!(f, "voice"),
            ModelType::Embedding => write!(f, "embedding"),
        }
    }
}

impl std::str::FromStr for ModelType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "chat" => Ok(ModelType::Chat),
            "voice" => Ok(ModelType::Voice),
            "embedding" => Ok(ModelType::Embedding),
            _ => Ok(ModelType::Chat),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
pub enum ModelCapability {
    TextChat,
    Vision,
    FunctionCalling,
    Reasoning,
    RealtimeVoice,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ModelParamOverrides {
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    /// When true, the provider adapter should send `max_completion_tokens`
    /// instead of `max_tokens` (required by OpenAI o-series models).
    pub use_max_completion_tokens: Option<bool>,
    /// When true, system messages are converted to user messages
    /// (for models that don't support the system role).
    pub no_system_role: Option<bool>,
    /// When true, always include max_tokens in the request
    /// (falls back to 4096 if conversation.max_tokens is not set).
    pub force_max_tokens: Option<bool>,
    /// Thinking parameter format for the provider API.
    /// "reasoning_effort" (default/OpenAI) or "enable_thinking" (SiliconFlow).
    pub thinking_param_style: Option<String>,
    /// Delay in milliseconds before each API request to this model.
    /// Used to avoid hitting rate limits (e.g. 429 errors) on providers
    /// with strict per-model rate quotas.
    pub request_delay_ms: Option<u64>,
}

// === Conversation & Message ===
