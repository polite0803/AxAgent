// SPDX-License-Identifier: AGPL-3.0-only

//! 仓储操作 DTO —— 消除 consumer→entities 的架构违规。
//! 这些 DTO 是 entities 模型在 harness 层的等价结构体，consumer
//! 通过 trait 方法接收/返回 DTO，不直接引用 `axagent_entities`。

use serde::{Deserialize, Serialize};

/// 笔记 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub wiki_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Option<String>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub is_processed: bool,
}

/// 创建笔记的输入 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNoteInput {
    pub title: String,
    pub content: String,
    pub wiki_id: String,
    pub tags: Option<String>,
    pub source: Option<String>,
    pub url: Option<String>,
}

/// 更新笔记的输入 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNoteInput {
    pub id: String,
    pub content: Option<String>,
    pub tags: Option<String>,
    pub is_processed: Option<bool>,
}

/// Wiki 页面 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: String,
    pub title: String,
    pub wiki_id: String,
    pub content: String,
    pub parent_id: Option<String>,
    pub path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Wiki 源 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiSource {
    pub id: String,
    pub wiki_id: String,
    pub url: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub source_type: String,
    pub created_at: String,
}

/// Settings DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsEntry {
    pub key: String,
    pub value: String,
}

/// Backlink DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteBacklink {
    pub id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub context: Option<String>,
}

/// Session DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
}

/// Wiki DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wiki {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub avatar_path: Option<String>,
    pub allow_public: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Agent profile DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
}

// ── WorkflowEngine 系列 ─────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowExecutionData {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub input_params: Option<String>,
    pub output_result: Option<String>,
    pub node_executions: Option<String>,
    pub total_time_ms: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowTemplateData {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: String,
    pub tags: Option<String>,
    pub version: i32,
    pub is_preset: bool,
    pub is_editable: bool,
    pub is_public: bool,
    pub trigger_config: Option<String>,
    pub nodes: String,
    pub edges: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
    pub variables: Option<String>,
    pub error_config: Option<String>,
}
