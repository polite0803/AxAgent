//! AxAgent Provider Adapter for ClawCode Runtime

use axagent_core::types::{
    ChatContent, ChatMessage, ChatRequest, ChatTool, ContentPart, ImageUrl,
    TokenUsage as AxAgentTokenUsage, ToolCall, ToolCallFunction,
};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use axagent_runtime::{
    ApiClient, ApiRequest, AssistantEvent, ContentBlock, ConversationMessage, MessageRole,
    RuntimeError, TokenUsage as RuntimeTokenUsage,
};
use futures::StreamExt;
use std::sync::Arc;

/// Callback type invoked for each streamed event during `ApiClient::stream()`.
/// Allows the caller to emit Tauri events in real-time as chunks arrive,
/// rather than waiting for the entire response to be collected.
pub type StreamEventCallback = Box<dyn Fn(&AssistantEvent) + Send + Sync>;

/// Adapter that bridges AxAgent's ProviderAdapter to ClawCode Runtime's ApiClient trait
pub struct AxAgentApiClient {
    adapter: Arc<dyn ProviderAdapter>,
    ctx: ProviderRequestContext,
    /// Tool definitions to include in every ChatRequest so the LLM knows what tools are available.
    tools: Option<Vec<ChatTool>>,
    /// Model ID to include in each ChatRequest.
    model: String,
    /// Temperature parameter.
    temperature: Option<f64>,
    /// Top-p parameter.
    top_p: Option<f64>,
    /// Max tokens parameter.
    max_tokens: Option<u32>,
    /// Thinking/reasoning token budget.
    thinking_budget: Option<u32>,
    /// When true, send `max_completion_tokens` instead of `max_tokens` (OpenAI o-series).
    use_max_completion_tokens: Option<bool>,
    /// Thinking parameter format: "reasoning_effort" (default) or "enable_thinking" (SiliconFlow).
    thinking_param_style: Option<String>,
    /// Optional callback invoked for each streamed event (for real-time Tauri event emission).
    on_event: Option<Arc<StreamEventCallback>>,
    /// Image URLs (data: URLs) to inject into the last user message for multimodal support.
    /// The runtime's `ContentBlock` enum only supports text, so we inject images at the
    /// wire-format conversion layer in `convert_messages`.
    image_urls: Vec<String>,
    /// When true, the provider respects prompt cache breakpoints and sends
    /// cache-aware annotations (e.g., `cache_control: { "type": "ephemeral" }`) with
    /// the system message to instruct the provider to cache the prefix and avoid
    /// re-processing it on subsequent turns.
    pub enable_cache_breakpoints: bool,
    /// The hash of the system prompt that is being cached. When this changes,
    /// the cache is invalidated and the next request will not include breakpoint
    /// annotations until a new baseline is established.
    pub system_prompt_cache_hash: Option<String>,
}

impl AxAgentApiClient {
    /// Create a new AxAgentApiClient
    pub fn new(adapter: Arc<dyn ProviderAdapter>, ctx: ProviderRequestContext) -> Self {
        Self {
            adapter,
            ctx,
            tools: None,
            model: String::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            on_event: None,
            image_urls: Vec::new(),
            enable_cache_breakpoints: false,
            system_prompt_cache_hash: None,
        }
    }

    /// Create a new AxAgentApiClient with tool definitions.
    pub fn with_tools(
        adapter: Arc<dyn ProviderAdapter>,
        ctx: ProviderRequestContext,
        tools: Vec<ChatTool>,
    ) -> Self {
        Self {
            adapter,
            ctx,
            tools: if tools.is_empty() { None } else { Some(tools) },
            model: String::new(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            thinking_budget: None,
            use_max_completion_tokens: None,
            thinking_param_style: None,
            on_event: None,
            image_urls: Vec::new(),
            enable_cache_breakpoints: false,
            system_prompt_cache_hash: None,
        }
    }

    /// Set the model ID for ChatRequests.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temperature: Option<f64>) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set top-p.
    pub fn with_top_p(mut self, top_p: Option<f64>) -> Self {
        self.top_p = top_p;
        self
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: Option<u32>) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set thinking budget.
    pub fn with_thinking_budget(mut self, thinking_budget: Option<u32>) -> Self {
        self.thinking_budget = thinking_budget;
        self
    }

    /// Set use_max_completion_tokens flag.
    pub fn with_use_max_completion_tokens(
        mut self,
        use_max_completion_tokens: Option<bool>,
    ) -> Self {
        self.use_max_completion_tokens = use_max_completion_tokens;
        self
    }

    /// Set thinking parameter style.
    pub fn with_thinking_param_style(mut self, thinking_param_style: Option<String>) -> Self {
        self.thinking_param_style = thinking_param_style;
        self
    }

    /// Set a callback that will be invoked for each streamed event.
    /// This enables real-time Tauri event emission during streaming.
    pub fn with_on_event(mut self, callback: StreamEventCallback) -> Self {
        self.on_event = Some(Arc::new(callback));
        self
    }

    /// Set image URLs (data: URLs) to inject into the last user message.
    /// Used for multimodal support — the runtime only supports text input,
    /// so images are attached at the wire-format conversion layer.
    pub fn with_image_urls(mut self, urls: Vec<String>) -> Self {
        self.image_urls = urls;
        self
    }
}

impl AxAgentApiClient {
    /// Convert Runtime's ConversationMessage to one or more AxAgent ChatMessages.
    ///
    /// A single Runtime `ConversationMessage` may contain both text and
    /// `ToolUse` blocks. In the OpenAI-style wire format these map to:
    /// - assistant message with `tool_calls` + optional text content
    /// - `role: "tool"` messages for each `ToolResult`
    fn convert_messages(
        messages: &[ConversationMessage],
        image_urls: &[String],
    ) -> Vec<ChatMessage> {
        let mut result = Vec::new();

        // Find the index of the last user message so we can attach images to it
        let last_user_idx = messages.iter().rposition(|m| m.role == MessageRole::User);

        for (idx, message) in messages.iter().enumerate() {
            match message.role {
                MessageRole::Tool => {
                    // Tool result messages: one ChatMessage per ToolResult block
                    for block in &message.blocks {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            output,
                            ..
                        } = block
                        {
                            result.push(ChatMessage {
                                role: "tool".to_string(),
                                content: ChatContent::Text(output.clone()),
                                tool_calls: None,
                                tool_call_id: Some(tool_use_id.clone()),
                            });
                        }
                    }
                },
                MessageRole::Assistant => {
                    let text_parts: String = message
                        .blocks
                        .iter()
                        .filter_map(|block| {
                            if let ContentBlock::Text { text } = block {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    let tool_calls: Vec<ToolCall> = message
                        .blocks
                        .iter()
                        .filter_map(|block| {
                            if let ContentBlock::ToolUse { id, name, input } = block {
                                Some(ToolCall {
                                    id: id.clone(),
                                    call_type: "function".to_string(),
                                    function: ToolCallFunction {
                                        name: name.clone(),
                                        arguments: input.clone(),
                                    },
                                })
                            } else {
                                None
                            }
                        })
                        .collect();

                    result.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: if text_parts.is_empty() && !tool_calls.is_empty() {
                            // Some providers require non-null content even when tool_calls exist
                            ChatContent::Text(String::new())
                        } else {
                            ChatContent::Text(text_parts)
                        },
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        tool_call_id: None,
                    });
                },
                _ => {
                    // User / System messages: simple text conversion
                    let content = message
                        .blocks
                        .iter()
                        .filter_map(|block| {
                            if let ContentBlock::Text { text } = block {
                                Some(text.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    let role_str = match message.role {
                        MessageRole::User => "user",
                        MessageRole::System => "system",
                        _ => "user",
                    };

                    // Inject image attachments into the last user message for multimodal support
                    let chat_content = if role_str == "user"
                        && Some(idx) == last_user_idx
                        && !image_urls.is_empty()
                    {
                        let mut parts = Vec::new();
                        if !content.is_empty() {
                            parts.push(ContentPart {
                                r#type: "text".to_string(),
                                text: Some(content.clone()),
                                image_url: None,
                            });
                        }
                        for url in image_urls {
                            parts.push(ContentPart {
                                r#type: "image_url".to_string(),
                                text: None,
                                image_url: Some(ImageUrl { url: url.clone() }),
                            });
                        }
                        ChatContent::Multipart(parts)
                    } else {
                        ChatContent::Text(content)
                    };

                    result.push(ChatMessage {
                        role: role_str.to_string(),
                        content: chat_content,
                        tool_calls: None,
                        tool_call_id: None,
                    });
                },
            }
        }

        result
    }

    /// Convert AxAgent's ToolCall to Runtime's ContentBlock
    fn convert_tool_call(tool_call: &ToolCall) -> ContentBlock {
        ContentBlock::ToolUse {
            id: tool_call.id.clone(),
            name: tool_call.function.name.clone(),
            input: tool_call.function.arguments.clone(),
        }
    }

    /// Convert AxAgent's TokenUsage to Runtime's TokenUsage
    fn convert_usage(usage: &AxAgentTokenUsage) -> RuntimeTokenUsage {
        RuntimeTokenUsage {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }
}

impl ApiClient for AxAgentApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        // Convert Runtime's ApiRequest to AxAgent's ChatRequest
        let chat_messages = Self::convert_messages(&request.messages, &self.image_urls);

        let chat_request = ChatRequest {
            model: self.model.clone(),
            messages: chat_messages,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            stream: true,
            tools: self.tools.clone(),
            thinking_budget: self.thinking_budget,
            use_max_completion_tokens: self.use_max_completion_tokens,
            thinking_param_style: self.thinking_param_style.clone(),
            api_mode: None,
            instructions: None,
            conversation: None,
            previous_response_id: None,
            store: None,
        };

        // Call AxAgent's provider stream
        let mut stream = self.adapter.chat_stream(&self.ctx, chat_request);
        let mut events = Vec::new();
        let on_event = self.on_event.clone();

        let process_stream = async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        if let Some(ref text) = chunk.content {
                            if !text.is_empty() {
                                let event = AssistantEvent::TextDelta(text.clone());
                                if let Some(ref cb) = on_event {
                                    cb(&event);
                                }
                                events.push(event);
                            }
                        }

                        if let Some(ref thinking) = chunk.thinking {
                            if !thinking.is_empty() {
                                let event = AssistantEvent::ThinkingDelta(thinking.clone());
                                if let Some(ref cb) = on_event {
                                    cb(&event);
                                }
                                events.push(event);
                            }
                        }

                        if let Some(ref tool_calls) = chunk.tool_calls {
                            for tool_call in tool_calls {
                                let tool_use = Self::convert_tool_call(tool_call);
                                if let ContentBlock::ToolUse { id, name, input } = tool_use {
                                    let event = AssistantEvent::ToolUse { id, name, input };
                                    if let Some(ref cb) = on_event {
                                        cb(&event);
                                    }
                                    events.push(event);
                                }
                            }
                        }

                        if let Some(ref usage) = chunk.usage {
                            let runtime_usage = Self::convert_usage(usage);
                            let event = AssistantEvent::Usage(runtime_usage);
                            if let Some(ref cb) = on_event {
                                cb(&event);
                            }
                            events.push(event);
                        }

                        if chunk.done {
                            let event = AssistantEvent::MessageStop;
                            if let Some(ref cb) = on_event {
                                cb(&event);
                            }
                            events.push(event);
                            break;
                        }
                    },
                    Err(e) => {
                        return Err(RuntimeError::new(e.to_string()));
                    },
                }
            }

            Ok(events)
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.block_on(process_stream)
        } else {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(process_stream)
        }
    }
}
