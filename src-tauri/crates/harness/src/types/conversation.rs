// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use super::provider_model::deserialize_double_option;
use super::rag_voice_etc::SourceRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub model_id: String,
    pub provider_id: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub search_enabled: bool,
    pub search_provider_id: Option<String>,
    pub thinking_budget: Option<i64>,
    pub enabled_mcp_server_ids: Vec<String>,
    pub enabled_knowledge_base_ids: Vec<String>,
    pub enabled_memory_namespace_ids: Vec<String>,
    pub enabled_wiki_ids: Vec<String>,
    pub message_count: u32,
    pub is_pinned: bool,
    pub is_archived: bool,
    pub context_compression: bool,
    pub category_id: Option<String>,
    pub parent_conversation_id: Option<String>,
    pub mode: String,
    pub work_strategy: Option<String>,
    pub scenario: Option<String>,
    pub workspace_dir: Option<String>,
    pub enabled_skill_ids: Vec<String>,
    pub agent_profile_id: Option<String>,
    pub workflow_template_id: Option<String>,
    pub session_type: String,
    pub workflow_status: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Conversation {
    // Business methods extracted to ConversationSourceResolver below.
}

/// Resolver for Conversation enabled sources (knowledge / memory / wiki).
pub struct ConversationSourceResolver;

impl ConversationSourceResolver {
    pub fn enabled_sources(conversation: &Conversation) -> Vec<SourceRef> {
        let mut sources = Vec::new();
        for id in &conversation.enabled_knowledge_base_ids {
            sources.push(SourceRef::knowledge(id));
        }
        for id in &conversation.enabled_memory_namespace_ids {
            sources.push(SourceRef::memory(id));
        }
        for id in &conversation.enabled_wiki_ids {
            sources.push(SourceRef::wiki(id));
        }
        sources
    }

    pub fn set_enabled_sources(conversation: &mut Conversation, sources: &[SourceRef]) {
        conversation.enabled_knowledge_base_ids = sources
            .iter()
            .filter(|s| s.container_type == "knowledge")
            .map(|s| s.id.clone())
            .collect();
        conversation.enabled_memory_namespace_ids =
            sources.iter().filter(|s| s.container_type == "memory").map(|s| s.id.clone()).collect();
        conversation.enabled_wiki_ids =
            sources.iter().filter(|s| s.container_type == "wiki").map(|s| s.id.clone()).collect();
    }

    pub fn source_ids_by_type(conversation: &Conversation, container_type: &str) -> Vec<String> {
        match container_type {
            "knowledge" => conversation.enabled_knowledge_base_ids.clone(),
            "memory" => conversation.enabled_memory_namespace_ids.clone(),
            "wiki" => conversation.enabled_wiki_ids.clone(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub token_count: Option<u32>,
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub attachments: Vec<Attachment>,
    pub thinking: Option<String>,
    pub created_at: i64,
    pub parent_message_id: Option<String>,
    pub version_index: i32,
    pub is_active: bool,
    pub tool_calls_json: Option<String>,
    pub tool_call_id: Option<String>,
    pub status: String,
    pub tokens_per_second: Option<f64>,
    pub first_token_latency_ms: Option<i64>,
    pub cache_creation_tokens: Option<u32>,
    pub cache_read_tokens: Option<u32>,
    /// Structured content blocks (JSON-encoded ContentBlock[]).
    pub parts: Option<String>,
    /// Parsed content blocks for frontend consumption.
    #[serde(default)]
    pub blocks: Option<Vec<ContentBlock>>,
    /// 引用回复：被引用消息的 ID（区别于 parent_message_id 的多版本语义）
    #[serde(default)]
    pub quoted_message_id: Option<String>,
    /// 认知编排决策标签：该消息对应一轮执行的决策信息（ExecutionMode / 路由路径 / 命中工作流 / 专家等）
    #[serde(default)]
    pub decision: Option<serde_json::Value>,
}

/// A structured content block in a message (Part-based model).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: String },
    /// 字段 camelCase（与前端 `src/types` ContentBlock 及 agent_query 落库的
    /// parts JSON 对齐）；snake_case 保留为 alias 兼容潜在旧数据。
    #[serde(rename = "tool_result")]
    #[serde(rename_all = "camelCase")]
    ToolResult {
        #[serde(alias = "tool_use_id")]
        tool_use_id: String,
        #[serde(alias = "tool_name")]
        tool_name: String,
        output: String,
        #[serde(alias = "is_error")]
        is_error: bool,
    },
}

impl From<crate::conversation_model::ContentBlock> for ContentBlock {
    fn from(block: crate::conversation_model::ContentBlock) -> Self {
        match block {
            crate::conversation_model::ContentBlock::Text { text } => ContentBlock::Text { text },
            crate::conversation_model::ContentBlock::ToolUse { id, name, input } => {
                ContentBlock::ToolUse { id, name, input }
            },
            crate::conversation_model::ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => ContentBlock::ToolResult { tool_use_id, tool_name, output, is_error },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationStats {
    pub total_messages: u64,
    pub total_user_messages: u64,
    pub total_assistant_messages: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub avg_tokens_per_second: Option<f64>,
    pub avg_first_token_latency_ms: Option<f64>,
    pub avg_response_time_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub message_count: u64,
    pub total_prompt_tokens: u64,
    pub total_completion_tokens: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostByProvider {
    pub provider_id: String,
    pub request_count: u64,
    pub token_count: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessagePage {
    pub messages: Vec<Message>,
    pub has_older: bool,
    pub oldest_message_id: Option<String>,
    pub total_active_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    #[serde(default)]
    pub id: String,
    pub file_type: String,
    pub file_name: String,
    #[serde(default)]
    pub file_path: String,
    pub file_size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInput {
    pub file_name: String,
    pub file_type: String,
    pub file_size: u64,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub conversation: Conversation,
    pub matched_message_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub conversation_id: String,
    pub summary_text: String,
    pub compressed_until_message_id: Option<String>,
    pub token_count: Option<u32>,
    pub model_used: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConversationInput {
    pub title: Option<String>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub is_pinned: Option<bool>,
    pub is_archived: Option<bool>,
    pub system_prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub temperature: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub max_tokens: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub top_p: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub frequency_penalty: Option<Option<f64>>,
    pub search_enabled: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub search_provider_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub thinking_budget: Option<Option<i64>>,
    pub enabled_mcp_server_ids: Option<Vec<String>>,
    pub enabled_knowledge_base_ids: Option<Vec<String>>,
    pub enabled_memory_namespace_ids: Option<Vec<String>>,
    pub enabled_wiki_ids: Option<Vec<String>>,
    pub context_compression: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub category_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub parent_conversation_id: Option<Option<String>>,
    pub mode: Option<String>,
    pub work_strategy: Option<Option<String>>,
    pub scenario: Option<String>,
    pub enabled_skill_ids: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub agent_profile_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub workflow_template_id: Option<Option<String>>,
    pub session_type: Option<String>,
    pub workflow_status: Option<Option<String>>,
}

impl UpdateConversationInput {
    pub fn enabled_sources(&self) -> Vec<SourceRef> {
        let mut sources = Vec::new();
        if let Some(ids) = &self.enabled_knowledge_base_ids {
            for id in ids {
                sources.push(SourceRef::knowledge(id));
            }
        }
        if let Some(ids) = &self.enabled_memory_namespace_ids {
            for id in ids {
                sources.push(SourceRef::memory(id));
            }
        }
        if let Some(ids) = &self.enabled_wiki_ids {
            for id in ids {
                sources.push(SourceRef::wiki(id));
            }
        }
        sources
    }
}

// ── 2.6 P1:会话级快照/回滚 DTO ──

/// 会话工作区快照 — 持久化于 `conversations.workspace_snapshot_json`。
///
/// `branches` 与 `active_branch_id` 在 `get_workspace_snapshot` 时由
/// `conversation_branches` 表实时拼装,不参与 `workspace_snapshot_json` 持久化
/// (避免分支增减后快照与表数据脱节)。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSnapshot {
    /// 启用的知识库 / memory / wiki ID(由 Conversation.enabled_*_ids 镜像)
    #[serde(default)]
    pub context_sources: Vec<String>,
    /// 当前会话激活的工具 ID(MCP server / skill)
    #[serde(default)]
    pub active_tools: Vec<String>,
    /// 知识库绑定(包含绑定元数据,如检索阈值、rerank 策略)
    #[serde(default)]
    pub knowledge_bindings: Vec<KnowledgeBinding>,
    /// 记忆策略(抽屉化、衰减、外溢阈值等)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_policy: Option<serde_json::Value>,
    /// 搜索策略(provider / top_k / 时间范围等)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_policy: Option<serde_json::Value>,
    /// 会话产出物(artifacts)的引用列表
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
    /// 当前激活的分支 ID(镜像自 `conversations.active_branch_id`,便于前端直接消费)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_branch_id: Option<String>,
    /// 分支列表(运行时由 conversation_branches 表拼装,不参与持久化)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<super::rag_voice_etc::ConversationBranch>,
}

/// 知识库绑定元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBinding {
    pub source_id: String,
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rerank: Option<bool>,
}

/// 产出物引用
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

/// 分支对比结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchComparison {
    pub branch_a: super::rag_voice_etc::ConversationBranch,
    pub branch_b: super::rag_voice_etc::ConversationBranch,
    /// 两条分支共享的前缀消息(从会话起点到分叉点)
    pub common_prefix: Vec<MessageSummary>,
    /// 仅在 branch_a 中存在的消息
    pub only_in_a: Vec<MessageSummary>,
    /// 仅在 branch_b 中存在的消息
    pub only_in_b: Vec<MessageSummary>,
    /// 分叉点消息 ID(branch_a 与 branch_b 的最近公共父消息)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diverge_at: Option<String>,
}

/// 消息摘要(用于分支对比,避免传输完整 content)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageSummary {
    pub id: String,
    pub role: String,
    /// content 前 200 字符
    pub content_preview: String,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationCategory {
    pub id: String,
    pub name: String,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub system_prompt: Option<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub default_temperature: Option<f64>,
    pub default_max_tokens: Option<i64>,
    pub default_top_p: Option<f64>,
    pub default_frequency_penalty: Option<f64>,
    pub sort_order: i32,
    pub is_collapsed: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationCategoryInput {
    pub name: String,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub system_prompt: Option<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub default_temperature: Option<f64>,
    pub default_max_tokens: Option<i64>,
    pub default_top_p: Option<f64>,
    pub default_frequency_penalty: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConversationCategoryInput {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub icon_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub icon_value: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub system_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_provider_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_model_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_temperature: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_max_tokens: Option<Option<i64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_top_p: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub default_frequency_penalty: Option<Option<f64>>,
}

// === Gateway System ===
