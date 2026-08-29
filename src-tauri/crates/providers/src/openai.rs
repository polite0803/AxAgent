// SPDX-License-Identifier: AGPL-3.0-only

// 1.97 起 clippy::items_after_test_module 升级为 warn(在 `-D warnings` 下变 deny),
// 历史上把测试模块放在文件中间以贴近被测代码,这里显式 allow 保留现有排版。
#![allow(clippy::items_after_test_module)]

use std::sync::Arc;

use async_trait::async_trait;
use axagent_harness::constants::default_url;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::speech::{AudioChunkStream, SpeakRequest, SpeechCapabilities, SpeechInput};
use axagent_harness::types::*;
use futures::Stream;
use futures::StreamExt;
use serde::{Deserialize, Deserializer, Serialize};
use std::pin::Pin;

use crate::url_utils::resolve_chat_url;
use crate::{ProviderAdapter, ProviderRequestContext, build_http_client};

const DEFAULT_BASE_URL: &str = default_url::OPENAI_BASE;

pub struct OpenAIAdapter {
    client: reqwest::Client,
}

impl Default for OpenAIAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAIAdapter {
    pub fn new() -> Self {
        Self {
            client: crate::build_default_http_client().unwrap_or_else(|e| {
                tracing::warn!("无法构建 OpenAI HTTP 客户端: {e}，降级为默认客户端");
                reqwest::Client::new()
            }),
        }
    }

    fn base_url(ctx: &ProviderRequestContext) -> String {
        ctx.base_url.clone().unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
    }

    fn chat_url(ctx: &ProviderRequestContext) -> String {
        resolve_chat_url(&Self::base_url(ctx), ctx.api_path.as_deref(), "/chat/completions")
    }

    #[allow(clippy::result_large_err)]
    pub fn get_client(&self, ctx: &ProviderRequestContext) -> Result<reqwest::Client> {
        match &ctx.proxy_config {
            Some(c) if c.proxy_type.as_deref() != Some("none") => build_http_client(Some(c)),
            _ => Ok(self.client.clone()),
        }
    }
}

// --- Internal request/response types ---

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_completion_tokens: Option<u32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ChatTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    /// SiliconFlow-style thinking toggle
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_thinking: Option<bool>,
    /// SiliconFlow-style thinking token budget
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<u32>,
    /// Structured Output 强制契约（OpenAI response_format）
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// For assistant messages: pass thinking back as reasoning_content when
    /// the provider requires it (e.g., SiliconFlow, DeepSeek thinking mode).
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    id: Option<String>,
    model: Option<String>,
    #[serde(default)]
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: Option<OpenAIMessageResp>,
    delta: Option<OpenAIDelta>,
}

#[derive(Deserialize)]
struct OpenAIMessageResp {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    reasoning_content: Option<String>,
    /// 通义千问 / 智谱 GLM 等国内厂商在 OpenAI 兼容模式下使用的思考字段
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    thinking: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    reasoning: Option<String>,
    reasoning_details: Option<Vec<ReasoningDetail>>,
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
    #[serde(flatten)]
    extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct OpenAIDelta {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    reasoning_content: Option<String>,
    /// 通义千问 / 智谱 GLM 等国内厂商在 OpenAI 兼容模式下使用的思考字段
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    thinking: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    reasoning: Option<String>,
    reasoning_details: Option<Vec<ReasoningDetail>>,
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
    #[serde(flatten)]
    extra: std::collections::BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ReasoningDetail {
    #[serde(default, deserialize_with = "deserialize_optional_text")]
    text: Option<String>,
}

/// Extract thinking text from delta/message fields.
/// Priority: reasoning_content > thinking > reasoning > reasoning_details[0].text
///
/// `reasoning_content` 是 DeepSeek / SiliconFlow 等厂商使用的字段；
/// `thinking` 是通义千问 / 智谱 GLM 等国内厂商在 OpenAI 兼容模式下使用的字段；
/// `reasoning` 与 `reasoning_details` 是其他兼容端点的回退字段。
fn extract_thinking(
    reasoning_content: &Option<String>,
    thinking: &Option<String>,
    reasoning: &Option<String>,
    reasoning_details: &Option<Vec<ReasoningDetail>>,
) -> Option<String> {
    if reasoning_content.is_some() {
        return reasoning_content.clone();
    }
    if thinking.is_some() {
        return thinking.clone();
    }
    if reasoning.is_some() {
        return reasoning.clone();
    }
    reasoning_details.as_ref().and_then(|details| details.first()).and_then(|d| d.text.clone())
}

fn deserialize_optional_text<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|raw| extract_text_from_json(&raw)))
}

fn deserialize_optional_json_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.map(|raw| match raw {
        serde_json::Value::String(text) => text,
        other => other.to_string(),
    }))
}

fn extract_text_from_json(value: &serde_json::Value) -> Option<String> {
    fn collect_text(value: &serde_json::Value, out: &mut String) {
        match value {
            serde_json::Value::String(text) => out.push_str(text),
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_text(item, out);
                }
            },
            serde_json::Value::Object(map) => {
                for key in ["text", "content", "delta", "parts", "part", "value", "output_text"] {
                    if let Some(child) = map.get(key) {
                        let before = out.len();
                        collect_text(child, out);
                        if out.len() > before {
                            return;
                        }
                    }
                }
            },
            _ => {},
        }
    }

    let mut text = String::new();
    collect_text(value, &mut text);
    if text.is_empty() { None } else { Some(text) }
}

fn extract_primary_content(
    content: &Option<String>,
    extra: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    if content.is_some() {
        return content.clone();
    }

    for key in ["text", "part", "parts", "value", "output_text"] {
        if let Some(value) = extra.get(key)
            && let Some(text) = extract_text_from_json(value)
        {
            return Some(text);
        }
    }

    None
}

fn extract_gemini_compat_chunk(data: &str) -> Option<ChatStreamChunk> {
    let parsed = serde_json::from_str::<GeminiCompatChunk>(data).ok()?;
    let content = parsed
        .candidates
        .as_ref()
        .and_then(|candidates| candidates.first())
        .and_then(|candidate| candidate.content.as_ref())
        .map(|content| {
            content.parts.iter().filter_map(|part| part.text.as_ref()).cloned().collect::<String>()
        })
        .filter(|text| !text.is_empty());

    let usage = parsed.usage_metadata.map(|usage| TokenUsage {
        input_tokens: usage.prompt_token_count.unwrap_or(0),
        output_tokens: usage.candidates_token_count.unwrap_or(0),
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: usage.cached_content_token_count.unwrap_or(0),
        cache_miss_input_tokens: None,
    });

    if content.is_none() && usage.is_none() {
        return None;
    }

    Some(ChatStreamChunk {
        content,
        thinking: None,
        done: false,
        is_final: None,
        usage,
        tool_calls: None,
    })
}

#[derive(Deserialize, Default)]
struct OpenAIUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    // DeepSeek 风格: 顶层 prompt_cache_*_tokens
    #[serde(default)]
    prompt_cache_hit_tokens: Option<u32>,
    // P1/P2 使用: DeepSeek 缓存未命中计数, 用于命中率埋点
    #[serde(default)]
    prompt_cache_miss_tokens: Option<u32>,
    // Kimi 风格: 顶层 cached_tokens (与 prompt_tokens_details.cached_tokens 不同位置)
    #[serde(default)]
    cached_tokens: Option<u32>,
    // OpenAI / MiMo 风格: 嵌套 prompt_tokens_details.cached_tokens
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

impl OpenAIUsage {
    /// 归一化缓存命中 token 数: 优先取 OpenAI 嵌套 cached_tokens,
    /// 回退到 DeepSeek 顶层 prompt_cache_hit_tokens, 都没有则 None.
    fn cache_read_tokens(&self) -> Option<u32> {
        if let Some(cached) = self.prompt_tokens_details.as_ref().and_then(|d| d.cached_tokens) {
            return Some(cached);
        }
        if let Some(cached) = self.cached_tokens {
            return Some(cached);
        }
        self.prompt_cache_hit_tokens
    }

    fn to_token_usage(&self) -> TokenUsage {
        TokenUsage {
            input_tokens: self.prompt_tokens,
            output_tokens: self.completion_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: self.cache_read_tokens().unwrap_or(0),
            cache_miss_input_tokens: self.prompt_cache_miss_tokens,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
struct OpenAIToolCallDelta {
    index: usize,
    id: Option<String>,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<OpenAIToolCallFunctionDelta>,
}

#[derive(Deserialize, Debug, Clone)]
struct OpenAIToolCallFunctionDelta {
    name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_json_string")]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

// Wrapped format used by API gateways (OneAPI/NewAPI etc.): {"code":0,"data":{"data":[...]}}
#[derive(Deserialize)]
struct WrappedModelsResponse {
    data: OpenAIModelsResponse,
}

#[derive(Deserialize)]
struct OpenAIModel {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCompatChunk {
    candidates: Option<Vec<GeminiCompatCandidate>>,
    usage_metadata: Option<GeminiCompatUsageMetadata>,
}

#[derive(Deserialize)]
struct GeminiCompatCandidate {
    content: Option<GeminiCompatContent>,
}

#[derive(Deserialize)]
struct GeminiCompatContent {
    parts: Vec<GeminiCompatPart>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCompatPart {
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCompatUsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    /// Gemini 上下文缓存命中 token 数 (cachedContentTokenCount).
    #[serde(default)]
    cached_content_token_count: Option<u32>,
}

// --- Embedding types ---

#[derive(Serialize)]
struct OpenAIEmbedRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedData>,
}

#[derive(Deserialize)]
struct OpenAIEmbedData {
    embedding: Vec<f32>,
}

// Re-export shared utilities for backward compatibility
pub use crate::extract_reasoning_from_text;

fn convert_messages(messages: &[ChatMessage]) -> Vec<OpenAIMessage> {
    messages
        .iter()
        .map(|msg| {
            match msg.role.as_str() {
                "tool" => OpenAIMessage {
                    role: "tool".to_string(),
                    content: Some(serde_json::Value::String(crate::extract_text_content(&msg.content))),
                    tool_calls: None,
                    tool_call_id: msg.tool_call_id.clone(),
                    reasoning_content: None,
                },
                "assistant" => {
                    let content_text = crate::extract_text_content(&msg.content);
                    let (visible_text, reasoning_from_text) = crate::extract_reasoning_from_text(&content_text);
                    // Priority: msg.thinking (dedicated field from API reasoning_content)
                    // > <think> tag parsing from visible text
                    // This ensures providers that return reasoning_content as a separate API field
                    // (e.g., SiliconFlow/DeepSeek thinking mode) have it passed back correctly.
                    let reasoning = msg.thinking.clone().or(reasoning_from_text);
                    let content = if visible_text.is_empty() {
                        None
                    } else {
                        Some(match &msg.content {
                            ChatContent::Text(_) => serde_json::Value::String(visible_text),
                            ChatContent::Multipart(parts) => serde_json::Value::Array(
                                parts
                                    .iter()
                                    .map(|part| {
                                        let mut value = serde_json::Map::new();
                                        value.insert(
                                            "type".to_string(),
                                            serde_json::Value::String(part.r#type.clone()),
                                        );
                                        if let Some(text) = &part.text {
                                            let (v, _) = crate::extract_reasoning_from_text(text);
                                            value.insert("text".to_string(), serde_json::Value::String(v));
                                        }
                                        if let Some(image_url) = &part.image_url {
                                            value.insert(
                                                "image_url".to_string(),
                                                serde_json::to_value(image_url).unwrap_or(serde_json::Value::Null),
                                            );
                                        }
                                        serde_json::Value::Object(value)
                                    })
                                    .collect(),
                            ),
                        })
                    };
                    OpenAIMessage {
                        role: "assistant".to_string(),
                        content,
                        tool_calls: msg.tool_calls.as_ref().map(|tcs| {
                            tcs.iter().map(|tc| serde_json::json!({
                                "id": tc.id,
                                "type": tc.call_type,
                                "function": { "name": tc.function.name, "arguments": tc.function.arguments }
                            })).collect()
                        }),
                        tool_call_id: None,
                        reasoning_content: if msg.thinking.is_some() { reasoning } else { None },
                    }
                }
                _ => {
                    let content = match &msg.content {
                        ChatContent::Text(text) => serde_json::Value::String(text.clone()),
                        ChatContent::Multipart(parts) => serde_json::Value::Array(
                            parts
                                .iter()
                                .map(|part| {
                                    let mut value = serde_json::Map::new();
                                    value.insert(
                                        "type".to_string(),
                                        serde_json::Value::String(part.r#type.clone()),
                                    );
                                    if let Some(text) = &part.text {
                                        value.insert("text".to_string(), serde_json::Value::String(text.clone()));
                                    }
                                    if let Some(image_url) = &part.image_url {
                                        value.insert(
                                            "image_url".to_string(),
                                            serde_json::to_value(image_url).unwrap_or(serde_json::Value::Null),
                                        );
                                    }
                                    serde_json::Value::Object(value)
                                })
                                .collect(),
                        ),
                    };
                    OpenAIMessage {
                        role: msg.role.clone(),
                        content: Some(content),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    }
                }
            }
        })
        .collect()
}

/// 将 provider-中性的 ResponseFormat 转换为 OpenAI API 的 response_format 格式。
fn convert_response_format(fmt: &ResponseFormat) -> serde_json::Value {
    match fmt {
        ResponseFormat::JsonObject => serde_json::json!({ "type": "json_object" }),
        ResponseFormat::JsonSchema { name, schema, strict } => {
            let mut json_schema = serde_json::json!({ "name": name, "schema": schema });
            if let Some(s) = strict {
                json_schema["strict"] = serde_json::Value::Bool(*s);
            }
            serde_json::json!({ "type": "json_schema", "json_schema": json_schema })
        },
    }
}

fn build_request(
    ctx: &ProviderRequestContext,
    request: &ChatRequest,
    messages: &[ChatMessage],
    stream: bool,
) -> OpenAIRequest {
    let base_url = ctx.base_url.as_deref().unwrap_or(DEFAULT_BASE_URL);
    let is_siliconflow = base_url.contains("siliconflow.cn");
    let mut thinking_style = request.thinking_param_style.as_deref().unwrap_or("reasoning_effort");

    // 兼容性降级:非 SiliconFlow 端点不识别 enable_thinking / thinking_budget
    if thinking_style == "enable_thinking" && !is_siliconflow {
        tracing::warn!(
            target: "axagent.providers",
            provider_id = %ctx.provider_id,
            base_url = %base_url,
            "thinking_param_style='enable_thinking' 非 SiliconFlow, 降级为 reasoning_effort"
        );
        thinking_style = "reasoning_effort";
    }

    // Structured Output 兼容性降级：
    // DeepSeek / Kimi / MiniMax 等不支持 json_schema 严格模式，
    // 降级为 json_object + system prompt 注入 schema 约束。
    let (response_format, final_messages) = if let Some(fmt) = request.response_format.as_ref() {
        if matches!(fmt, ResponseFormat::JsonSchema { .. })
            && !crate::structured_output::supports_json_schema_strict(base_url)
        {
            tracing::warn!(
                target: "axagent.providers",
                provider_id = %ctx.provider_id,
                base_url = %base_url,
                "response_format=JsonSchema 降级为 JsonObject + prompt 注入（provider 不支持严格模式）"
            );
            let constraint = crate::structured_output::build_schema_constraint(fmt);
            let new_messages = crate::structured_output::inject_constraint_into_messages(
                messages.to_vec(),
                &constraint,
            );
            (Some(serde_json::json!({ "type": "json_object" })), new_messages)
        } else {
            (Some(crate::openai::convert_response_format(fmt)), messages.to_vec())
        }
    } else {
        (None, messages.to_vec())
    };

    // "none" style: never send any thinking-related params
    // "enable_thinking" style (SiliconFlow): enable_thinking + thinking_budget fields
    let (enable_thinking, sf_thinking_budget) = if thinking_style == "enable_thinking" {
        match request.thinking_budget {
            Some(0) => (Some(false), None),
            Some(b) => (Some(true), Some(b.max(128))),
            None => (None, None),
        }
    } else {
        (None, None)
    };

    // "reasoning_effort" style (OpenAI): reasoning_effort field
    let reasoning_effort = if thinking_style == "reasoning_effort" {
        request.thinking_budget.map(|b| match b {
            0 => "none".to_string(),
            1..=2048 => "low".to_string(),
            2049..=6144 => "medium".to_string(),
            _ => "high".to_string(),
        })
    } else {
        None
    };

    let has_thinking = reasoning_effort.is_some() || enable_thinking == Some(true);

    // Use max_completion_tokens when: model config says so, reasoning mode,
    // o-series models, or gpt-5+ (which deprecate max_tokens)
    let use_completion_tokens = request.use_max_completion_tokens == Some(true)
        || has_thinking
        || request.model.starts_with("o1")
        || request.model.starts_with("o3")
        || request.model.starts_with("o4")
        || request.model.starts_with("gpt-5");

    let (max_tokens, max_completion_tokens) = if use_completion_tokens {
        (None, request.max_tokens.filter(|&v| v > 0))
    } else {
        (request.max_tokens.filter(|&v| v > 0), None)
    };

    // 始终发送 include_usage，确保流式响应中返回 token 用量。
    // 标准 OpenAI / DeepSeek / SiliconFlow 原生支持；
    // 其他兼容提供商(含 CC Switch 等自定义网关)通常忽略未知字段，不会报错。
    let stream_options = if stream {
        Some(StreamOptions { include_usage: true })
    } else {
        None
    };

    let body = OpenAIRequest {
        model: request.model.clone(),
        messages: convert_messages(&final_messages),
        temperature: if has_thinking {
            None
        } else {
            request.temperature
        },
        top_p: if has_thinking { None } else { request.top_p },
        max_tokens,
        max_completion_tokens,
        stream,
        stream_options,
        tools: request.tools.clone(),
        reasoning_effort,
        enable_thinking,
        thinking_budget: sf_thinking_budget,
        response_format,
    };

    tracing::debug!(
        target: "axagent.providers",
        model = %body.model,
        base_url = %base_url,
        thinking_style = %thinking_style,
        has_thinking = has_thinking,
        enable_thinking = ?body.enable_thinking,
        thinking_budget = ?body.thinking_budget,
        reasoning_effort = ?body.reasoning_effort,
        max_tokens = ?body.max_tokens,
        max_completion_tokens = ?body.max_completion_tokens,
        tools_count = body.tools.as_ref().map(|t| t.len()).unwrap_or(0),
        "openai build_request"
    );

    body
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn convert_messages_omits_null_fields_for_openai_compatible_requests() {
        let messages = convert_messages(&[ChatMessage {
            role: "user".to_string(),
            content: ChatContent::Multipart(vec![
                ContentPart {
                    r#type: "text".to_string(),
                    text: Some("Describe this image".to_string()),
                    image_url: None,
                },
                ContentPart {
                    r#type: "image_url".to_string(),
                    text: None,
                    image_url: Some(ImageUrl { url: "data:image/png;base64,YWJj".to_string() }),
                },
            ]),
            tool_calls: None,
            tool_call_id: None,
            thinking: None,
        }]);

        assert_eq!(
            messages[0].content,
            Some(json!([
                { "type": "text", "text": "Describe this image" },
                {
                    "type": "image_url",
                    "image_url": { "url": "data:image/png;base64,YWJj" }
                }
            ]))
        );
    }
}

#[async_trait]
impl ProviderAdapter for OpenAIAdapter {
    async fn chat(
        &self,
        ctx: &ProviderRequestContext,
        request: Arc<ChatRequest>,
    ) -> Result<ChatResponse> {
        let url = Self::chat_url(ctx);
        let body = build_request(ctx, &request, &request.messages, false);

        let body_size = serde_json::to_string(&body).ok().map(|s| s.len()).unwrap_or(0);
        // P0 DIAG: body 组成分析 — 定位 5.4MB/140万 tokens 到底来自 messages 还是 tools
        let messages_size =
            serde_json::to_string(&body.messages).ok().map(|s| s.len()).unwrap_or(0);
        let tools_size = body
            .tools
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok())
            .map(|s| s.len())
            .unwrap_or(0);
        let messages_content_chars: usize = body
            .messages
            .iter()
            .map(|m| match &m.content {
                Some(c) => c.as_str().map(|s| s.len()).unwrap_or(0),
                None => 0,
            })
            .sum();
        tracing::info!(
            target: "axagent.providers.req",
            url = %url,
            provider_id = %ctx.provider_id,
            model = %body.model,
            stream = body.stream,
            tools_count = body.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            messages_count = body.messages.len(),
            body_size_bytes = body_size,
            messages_size_bytes = messages_size,
            messages_content_chars = messages_content_chars,
            tools_size_bytes = tools_size,
            "[PROVIDER.chat] 即将发送请求"
        );

        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .post(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key))
                .json(&body),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("OpenAI API error {status}: {text}")));
        }

        let raw_text = resp.text().await.unwrap_or_default();
        // P0 DIAG: 字符级截断（避免 UTF-8 边界 panic）
        let preview: String = raw_text.chars().take(500).collect();
        tracing::info!(
            target: "axagent.providers.resp",
            provider_id = %ctx.provider_id,
            model = %body.model,
            raw_len = raw_text.len(),
            raw_preview = %preview,
            "📨 [RAW] chat 响应原始 body"
        );

        let oai: OpenAIResponse = serde_json::from_str(&raw_text)
            .map_err(|e| AxAgentError::Provider(format!("Parse error: {e}")))?;

        let choice = oai
            .choices
            .first()
            .ok_or_else(|| AxAgentError::Provider("No choices in response".into()))?;
        let msg = choice
            .message
            .as_ref()
            .ok_or_else(|| AxAgentError::Provider("No message in choice".into()))?;

        tracing::info!(
            target: "axagent.providers.resp",
            provider_id = %ctx.provider_id,
            model = %body.model,
            content_len = msg.content.as_deref().map(|c| c.len()).unwrap_or(0),
            has_thinking = msg.thinking.is_some(),
            tool_calls_count = msg.tool_calls.as_ref().map(|t| t.len()).unwrap_or(0),
            "[PROVIDER.chat] 收到响应"
        );

        let usage = oai.usage.map(|u| u.to_token_usage()).unwrap_or_default();

        let tool_calls = msg.tool_calls.as_ref().map(|tcs| {
            tcs.iter()
                .map(|tc| axagent_harness::types::ToolCall {
                    id: tc.id.clone().unwrap_or_default(),
                    call_type: tc.call_type.clone().unwrap_or_else(|| "function".into()),
                    function: axagent_harness::types::ToolCallFunction {
                        name: tc.function.as_ref().and_then(|f| f.name.clone()).unwrap_or_default(),
                        arguments: tc
                            .function
                            .as_ref()
                            .and_then(|f| f.arguments.clone())
                            .unwrap_or_default(),
                    },
                })
                .collect()
        });

        Ok(ChatResponse {
            id: oai.id.unwrap_or_default(),
            model: oai.model.unwrap_or_else(|| request.model.clone()),
            content: extract_primary_content(&msg.content, &msg.extra).unwrap_or_default(),
            thinking: extract_thinking(
                &msg.reasoning_content,
                &msg.thinking,
                &msg.reasoning,
                &msg.reasoning_details,
            ),
            usage,
            tool_calls,
        })
    }

    fn chat_stream(
        &self,
        ctx: &ProviderRequestContext,
        request: ChatRequest,
        cancel_token: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatStreamChunk>> + Send>> {
        let client = self.get_client(ctx).unwrap_or_else(|e| {
            tracing::warn!("Failed to build proxy-aware HTTP client, falling back to default: {e}");
            self.client.clone()
        });
        let api_key = ctx.api_key.clone();
        let custom_headers = ctx.custom_headers.clone();
        let url = Self::chat_url(ctx);
        let body = build_request(ctx, &request, &request.messages, true);
        let provider_id = ctx.provider_id.clone();

        let body_size = serde_json::to_string(&body).ok().map(|s| s.len()).unwrap_or(0);
        // P0 DIAG: body 组成分析 — 定位 5.4MB/140万 tokens 到底来自 messages 还是 tools
        let messages_size =
            serde_json::to_string(&body.messages).ok().map(|s| s.len()).unwrap_or(0);
        let tools_size = body
            .tools
            .as_ref()
            .and_then(|t| serde_json::to_string(t).ok())
            .map(|s| s.len())
            .unwrap_or(0);
        let messages_content_chars: usize = body
            .messages
            .iter()
            .map(|m| match &m.content {
                Some(c) => c.as_str().map(|s| s.len()).unwrap_or(0),
                None => 0,
            })
            .sum();
        // P0 DIAG: 逐条消息分析 — 定位哪条消息撑爆了 context
        tracing::info!(
            target: "axagent.providers.req",
            "[PROVIDER.chat_stream] 逐条消息分析 (共 {} 条):",
            body.messages.len()
        );
        for (i, m) in body.messages.iter().enumerate() {
            let chars = match &m.content {
                Some(c) => c.as_str().map(|s| s.len()).unwrap_or(0),
                None => 0,
            };
            let preview = match &m.content {
                Some(c) => c
                    .as_str()
                    .map(|s| {
                        let trimmed = s.chars().take(80).collect::<String>();
                        if s.chars().count() > 80 {
                            format!("{}...(+{} chars)", trimmed, s.len() - 80)
                        } else {
                            trimmed
                        }
                    })
                    .unwrap_or_default(),
                None => String::from("(无内容)"),
            };
            let tool_calls = m.tool_calls.as_ref().map(|tcs| tcs.len()).unwrap_or(0);
            tracing::info!(
                target: "axagent.providers.req",
                "  [msg#{}] role={} chars={} tool_calls={} preview=\"{}\"",
                i, m.role, chars, tool_calls, preview
            );
        }
        tracing::info!(
            target: "axagent.providers.req",
            url = %url,
            provider_id = %provider_id,
            model = %body.model,
            stream = body.stream,
            tools_count = body.tools.as_ref().map(|t| t.len()).unwrap_or(0),
            messages_count = body.messages.len(),
            body_size_bytes = body_size,
            messages_size_bytes = messages_size,
            messages_content_chars = messages_content_chars,
            tools_size_bytes = tools_size,
            "[PROVIDER.chat_stream] 即将发送请求"
        );

        let (mut tx, rx) = futures::channel::mpsc::channel(256);

        tokio::spawn(async move {
            let resp = match crate::apply_stream_headers_to_request(
                client
                    .post(&url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .json(&body),
                &custom_headers,
            )
            .send()
            .await
            {
                Ok(r) if r.status().is_success() => {
                    let ct = r
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    tracing::info!(
                        target: "axagent.providers.sse",
                        status = r.status().as_u16(),
                        content_type = ct,
                        url = %url,
                        provider_id = %provider_id,
                        model = %body.model,
                        body_preview = %serde_json::to_string(&body).ok().map(|s| s[..s.len().min(500)].to_string()).unwrap_or_default(),
                        "[SSE-DIAG] HTTP 响应成功"
                    );
                    r
                },
                Ok(r) => {
                    let s = r.status();
                    let t = r.text().await.unwrap_or_default();
                    let _ = tx.try_send(Err(AxAgentError::execution_with_source(
                        super::diagnose_http_status("OpenAI", s, &t),
                        anyhow::anyhow!("HTTP {s}: {t}"),
                    )));
                    return;
                },
                Err(e) => {
                    let _ = tx.try_send(Err(AxAgentError::execution_with_source(
                        super::diagnose_reqwest_error(&e),
                        e,
                    )));
                    return;
                },
            };

            let mut byte_stream = resp.bytes_stream();
            let mut buf = String::new();
            let mut pending_tool_calls: Vec<(String, String, String, String)> = Vec::new();
            let mut event_data_lines: Vec<String> = Vec::new();
            let mut last_usage: Option<axagent_harness::types::TokenUsage> = None;
            let mut total_bytes_received: usize = 0;
            let mut total_data_events: usize = 0;

            let mut process_event = |data: &str| -> bool {
                tracing::info!(
                    target: "axagent.providers.sse",
                    raw = %data[..data.len().min(300)].replace('\n', " "),
                    "[SSE-DIAG] 收到 data 事件"
                );
                if data.trim() == "[DONE]" {
                    let tool_calls = if pending_tool_calls.is_empty() {
                        None
                    } else {
                        Some(
                            pending_tool_calls
                                .iter()
                                .map(|(id, ct, name, args)| axagent_harness::types::ToolCall {
                                    id: id.clone(),
                                    call_type: ct.clone(),
                                    function: axagent_harness::types::ToolCallFunction {
                                        name: name.clone(),
                                        arguments: args.clone(),
                                    },
                                })
                                .collect(),
                        )
                    };
                    let _ = tx.try_send(Ok(ChatStreamChunk {
                        content: None,
                        thinking: None,
                        done: true,
                        is_final: None,
                        usage: last_usage.take(),
                        tool_calls,
                    }));
                    return true;
                }

                let parsed = match serde_json::from_str::<OpenAIResponse>(data) {
                    Ok(value) => value,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse SSE event JSON: {e}. Data: {}",
                            &data[..data.len().min(200)]
                        );
                        return false;
                    },
                };

                if let Some(choice) = parsed.choices.first() {
                    let tool_call_deltas = choice
                        .delta
                        .as_ref()
                        .and_then(|delta| delta.tool_calls.as_ref())
                        .or_else(|| {
                            choice.message.as_ref().and_then(|message| message.tool_calls.as_ref())
                        });
                    if let Some(tc_deltas) = tool_call_deltas {
                        for tc in tc_deltas {
                            tracing::info!(
                                target: "axagent.providers.toolcall",
                                index = tc.index,
                                id = ?tc.id,
                                call_type = ?tc.call_type,
                                name = ?tc.function.as_ref().and_then(|f| f.name.as_ref()),
                                args_preview = ?tc.function.as_ref().and_then(|f| f.arguments.as_ref()).map(|a| &a[..a.len().min(100)]),
                                "tool_call delta received"
                            );
                            // 上限保护:防止恶意/异常上游把 index 推到很大,
                            // 导致 vector grow 吃掉内存
                            const MAX_PENDING_TOOL_CALLS: usize = 256;
                            let idx =
                                (tc.index as u32).min(MAX_PENDING_TOOL_CALLS as u32 - 1) as usize;
                            while pending_tool_calls.len() <= idx {
                                pending_tool_calls.push((
                                    String::new(),
                                    String::from("function"),
                                    String::new(),
                                    String::new(),
                                ));
                            }
                            if let Some(ref id) = tc.id
                                && !id.is_empty()
                            {
                                pending_tool_calls[idx].0 = id.clone();
                            }
                            if let Some(ref ct) = tc.call_type
                                && !ct.is_empty()
                            {
                                pending_tool_calls[idx].1 = ct.clone();
                            }
                            if let Some(ref f) = tc.function {
                                if let Some(ref name) = f.name
                                    && !name.is_empty()
                                {
                                    pending_tool_calls[idx].2 = name.clone();
                                }
                                if let Some(ref args) = f.arguments {
                                    pending_tool_calls[idx].3.push_str(args);
                                }
                            }
                        }
                    }

                    let usage = parsed.usage.map(|u| u.to_token_usage());
                    if let Some(u) = usage {
                        last_usage = Some(u);
                    }
                    let content = choice
                        .delta
                        .as_ref()
                        .and_then(|delta| extract_primary_content(&delta.content, &delta.extra))
                        .or_else(|| {
                            choice.message.as_ref().and_then(|message| {
                                extract_primary_content(&message.content, &message.extra)
                            })
                        });
                    let thinking = choice
                        .delta
                        .as_ref()
                        .and_then(|delta| {
                            extract_thinking(
                                &delta.reasoning_content,
                                &delta.thinking,
                                &delta.reasoning,
                                &delta.reasoning_details,
                            )
                        })
                        .or_else(|| {
                            choice.message.as_ref().and_then(|message| {
                                extract_thinking(
                                    &message.reasoning_content,
                                    &message.thinking,
                                    &message.reasoning,
                                    &message.reasoning_details,
                                )
                            })
                        });

                    tracing::info!(
                        target: "axagent.providers.sse",
                        has_content = content.is_some(),
                        has_thinking = thinking.is_some(),
                        has_tool_calls_parsed = !pending_tool_calls.is_empty(),
                        content_preview = content.as_deref().map(|s| &s[..s.len().min(80)]).unwrap_or(""),
                        thinking_preview = thinking.as_deref().map(|s| &s[..s.len().min(80)]).unwrap_or(""),
                        "[SSE-DIAG] choice 解析结果"
                    );
                    // P0: 去掉 content/thinking 空 chunk 过滤 — 初始 role chunk、usage 更新
                    // chunk 等都需要透传给下游。即使 content/thinking 都为空也发送（心跳），
                    // 保证 agent runtime 能感知连接活跃。
                    let _ = tx.try_send(Ok(ChatStreamChunk {
                        content,
                        thinking,
                        done: false,
                        is_final: None,
                        usage: None,
                        tool_calls: None,
                    }));
                    return false;
                }

                if let Some(u) = parsed.usage {
                    last_usage = Some(u.to_token_usage());
                }

                if let Some(chunk) = extract_gemini_compat_chunk(data) {
                    let _ = tx.try_send(Ok(chunk));
                }

                false
            };

            while let Some(chunk) = byte_stream.next().await {
                if let Some(ref token) = cancel_token
                    && token.load(std::sync::atomic::Ordering::Relaxed)
                {
                    let _ =
                        tx.try_send(Err(AxAgentError::Provider("Stream cancelled".to_string())));
                    return;
                }
                match chunk {
                    Ok(bytes) => {
                        total_bytes_received += bytes.len();
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buf.find('\n') {
                            let line = buf[..pos].trim_end_matches('\r').to_string();
                            buf = buf[pos + 1..].to_string();

                            if line.is_empty() {
                                if event_data_lines.is_empty() {
                                    continue;
                                }
                                let data = event_data_lines.join("\n");
                                event_data_lines.clear();
                                total_data_events += 1;
                                if process_event(&data) {
                                    return;
                                }
                                continue;
                            }

                            if line.starts_with(':') {
                                continue;
                            }

                            if let Some(d) = line.strip_prefix("data: ") {
                                event_data_lines.push(d.to_string());
                            } else if let Some(d) = line.strip_prefix("data:") {
                                event_data_lines.push(d.to_string());
                            }
                        }
                    },
                    Err(e) => {
                        let _ = tx.try_send(Err(AxAgentError::Provider(format!(
                            "Stream error: {e}. This may be caused by network instability, proxy issues, or the provider terminating the connection. Please try again."
                        ))));
                        return;
                    },
                }
            }

            let trailing_line = buf.trim_end_matches('\r');
            if let Some(d) = trailing_line.strip_prefix("data: ") {
                event_data_lines.push(d.to_string());
            } else if let Some(d) = trailing_line.strip_prefix("data:") {
                event_data_lines.push(d.to_string());
            }

            if !event_data_lines.is_empty() {
                let data = event_data_lines.join("\n");
                total_data_events += 1;
                if process_event(&data) {
                    return;
                }
            }

            // 流结束统计
            tracing::info!(
                target: "axagent.providers.sse",
                total_bytes_received,
                total_data_events,
                pending_tool_calls = pending_tool_calls.len(),
                has_last_usage = last_usage.is_some(),
                trailing_data = !buf.is_empty(),
                "[SSE-DIAG] 流结束统计"
            );

            // Stream ended without explicit [DONE]
            // 即使流异常结束,也要把已累计的 usage 透出,便于上层统计 token 消耗
            let last_usage = last_usage.take();
            let _ = tx.try_send(Ok(ChatStreamChunk {
                content: None,
                thinking: None,
                done: true,
                is_final: None,
                usage: last_usage,
                tool_calls: None,
            }));
        });

        Box::pin(rx)
    }

    async fn list_models(&self, ctx: &ProviderRequestContext) -> Result<Vec<Model>> {
        let url = format!("{}/models", Self::base_url(ctx));

        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .get(&url)
                .timeout(std::time::Duration::from_secs(5))
                .header("Authorization", format!("Bearer {}", ctx.api_key)),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Request failed: {e}")))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("OpenAI API error {s}: {t}")));
        }

        let body =
            resp.text().await.map_err(|e| AxAgentError::Provider(format!("Read error: {e}")))?;

        let convert = |models: Vec<OpenAIModel>| -> Vec<Model> {
            models
                .into_iter()
                .map(|m| {
                    let model_type =
                        axagent_harness::types::provider_model::detect_model_type(&m.id);
                    let mut caps = match model_type {
                        ModelType::Chat => vec![ModelCapability::TextChat],
                        ModelType::Embedding => vec![],
                        ModelType::Voice => vec![ModelCapability::RealtimeVoice],
                    };
                    let id_lower = m.id.to_lowercase();
                    if id_lower.contains("gpt-4o")
                        || id_lower.contains("gpt-4-turbo")
                        || id_lower.contains("claude")
                        || id_lower.contains("vision")
                    {
                        caps.push(ModelCapability::Vision);
                    }
                    if id_lower.starts_with("o1")
                        || id_lower.starts_with("o3")
                        || id_lower.starts_with("o4")
                    {
                        caps.push(ModelCapability::Reasoning);
                    }
                    Model {
                        provider_id: ctx.provider_id.clone(),
                        model_id: m.id.clone(),
                        name: m.id,
                        group_name: None,
                        model_type,
                        capabilities: caps,
                        max_tokens: None,
                        max_output_tokens: None,
                        enabled: true,
                        param_overrides: None,
                        input_price_per_mtok: None,
                        output_price_per_mtok: None,
                    }
                })
                .collect()
        };

        // Try standard OpenAI format: {"data": [...]}
        if let Ok(r) = serde_json::from_str::<OpenAIModelsResponse>(&body) {
            return Ok(convert(r.data));
        }

        // Try wrapped gateway format: {"code":0,"data":{"data":[...]}}
        if let Ok(r) = serde_json::from_str::<WrappedModelsResponse>(&body) {
            return Ok(convert(r.data.data));
        }

        // Try bare array: [{"id": "model-1"}, ...]
        if let Ok(models) = serde_json::from_str::<Vec<OpenAIModel>>(&body) {
            return Ok(convert(models));
        }

        Err(AxAgentError::Provider(format!(
            "Unsupported models response format (body: {})",
            if body.len() > 200 {
                &body[..200]
            } else {
                &body
            }
        )))
    }

    async fn validate_key(&self, ctx: &ProviderRequestContext) -> Result<bool> {
        // 轻量端点:`/models` 仅校验鉴权有效性,
        // 比之前的 list_models 还要轻,避免触发模型列表遍历开销。
        // 401/403 → 鉴权失败;200/4xx(除鉴权) → 鉴权通过
        let url = format!("{}/models", Self::base_url(ctx));
        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .get(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key)),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Request failed: {e}")))?;
        let status = resp.status().as_u16();
        Ok(status != 401 && status != 403)
    }

    async fn embed(
        &self,
        ctx: &ProviderRequestContext,
        request: EmbedRequest,
    ) -> Result<EmbedResponse> {
        let url = format!("{}/embeddings", Self::base_url(ctx));
        let body = OpenAIEmbedRequest {
            model: request.model,
            input: request.input,
            dimensions: request.dimensions,
        };

        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .post(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key))
                .json(&body),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("Embed request failed: {e}")))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("OpenAI embed API error {s}: {t}")));
        }

        let result: OpenAIEmbedResponse = resp
            .json()
            .await
            .map_err(|e| AxAgentError::Provider(format!("Embed parse error: {e}")))?;

        let dimensions = result.data.first().map(|d| d.embedding.len()).unwrap_or(0);
        let embeddings: Vec<Vec<f32>> = result.data.into_iter().map(|d| d.embedding).collect();

        Ok(EmbedResponse { embeddings, dimensions })
    }

    async fn realtime_config(
        &self,
        ctx: &ProviderRequestContext,
        model: &str,
    ) -> Result<axagent_harness::RealtimeProviderConfig> {
        // OpenAI Realtime API 的 WebSocket 端点
        let base = Self::base_url(ctx).replace("https://", "wss://").replace("http://", "ws://");
        let ws_url = format!("{base}/realtime?model={model}");
        Ok(axagent_harness::RealtimeProviderConfig {
            ws_url,
            api_key: ctx.api_key.clone(),
            headers: None,
            provider_type: "openai".to_string(),
        })
    }

    // ── Speech: STT / TTS ──

    fn supports_speech(&self) -> SpeechCapabilities {
        SpeechCapabilities::all()
    }

    async fn transcribe(&self, ctx: &ProviderRequestContext, input: SpeechInput) -> Result<String> {
        let base = Self::base_url(ctx);
        let url = format!("{}/audio/transcriptions", base.trim_end_matches('/'));

        // Whisper 需要带容器的音频（wav/mp3/...），把前端上传的裸 PCM16 包成 WAV。
        let wav = pcm_to_wav(&input.data, input.format.sample_rate, input.format.channels as u16);

        let part = reqwest::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AxAgentError::Provider(format!("STT audio part error: {e}")))?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", "whisper-1")
            .text("response_format", "json");

        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .post(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key))
                .multipart(form),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("STT request failed: {e}")))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("OpenAI STT API error {s}: {t}")));
        }

        #[derive(serde::Deserialize)]
        struct SttResp {
            text: String,
        }

        let result: SttResp = resp
            .json()
            .await
            .map_err(|e| AxAgentError::Provider(format!("STT parse error: {e}")))?;
        Ok(result.text)
    }

    async fn speech(
        &self,
        ctx: &ProviderRequestContext,
        req: SpeakRequest,
    ) -> Result<AudioChunkStream> {
        let base = Self::base_url(ctx);
        let url = format!("{}/audio/speech", base.trim_end_matches('/'));
        let model = req.model.clone().unwrap_or_else(|| "tts-1".to_string());
        let voice = req.voice.clone().unwrap_or_else(|| "alloy".to_string());

        let body = serde_json::json!({
            "model": model,
            "input": req.text,
            "voice": voice,
            // OpenAI `pcm` 返回 24kHz 16-bit 小端 PCM，正好匹配前端 24kHz PCM16 解码。
            "response_format": "pcm",
        });

        let resp = crate::apply_request_headers(
            self.get_client(ctx)?
                .post(&url)
                .header("Authorization", format!("Bearer {}", ctx.api_key))
                .json(&body),
            ctx,
        )
        .send()
        .await
        .map_err(|e| AxAgentError::Provider(format!("TTS request failed: {e}")))?;

        if !resp.status().is_success() {
            let s = resp.status();
            let t = resp.text().await.unwrap_or_default();
            return Err(AxAgentError::Provider(format!("OpenAI TTS API error {s}: {t}")));
        }

        // 直接以传输层流的方式转发音频字节，实现「边合成边播」。
        let stream = resp.bytes_stream().map(|r| {
            r.map(|b| b.to_vec())
                .map_err(|e| AxAgentError::Provider(format!("TTS stream error: {e}")))
        });
        Ok(Box::pin(stream))
    }
}

/// 把裸 PCM16 字节包成标准 WAV 文件头（44 字节），供 Whisper 等 API 使用。
fn pcm_to_wav(pcm: &[u8], sample_rate: u32, channels: u16) -> Vec<u8> {
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * bits_per_sample as u32 / 8;
    let block_align = channels * bits_per_sample / 8;
    let data_len = pcm.len() as u32;

    let mut wav = Vec::with_capacity(44 + pcm.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk 大小（PCM）
    wav.extend_from_slice(&1u16.to_le_bytes()); // audio format = PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(pcm);
    wav
}
