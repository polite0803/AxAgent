use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallContext {
    pub tool_name: String,
    pub tool_namespace: Option<String>,
    pub arguments: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallContext {
    pub model: String,
    pub message_count: usize,
    pub tool_count: usize,
    pub estimated_tokens: Option<u64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCallResult {
    pub content: String,
    pub tool_calls: Option<Vec<String>>,
    pub usage_prompt_tokens: Option<u32>,
    pub usage_completion_tokens: Option<u32>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub hook_name: String,
    pub session_id: Option<String>,
    pub metadata: serde_json::Value,
}

impl HookContext {
    pub fn new(hook_name: &str) -> Self {
        Self {
            hook_name: hook_name.to_string(),
            session_id: None,
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookDecision {
    Allow,
    Veto { reason: String },
    Modify { changes: serde_json::Value },
}

#[async_trait]
pub trait PluginHook: Send + Sync {
    fn name(&self) -> &str;

    fn priority(&self) -> i32 {
        0
    }

    async fn on_session_start(&self, _session_id: &str) {}
    async fn on_session_end(&self, _session_id: &str) {}

    async fn pre_tool_call(&self, _ctx: &ToolCallContext) -> Option<HookDecision> {
        None
    }

    async fn post_tool_call(&self, _ctx: &ToolCallContext, _result: &ToolCallResult) {}

    async fn transform_tool_result(
        &self,
        _tool_name: &str,
        result: serde_json::Value,
    ) -> Option<serde_json::Value> {
        Some(result)
    }

    async fn pre_llm_call(&self, _ctx: &LlmCallContext) -> Option<HookDecision> {
        None
    }

    async fn post_llm_call(&self, _ctx: &LlmCallContext, _result: &LlmCallResult) {}

    async fn transform_llm_response(&self, content: String) -> String {
        content
    }

    async fn transform_terminal_output(&self, output: String) -> String {
        output
    }

    async fn on_error(&self, _error: &str, _context: Option<serde_json::Value>) {}
}

pub type SharedHook = Arc<dyn PluginHook>;
