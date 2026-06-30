//! Payload types for agent Tauri events and commands.
//! Extracted from mod.rs to reduce file size.

use axagent_harness::types::AttachmentInput;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------

/// Agent 运行阶段状态，前端据此更新加载提示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// 当前阶段: "init" | "setup" | "running" | "done" | "error"
    pub phase: String,
    pub message: String,
    /// 消息的错误码，用于前端i18n翻译查询
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDonePayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    pub text: String,
    pub thinking: Option<String>,
    pub usage: Option<AgentUsagePayload>,
    #[serde(rename = "numTurns")]
    pub num_turns: Option<u32>,
    #[serde(rename = "costUsd")]
    pub cost_usd: Option<f64>,
    /// Structured content blocks from the agent session (short-term Part-based model).
    pub blocks: Option<Vec<AgentContentBlock>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<String>,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: Option<String>,
    #[serde(rename = "toolName")]
    pub tool_name: Option<String>,
    pub output: Option<String>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsagePayload {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentErrorPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: Option<String>,
    pub message: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolStartPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub input: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolUsePayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub input: Value,
    #[serde(rename = "executionId")]
    pub execution_id: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolResultPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub input: Value,
    pub output: String,
    pub is_error: bool,
    #[serde(rename = "executionId")]
    pub execution_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamTextPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStreamThinkingPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    pub thinking: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCachePayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    pub unexpected: bool,
    pub reason: String,
    #[serde(rename = "cacheReadTokens")]
    pub cache_read_input_tokens: u32,
    #[serde(rename = "tokenDrop")]
    pub token_drop: u32,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPermissionPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    pub input: Value,
    #[serde(rename = "riskLevel")]
    pub risk_level: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentCardPayload {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "agentType")]
    pub agent_type: String,
    #[serde(rename = "agentName")]
    pub agent_name: String,
    pub description: String,
    pub status: String,
    #[serde(rename = "childConversationId")]
    pub child_conversation_id: Option<String>,
    #[serde(rename = "childSessionId")]
    pub child_session_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Request/response types for Tauri commands
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct AgentQueryRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub input: String,
    #[serde(rename = "providerId")]
    pub provider_id: String,
    #[serde(rename = "model_id")]
    pub model_id: String,
    #[serde(rename = "enabledMcpServerIds")]
    pub enabled_mcp_server_ids: Option<Vec<String>>,
    #[serde(rename = "enabledKnowledgeBaseIds")]
    pub enabled_knowledge_base_ids: Option<Vec<String>>,
    #[serde(rename = "enabledMemoryNamespaceIds")]
    pub enabled_memory_namespace_ids: Option<Vec<String>>,
    #[serde(rename = "enabledWikiIds")]
    pub enabled_wiki_ids: Option<Vec<String>>,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    #[serde(rename = "thinkingBudget")]
    pub thinking_budget: Option<u32>,
    /// ID of the search provider to enable web search for this agent session.
    /// 配置通过 UnifiedToolRegistry.tool_extra 传递给 WebSearchTool。
    #[serde(rename = "searchProviderId")]
    pub search_provider_id: Option<String>,
    /// Attachments (images, files) to include with the user message.
    /// Images are described in the system prompt since the runtime currently
    /// only supports text input.
    pub attachments: Option<Vec<AttachmentInput>>,
    pub options: Option<AgentOptions>,
    /// Agent role for role-based tool filtering and system prompt selection.
    /// When set, only tools matching the role's `default_tools()` are exposed
    /// to the LLM, and the role's system prompt is prepended.
    pub role: Option<String>,
    /// Agent profile ID from the agent_profiles table. AgentProfile 是 Expert（技能）
    /// 和 AgentRole（岗位）的统一组装体，是 Agent 的唯一入口。
    #[serde(rename = "agentProfileId")]
    pub agent_profile_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentOptions {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    /// 禁用的工具名称列表（例如 ["Bash", "WebSearch"]），
    /// 这些工具不会传给 LLM 也不会被执行。
    #[serde(rename = "disabledTools")]
    pub disabled_tools: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AgentQueryResponse {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentApproveRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: String,
    pub decision: String,
    #[serde(rename = "toolName")]
    pub tool_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentRespondAskRequest {
    #[serde(rename = "askId")]
    pub ask_id: String,
    pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct AgentCancelRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
}

pub type AgentApproveResponse = ();

pub type AgentCancelResponse = ();

#[derive(Debug, Deserialize)]
pub struct AgentUpdateSessionRequest {
    #[serde(alias = "conversation_id", rename = "conversationId")]
    pub conversation_id: String,
    pub name: Option<String>,
    pub metadata: Option<Value>,
    pub cwd: Option<String>,
    #[serde(alias = "permission_mode", rename = "permissionMode")]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentUpdateSessionResponse {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub name: Option<String>,
    pub metadata: Option<Value>,
    pub cwd: Option<String>,
    #[serde(rename = "permissionMode")]
    pub permission_mode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentGetSessionRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
}

#[derive(Debug, Serialize)]
pub struct AgentGetSessionResponse {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    pub name: Option<String>,
    pub metadata: Option<Value>,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    #[serde(rename = "lastActiveAt")]
    pub last_active_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct AgentEnsureWorkspaceRequest {
    #[serde(rename = "conversationId")]
    #[allow(dead_code)]
    pub conversation_id: String,
    #[serde(rename = "workspaceUri")]
    pub workspace_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentEnsureWorkspaceResponse {
    #[serde(rename = "workspacePath")]
    pub workspace_path: String,
}
