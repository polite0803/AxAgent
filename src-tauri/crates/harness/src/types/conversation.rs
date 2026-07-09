// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};

use super::provider_model::deserialize_double_option;
use super::rag_voice_etc::SourceRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// A structured content block in a message (Part-based model).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: String },
    #[serde(rename = "tool_result")]
    ToolResult { tool_use_id: String, tool_name: String, output: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct MessagePage {
    pub messages: Vec<Message>,
    pub has_older: bool,
    pub oldest_message_id: Option<String>,
    pub total_active_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
