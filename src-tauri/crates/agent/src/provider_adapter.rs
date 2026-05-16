//! AxAgent Provider Adapter for ClawCode Runtime

use axagent_core::types::{
    ChatContent, ChatMessage, ChatRequest, ChatTool, ContentPart, ImageUrl,
    TokenUsage as AxAgentTokenUsage, ToolCall, ToolCallFunction,
};
use axagent_providers::{ProviderAdapter, ProviderRequestContext};
use axagent_runtime_core::{
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
    /// Delay in milliseconds before each API request, used to avoid rate limits.
    request_delay_ms: Option<u64>,
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
            request_delay_ms: None,
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
            request_delay_ms: None,
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

    /// Set request delay in milliseconds (applied before each API call to avoid rate limits).
    pub fn with_request_delay_ms(mut self, request_delay_ms: Option<u64>) -> Self {
        self.request_delay_ms = request_delay_ms;
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
    /// Extract thinking content from text that may contain `<think data-axagent="1">...</think>` tags.
    /// The runtime's `build_assistant_message` wraps thinking in `<think data-axagent="1">` tags.
    /// Returns (cleaned_text, extracted_thinking).
    fn extract_thinking_from_text(text: &str) -> (String, Option<String>) {
        const THINK_START_TAG: &str = "<think data-axagent=\"1\">";
        const THINK_END_TAG: &str = "</think>";

        if let Some(start) = text.find(THINK_START_TAG) {
            let after_start = &text[start + THINK_START_TAG.len()..];
            if let Some(end) = after_start.find(THINK_END_TAG) {
                let thinking = after_start[..end].trim().to_string();
                let thinking = if thinking.is_empty() {
                    None
                } else {
                    Some(thinking)
                };
                // Everything before <think> tag + everything after </think> tag
                let before = &text[..start];
                let after = &after_start[end + THINK_END_TAG.len()..];
                let cleaned = format!("{}{}", before, after).trim().to_string();
                return (cleaned, thinking);
            }
        }
        (text.to_string(), None)
    }

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
                                thinking: None,
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

                    // Extract thinking from <think data-axagent="1"> tags embedded by
                    // the runtime's build_assistant_message, so it flows through
                    // ChatMessage.thinking → OpenAIMessage.reasoning_content.
                    let (clean_text, extracted_thinking) =
                        Self::extract_thinking_from_text(&text_parts);

                    result.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: ChatContent::Text(clean_text),
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        tool_call_id: None,
                        thinking: extracted_thinking,
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
                        thinking: None,
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
        // Apply request delay to avoid rate limits
        if let Some(delay_ms) = self.request_delay_ms {
            if delay_ms > 0 {
                let delay = std::time::Duration::from_millis(delay_ms);
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.block_on(tokio::time::sleep(delay));
                } else {
                    std::thread::sleep(delay);
                }
            }
        }

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
        let mut stream = self.adapter.chat_stream(&self.ctx, chat_request, None);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_messages() -> Vec<ConversationMessage> {
        vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
            usage: None,
        }]
    }

    #[test]
    fn test_convert_messages_user() {
        let messages = make_test_messages();
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
    }

    #[test]
    fn test_convert_messages_system() {
        let messages = vec![ConversationMessage {
            role: MessageRole::System,
            blocks: vec![ContentBlock::Text {
                text: "You are helpful".to_string(),
            }],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "system");
    }

    #[test]
    fn test_convert_messages_assistant_with_text() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::Text {
                text: "Hi there".to_string(),
            }],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "assistant");
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "Hi there"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_convert_messages_assistant_with_tool_use() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "Let me check".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "search".to_string(),
                    input: "{}".to_string(),
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].tool_calls.is_some());
        let tool_calls = result[0].tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "search");
    }

    #[test]
    fn test_convert_messages_tool_result() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                tool_name: "search".to_string(),
                output: "result data".to_string(),
                is_error: false,
            }],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "tool");
        assert_eq!(result[0].tool_call_id, Some("call_1".to_string()));
    }

    #[test]
    fn test_convert_messages_with_image_urls() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Describe this image".to_string(),
            }],
            usage: None,
        }];
        let image_urls = vec!["data:image/png;base64,abc".to_string()];
        let result = AxAgentApiClient::convert_messages(&messages, &image_urls);
        assert_eq!(result.len(), 1);
        match &result[0].content {
            ChatContent::Multipart(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].r#type, "text");
                assert_eq!(parts[1].r#type, "image_url");
            },
            _ => panic!("Expected multipart content"),
        }
    }

    #[test]
    fn test_convert_messages_image_urls_only_on_last_user() {
        let messages = vec![
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "First".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "Second".to_string(),
                }],
                usage: None,
            },
        ];
        let image_urls = vec!["data:image/png;base64,abc".to_string()];
        let result = AxAgentApiClient::convert_messages(&messages, &image_urls);
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "First"),
            _ => panic!("Expected text content for first message"),
        }
        match &result[1].content {
            ChatContent::Multipart(_) => {},
            _ => panic!("Expected multipart content for last user message"),
        }
    }

    #[test]
    fn test_convert_tool_call() {
        let tool_call = ToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "search".to_string(),
                arguments: "{\"q\": \"test\"}".to_string(),
            },
        };
        let block = AxAgentApiClient::convert_tool_call(&tool_call);
        match block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "search");
                assert_eq!(input, "{\"q\": \"test\"}");
            },
            _ => panic!("Expected ToolUse block"),
        }
    }

    #[test]
    fn test_convert_usage() {
        let usage = AxAgentTokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let runtime_usage = AxAgentApiClient::convert_usage(&usage);
        assert_eq!(runtime_usage.input_tokens, 100);
        assert_eq!(runtime_usage.output_tokens, 50);
    }

    #[test]
    fn test_convert_messages_assistant_tool_calls_only() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![ContentBlock::ToolUse {
                id: "call_1".to_string(),
                name: "tool".to_string(),
                input: "{}".to_string(),
            }],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        match &result[0].content {
            ChatContent::Text(t) => assert!(t.is_empty()),
            _ => panic!("Expected empty text content"),
        }
        assert!(result[0].tool_calls.is_some());
    }

    #[test]
    fn test_convert_messages_multiple_tool_results() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![
                ContentBlock::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    tool_name: "search".to_string(),
                    output: "result1".to_string(),
                    is_error: false,
                },
                ContentBlock::ToolResult {
                    tool_use_id: "call_2".to_string(),
                    tool_name: "read".to_string(),
                    output: "result2".to_string(),
                    is_error: false,
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "tool");
        assert_eq!(result[0].tool_call_id, Some("call_1".to_string()));
        assert_eq!(result[1].role, "tool");
        assert_eq!(result[1].tool_call_id, Some("call_2".to_string()));
    }

    #[test]
    fn test_convert_messages_assistant_text_and_tool_use() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::Text {
                    text: "Let me search".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "search".to_string(),
                    input: "{\"q\":\"test\"}".to_string(),
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "Let me search"),
            _ => panic!("Expected text content"),
        }
        let tool_calls = result[0].tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, "call_1");
        assert_eq!(tool_calls[0].call_type, "function");
        assert_eq!(tool_calls[0].function.name, "search");
        assert_eq!(tool_calls[0].function.arguments, "{\"q\":\"test\"}");
    }

    #[test]
    fn test_convert_messages_multiple_tool_calls() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Assistant,
            blocks: vec![
                ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "search".to_string(),
                    input: "{}".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "call_2".to_string(),
                    name: "read".to_string(),
                    input: "{}".to_string(),
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        let tool_calls = result[0].tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 2);
    }

    #[test]
    fn test_convert_messages_system_text() {
        let messages = vec![ConversationMessage {
            role: MessageRole::System,
            blocks: vec![ContentBlock::Text {
                text: "You are helpful".to_string(),
            }],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result[0].role, "system");
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "You are helpful"),
            _ => panic!("Expected text content"),
        }
        assert!(result[0].tool_calls.is_none());
        assert!(result[0].tool_call_id.is_none());
    }

    #[test]
    fn test_convert_messages_image_urls_with_empty_text() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: String::new(),
            }],
            usage: None,
        }];
        let image_urls = vec!["data:image/png;base64,abc".to_string()];
        let result = AxAgentApiClient::convert_messages(&messages, &image_urls);
        match &result[0].content {
            ChatContent::Multipart(parts) => {
                assert_eq!(parts.len(), 1);
                assert_eq!(parts[0].r#type, "image_url");
            },
            _ => panic!("Expected multipart content"),
        }
    }

    #[test]
    fn test_convert_messages_image_urls_multiple() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text {
                text: "Compare these".to_string(),
            }],
            usage: None,
        }];
        let image_urls = vec![
            "data:image/png;base64,img1".to_string(),
            "data:image/png;base64,img2".to_string(),
        ];
        let result = AxAgentApiClient::convert_messages(&messages, &image_urls);
        match &result[0].content {
            ChatContent::Multipart(parts) => {
                assert_eq!(parts.len(), 3);
                assert_eq!(parts[0].r#type, "text");
                assert_eq!(parts[1].r#type, "image_url");
                assert_eq!(parts[2].r#type, "image_url");
            },
            _ => panic!("Expected multipart content"),
        }
    }

    #[test]
    fn test_convert_messages_no_image_urls_on_system() {
        let messages = vec![ConversationMessage {
            role: MessageRole::System,
            blocks: vec![ContentBlock::Text {
                text: "System prompt".to_string(),
            }],
            usage: None,
        }];
        let image_urls = vec!["data:image/png;base64,abc".to_string()];
        let result = AxAgentApiClient::convert_messages(&messages, &image_urls);
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "System prompt"),
            _ => panic!("Expected text content, not multipart"),
        }
    }

    #[test]
    fn test_convert_tool_call_fields() {
        let tool_call = ToolCall {
            id: "call_abc".to_string(),
            call_type: "function".to_string(),
            function: ToolCallFunction {
                name: "execute".to_string(),
                arguments: "{\"cmd\":\"ls\"}".to_string(),
            },
        };
        let block = AxAgentApiClient::convert_tool_call(&tool_call);
        match block {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "execute");
                assert_eq!(input, "{\"cmd\":\"ls\"}");
            },
            _ => panic!("Expected ToolUse block"),
        }
    }

    #[test]
    fn test_convert_usage_fields() {
        let usage = AxAgentTokenUsage {
            prompt_tokens: 500,
            completion_tokens: 250,
            total_tokens: 750,
        };
        let runtime_usage = AxAgentApiClient::convert_usage(&usage);
        assert_eq!(runtime_usage.input_tokens, 500);
        assert_eq!(runtime_usage.output_tokens, 250);
        assert_eq!(runtime_usage.cache_creation_input_tokens, 0);
        assert_eq!(runtime_usage.cache_read_input_tokens, 0);
    }

    #[test]
    fn test_convert_messages_mixed_conversation() {
        let messages = vec![
            ConversationMessage {
                role: MessageRole::System,
                blocks: vec![ContentBlock::Text {
                    text: "System".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: "Hello".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::Text {
                    text: "Hi".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::Assistant,
                blocks: vec![ContentBlock::ToolUse {
                    id: "call_1".to_string(),
                    name: "tool".to_string(),
                    input: "{}".to_string(),
                }],
                usage: None,
            },
            ConversationMessage {
                role: MessageRole::Tool,
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    tool_name: "tool".to_string(),
                    output: "done".to_string(),
                    is_error: false,
                }],
                usage: None,
            },
        ];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
        assert_eq!(result[3].role, "assistant");
        assert_eq!(result[4].role, "tool");
    }

    #[test]
    fn test_convert_messages_tool_result_ignores_non_tool_result_blocks() {
        let messages = vec![ConversationMessage {
            role: MessageRole::Tool,
            blocks: vec![
                ContentBlock::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    tool_name: "tool".to_string(),
                    output: "result".to_string(),
                    is_error: false,
                },
                ContentBlock::Text {
                    text: "extra text".to_string(),
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "tool");
    }

    #[test]
    fn test_convert_messages_user_with_multiple_text_blocks() {
        let messages = vec![ConversationMessage {
            role: MessageRole::User,
            blocks: vec![
                ContentBlock::Text {
                    text: "Hello".to_string(),
                },
                ContentBlock::Text {
                    text: " World".to_string(),
                },
            ],
            usage: None,
        }];
        let result = AxAgentApiClient::convert_messages(&messages, &[]);
        match &result[0].content {
            ChatContent::Text(t) => assert_eq!(t, "Hello World"),
            _ => panic!("Expected text content"),
        }
    }
}
