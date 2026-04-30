pub mod anthropic;
pub mod chat_completions;
pub mod responses;

pub use anthropic::AnthropicTransport;
pub use chat_completions::ChatCompletionsTransport;
pub use responses::ResponsesTransport;

use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TransportRequest {
    pub model: String,
    pub messages: Vec<TransportMessage>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<serde_json::Value>,
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct TransportMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransportResponse {
    pub content: String,
    pub tool_calls: Option<Vec<TransportToolCall>>,
    pub usage: TransportUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransportToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Default)]
pub struct TransportUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[async_trait]
pub trait TransportProvider: Send + Sync {
    fn provider_name(&self) -> &'static str;

    async fn send(
        &self,
        request: TransportRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> anyhow::Result<TransportResponse>;

    async fn send_streaming(
        &self,
        request: TransportRequest,
        api_key: &str,
        base_url: Option<&str>,
    ) -> anyhow::Result<
        Box<dyn futures::Stream<Item = anyhow::Result<TransportStreamChunk>> + Send + Unpin>,
    >;
}

#[derive(Debug, Clone)]
pub struct TransportStreamChunk {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<TransportToolCall>>,
    pub finish_reason: Option<String>,
    pub usage: Option<TransportUsage>,
}
