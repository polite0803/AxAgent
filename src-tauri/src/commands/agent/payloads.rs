//! Payload types for agent Tauri events and commands.
//! Extracted from mod.rs to reduce file size.

use axagent_harness::types::AttachmentInput;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 前端注入的 Agent 上下文 — 供后端构建系统提示
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentContextPayload {
    /// 当前页面标识（如 "settings", "chat"）
    pub page: String,
    /// 当前页面 URL 或路由路径
    pub url: String,
    /// 页面暴露给 Agent 的快捷操作列表
    #[serde(default)]
    pub quick_actions: Vec<AgentQuickActionPayload>,
    /// 页面数据快照
    #[serde(default)]
    pub data: Option<Value>,
}

/// 前端快捷操作定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentQuickActionPayload {
    /// 操作唯一标识符
    pub id: String,
    /// 操作描述
    pub description: String,
    /// 是否需要用户确认
    #[serde(default)]
    pub require_confirmation: bool,
}

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
#[serde(rename_all = "camelCase")]
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
    /// Agent profile ID from the agent_profiles table. AgentProfile 是 Expert（技能）
    /// 和 AgentRole（岗位）的统一组装体，是 Agent 的唯一入口。
    #[serde(rename = "agentProfileId")]
    pub agent_profile_id: Option<String>,
    /// 动态匹配的专家（expert_id → agency_experts）。当命中的执行载体（如角色护照
    /// 落到只有角色、未组合专家的 role-bridge）时，由认知编排层通过 RAR 检索动态
    /// 补全，使"角色 + 专家"运行时组合生效；覆盖 profile 自带 expert_id。
    #[serde(rename = "expertId")]
    pub expert_id: Option<String>,
    /// 前端注入的页面上下文 — 供 Agent 理解当前环境
    #[serde(rename = "agentContext")]
    pub agent_context: Option<AgentContextPayload>,
    /// 认知编排器决策的执行模式（ask / act / delegate 等），供 agent 运行时感知
    /// 当前编排模式以调整行为；直连 agent（非认知编排）调用时缺省为 None。
    #[serde(rename = "executionMode", default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<String>,
    /// P0: 任务形态决策（原则三标尺输出）。
    /// 当 UNITY_P0_TASK_SHAPE 启用时，由 TaskShapeClassifier 在路由前产出，
    /// 运行时据此覆盖会话权限初值（按任务而非按会话）。
    /// `None` 表示未启用 flag 或直连 agent 调用。
    #[serde(rename = "taskShape", default, skip_serializing_if = "Option::is_none")]
    pub task_shape: Option<axagent_harness::TaskShapeDecision>,
    /// 认知编排按需注入的工具名列表（Phase 1.5 暴露闭环）：
    /// 主动模式（execution_mode=Some）下，命中能力的真实工具定义凭此注入 chat_tools，
    /// 解决此前"主动模式工具列表为空、发现的能力执行不了"的执行断链。
    #[serde(rename = "extraTools", default, skip_serializing_if = "Option::is_none")]
    pub extra_tools: Option<Vec<String>>,
    /// 认知编排按需加载的技能名列表（遗留边界①补充）：
    /// 主动模式（execution_mode=Some）下，命中 Skill 护照时按名加载该技能
    /// （skill_tools 工具定义 + 注册执行 handler），解决"主动模式技能不可用"。
    /// 与 extra_tools 的区别：技能需注册 handler（load_skill_tools + register_skill_tool），
    /// 不能仅注入 schema（否则 LLM 调用 skill_xxx 会 404）。
    #[serde(rename = "extraSkills", default, skip_serializing_if = "Option::is_none")]
    pub extra_skills: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentOptions {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
    /// 禁用的工具名称列表（例如 ["Bash", "WebSearch"]），
    /// 这些工具不会传给 LLM 也不会被执行。
    #[serde(rename = "disabledTools")]
    pub disabled_tools: Option<Vec<String>>,
    /// 活跃功能域列表（例如 ["General"]），
    /// 仅该域内的工具会传给 LLM。默认 ["General"]。
    #[serde(rename = "activeDomains")]
    pub active_domains: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct AgentQueryResponse {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    #[serde(rename = "assistantMessageId")]
    pub assistant_message_id: String,
    /// P0-2：计划确认被拒绝时返回 "rejected"，正常执行时 None。
    pub status: Option<String>,
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
pub struct AgentApprovePlanRequest {
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// 用户决策："approve" 批准执行，"reject" 拒绝执行。
    pub decision: String,
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
    #[serde(rename = "workspaceUri")]
    pub workspace_uri: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentEnsureWorkspaceResponse {
    #[serde(rename = "workspacePath")]
    pub workspace_path: String,
}
