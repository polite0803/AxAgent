// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A single retrieved chunk from RAG search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagRetrievedItem {
    pub content: String,
    pub score: f32,
    pub document_id: String,
    /// Chunk ID within the vector store.
    #[serde(default)]
    pub id: String,
    /// Human-readable document name (populated for knowledge items).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
    /// Chunk 在文档内的顺序索引（从 0 开始），用于引用追溯定位
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunk_index: Option<i32>,
}

/// Results from a single RAG source (knowledge base or memory namespace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSourceResult {
    /// "knowledge" or "memory"
    pub source_type: String,
    pub container_id: String,
    pub items: Vec<RagRetrievedItem>,
    /// 容器显示名（KB 名称 / memory namespace 名称 / wiki 名称），
    /// 用于前端引用追溯展示
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// Combined results of RAG context collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagContextResult {
    /// Formatted context parts for injection into system prompt.
    pub context_parts: Vec<String>,
    /// Structured results for frontend display.
    pub source_results: Vec<RagSourceResult>,
}

/// Tauri event emitted after RAG context retrieval completes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagContextRetrievedEvent {
    pub conversation_id: String,
    pub sources: Vec<RagSourceResult>,
}

// === Embedding Types ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedRequest {
    pub model: String,
    pub input: Vec<String>,
    pub dimensions: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub dimensions: usize,
}

// === Realtime Voice ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeConfig {
    pub model_id: String,
    pub voice: Option<String>,
    pub audio_format: AudioFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u8,
    pub encoding: AudioEncoding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioEncoding {
    Pcm16,
    Opus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoiceSessionState {
    Idle,
    Connecting,
    Connected,
    Speaking,
    Listening,
    Disconnecting,
    Error,
}

// ─── Phase-2 Types ───────────────────────────────────────────────

// Search
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchProvider {
    pub id: String,
    pub name: String,
    pub provider_type: String, // tavily | zhipu | bocha
    pub endpoint: Option<String>,
    pub has_api_key: bool,
    pub enabled: bool,
    pub region: Option<String>,
    pub language: Option<String>,
    pub safe_search: Option<bool>,
    pub result_limit: i32,
    pub timeout_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCitation {
    pub id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
    pub provider_id: String,
    pub rank: i32,
}

// MCP & Tools
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub transport: String, // stdio | http | sse
    pub command: Option<String>,
    pub args_json: Option<String>,
    pub endpoint: Option<String>,
    pub env_json: Option<String>,
    pub enabled: bool,
    pub permission_policy: String, // ask | allow_safe | allow_all
    pub source: String,            // builtin | custom
    pub discover_timeout_secs: Option<i32>,
    pub execute_timeout_secs: Option<i32>,
    pub headers_json: Option<String>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub description: Option<String>,
    pub input_schema_json: Option<String>,
    /// 是否幂等（多次调用结果一致，可安全重试）
    #[serde(default)]
    pub idempotent: bool,
    /// 单次调用的预估成本
    #[serde(default)]
    pub estimated_cost: Option<crate::tool::EstimatedCost>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecution {
    pub id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub server_id: String,
    pub tool_name: String,
    pub status: String, // pending | running | success | failed | cancelled
    pub input_preview: Option<String>,
    pub output_preview: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: Option<i64>,
    pub created_at: String,
    pub approval_status: Option<String>,
    pub skill_steps_json: Option<String>,
    pub depends_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub conversation_id: String,
    pub cwd: Option<String>,
    pub workspace_locked: i32,
    pub permission_mode: String,
    pub runtime_status: String,
    pub sdk_context_json: Option<String>,
    pub sdk_context_backup_json: Option<String>,
    pub total_tokens: i64,
    pub total_cost_usd: f64,
    pub created_at: i64,
    pub updated_at: i64,
}

// Knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeBase {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub embedding_provider: Option<String>,
    pub enabled: bool,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub sort_order: i32,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub chunk_size: Option<i32>,
    pub chunk_overlap: Option<i32>,
    pub separator: Option<String>,
    /// 知识库类型：默认 `indexed`（走 RAG），`connected_vault` 指向 Obsidian vault
    /// （不走 RAG，agent 通过 9 个 obsidian_* 工具直接读写 live 文件）
    #[serde(default)]
    pub kind: crate::KbKind,
    /// ConnectedVault 类型 KB 的 vault 根路径（绝对路径）
    /// 其他类型为 None
    #[serde(default)]
    pub vault_path: Option<String>,
}

impl KnowledgeBase {
    pub fn source_config(&self) -> SourceConfig {
        SourceConfig {
            embedding_provider: self.embedding_provider.clone(),
            embedding_dimensions: self.embedding_dimensions,
            retrieval_threshold: self.retrieval_threshold,
            retrieval_top_k: self.retrieval_top_k,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocument {
    pub id: String,
    pub knowledge_base_id: String,
    pub title: String,
    pub source_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub indexing_status: String, // pending | indexing | ready | failed
    pub doc_type: String,        // file | url | text | conversation | ...
    pub index_error: Option<String>,
    pub source_conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntity {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub source_language: Option<String>,
    pub properties: Value,
    pub lifecycle: Option<Value>,
    pub behaviors: Option<Value>,
    pub metadata: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
    // v101: trajectory entity fields
    #[serde(default = "default_aliases")]
    pub aliases: String,
    #[serde(default)]
    pub mention_count: i32,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    pub first_seen_at: Option<String>,
    pub last_seen_at: Option<String>,
}

fn default_aliases() -> String {
    "[]".to_string()
}
fn default_confidence() -> f64 {
    0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeEntityInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub source_language: Option<String>,
    pub properties: Value,
    pub lifecycle: Option<Value>,
    pub behaviors: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAttribute {
    pub id: String,
    pub knowledge_base_id: String,
    pub entity_id: String,
    pub name: String,
    pub attribute_type: String,
    pub data_type: String,
    pub description: Option<String>,
    pub is_required: bool,
    pub default_value: Option<String>,
    pub constraints: Option<Value>,
    pub validation_rules: Option<Value>,
    pub metadata: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeAttributeInput {
    pub knowledge_base_id: String,
    pub entity_id: String,
    pub name: String,
    pub attribute_type: String,
    pub data_type: String,
    pub description: Option<String>,
    pub is_required: bool,
    pub default_value: Option<String>,
    pub constraints: Option<Value>,
    pub validation_rules: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRelation {
    pub id: String,
    pub knowledge_base_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub relation_type: String,
    pub description: Option<String>,
    pub properties: Option<Value>,
    pub metadata: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
    // v101: trajectory relationship weight
    #[serde(default = "default_weight")]
    pub weight: f64,
}

fn default_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeRelationInput {
    pub knowledge_base_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub relation_type: String,
    pub description: Option<String>,
    pub properties: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeFlow {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub flow_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub steps: Value,
    pub decision_points: Option<Value>,
    pub error_handling: Option<Value>,
    pub preconditions: Option<Value>,
    pub postconditions: Option<Value>,
    pub metadata: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeFlowInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub flow_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub steps: Value,
    pub decision_points: Option<Value>,
    pub error_handling: Option<Value>,
    pub preconditions: Option<Value>,
    pub postconditions: Option<Value>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeInterface {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub interface_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub error_codes: Option<Value>,
    pub communication_pattern: Option<String>,
    pub version: Option<String>,
    pub metadata: Option<Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeInterfaceInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub interface_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub error_codes: Option<Value>,
    pub communication_pattern: Option<String>,
    pub version: Option<String>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievalHit {
    pub id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub knowledge_base_id: String,
    pub document_id: String,
    pub chunk_ref: String,
    pub score: f64,
    pub preview: String,
}

// Memory
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryNamespace {
    pub id: String,
    pub name: String,
    pub scope: String, // global | project
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    pub sort_order: i32,
}

impl MemoryNamespace {
    pub fn source_config(&self) -> SourceConfig {
        SourceConfig {
            embedding_provider: self.embedding_provider.clone(),
            embedding_dimensions: self.embedding_dimensions,
            retrieval_threshold: self.retrieval_threshold,
            retrieval_top_k: self.retrieval_top_k,
        }
    }
}

// Wiki（从 dao 提升到 harness，让 search crate 不用反向依赖 dao）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wiki {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub root_path: String,
    pub schema_version: String,
    pub note_count: i32,
    pub source_count: i32,
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Wiki {
    pub fn source_config(&self) -> SourceConfig {
        SourceConfig {
            embedding_provider: self.embedding_provider.clone(),
            embedding_dimensions: self.embedding_dimensions,
            retrieval_threshold: self.retrieval_threshold,
            retrieval_top_k: self.retrieval_top_k,
        }
    }
}

// NoteLink（从 dao 提升到 harness）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteLink {
    pub id: i64,
    pub vault_id: String,
    pub source_note_id: String,
    pub target_note_id: String,
    pub link_text: String,
    pub link_type: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryItem {
    pub id: String,
    pub namespace_id: String,
    pub title: String,
    pub content: String,
    pub source: String,       // manual | auto_extract
    pub index_status: String, // pending | indexing | ready | failed | skipped
    pub index_error: Option<String>,
    pub updated_at: String,
    // 三层记忆系统：v101 已持久化，DTO 同步暴露
    pub tier: String,    // short_term | working | long_term | core
    pub importance: f64, // 0.0 ~ 1.0
    pub access_count: i32,
    pub last_accessed: Option<i64>, // unix millis
    pub decay_rate: f64,            // 每小时衰减系数
    pub expires_at: Option<i64>,    // unix millis，None 表示不过期
    pub memory_nature: String,      // episodic | semantic
    pub tags: Vec<String>,
    pub source_conversation_id: Option<String>,
    pub source_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub container_type: String,
    pub id: String,
}

impl SourceRef {
    pub fn knowledge(id: impl Into<String>) -> Self {
        SourceRef { container_type: "knowledge".to_string(), id: id.into() }
    }
    pub fn memory(id: impl Into<String>) -> Self {
        SourceRef { container_type: "memory".to_string(), id: id.into() }
    }
    pub fn wiki(id: impl Into<String>) -> Self {
        SourceRef { container_type: "wiki".to_string(), id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceConfig {
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
}

impl SourceConfig {
    pub fn with_rag_options(
        self,
        _rerank_enabled: Option<bool>,
        _self_rag_enabled: Option<bool>,
        _query_enhancement_enabled: Option<bool>,
    ) -> Self {
        // RAG 选项现在通过 RAGPipelineConfig.rerank / .self_rag 的 Value 字段传递
        self
    }
}

// Artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub id: String,
    pub conversation_id: String,
    pub kind: String, // draft | note | report | snippet | checklist
    pub title: String,
    pub content: String,
    pub format: String, // markdown | text | json
    pub pinned: bool,
    pub updated_at: String,
}

// Context Sources
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSource {
    pub id: String,
    pub conversation_id: String,
    pub message_id: Option<String>,
    #[serde(rename = "type")]
    pub source_type: String, // app | attachment | search | knowledge | memory | tool
    pub ref_id: String,
    pub title: String,
    pub enabled: bool,
    pub summary: Option<String>,
    /// 多文档协同：限制 RAG 检索范围到这些文档 ID；
    /// 空数组或 None 表示检索整个容器
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doc_ids: Vec<String>,
}

// Conversation Branches
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationBranch {
    pub id: String,
    pub conversation_id: String,
    pub parent_message_id: String,
    pub branch_label: String,
    pub branch_index: i32,
    pub compared_message_ids_json: Option<String>,
    pub created_at: String,
}

// Backup & Migration

/// JSON 备份恢复策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreStrategy {
    /// 清空现有数据后导入
    Overwrite,
    /// 合并导入，跳过已存在的记录
    Merge,
    /// 仅验证备份文件完整性，不实际修改数据
    DryRun,
}

impl std::fmt::Display for RestoreStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreStrategy::Overwrite => write!(f, "overwrite"),
            RestoreStrategy::Merge => write!(f, "merge"),
            RestoreStrategy::DryRun => write!(f, "dry_run"),
        }
    }
}

/// 单个表的恢复结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRestoreResult {
    pub table: String,
    pub rows_imported: usize,
    pub rows_skipped: usize,
    pub rows_errored: usize,
}

/// JSON 备份恢复报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub backup_version: String,
    pub strategy: String,
    pub tables_restored: Vec<TableRestoreResult>,
    pub total_imported: usize,
    pub total_skipped: usize,
    pub total_errored: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub id: String,
    pub version: String,
    pub created_at: String,
    pub encrypted: bool,
    pub checksum: String,
    pub object_counts_json: String,
    pub source_app_version: String,
    pub file_path: Option<String>,
    pub file_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupTarget {
    pub id: String,
    pub kind: String, // local | webdav | s3
    pub config_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoBackupSettings {
    pub enabled: bool,
    pub interval_hours: u32,
    pub max_count: u32,
    pub backup_dir: Option<String>,
}

// Gateway Phase-2
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgramPolicy {
    pub id: String,
    pub program_name: String,
    pub allowed_provider_ids_json: String,
    pub allowed_model_ids_json: String,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub rate_limit_per_minute: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayDiagnostic {
    pub id: String,
    pub category: String, // provider_latency | provider_error | proxy | auth | port
    pub status: String,   // ok | warning | error
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayRequestLog {
    pub id: String,
    pub key_id: String,
    pub key_name: String,
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub provider_id: Option<String>,
    pub status_code: i32,
    pub duration_ms: i64,
    pub request_tokens: i64,
    pub response_tokens: i64,
    pub error_message: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayTemplate {
    pub id: String,
    pub name: String,
    pub target: String, // cursor | vscode | claude_code | openai_compatible
    pub format: String, // json | yaml | markdown
    pub content: String,
    pub copy_hint: Option<String>,
}

// CLI Tool Integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliToolInfo {
    pub id: String,
    pub name: String,
    pub status: String, // not_installed | not_connected | connected
    pub version: Option<String>,
    pub config_path: Option<String>,
    pub has_backup: bool,
    pub connected_protocol: Option<String>,
}

// Desktop
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopState {
    pub window_key: String, // main | mini | voice | artifact
    pub width: i32,
    pub height: i32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub maximized: bool,
    pub visible: bool,
}

// ─── Phase-2 Input Types (non-FromRow) ───────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateSearchProviderInput {
    pub name: String,
    pub provider_type: String,
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub region: Option<String>,
    pub language: Option<String>,
    pub safe_search: Option<bool>,
    pub result_limit: Option<i32>,
    pub timeout_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateMcpServerInput {
    pub name: String,
    pub alias: Option<String>,
    pub description: Option<String>,
    pub transport: String,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub endpoint: Option<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    pub enabled: Option<bool>,
    pub permission_policy: Option<String>,
    pub source: Option<String>,
    pub discover_timeout_secs: Option<i32>,
    pub execute_timeout_secs: Option<i32>,
    pub headers_json: Option<String>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArtifactInput {
    pub conversation_id: String,
    pub source_message_id: Option<String>,
    pub kind: String,
    pub title: String,
    pub content: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateArtifactInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub format: Option<String>,
    pub pinned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateContextSourceInput {
    pub conversation_id: String,
    pub message_id: Option<String>,
    pub source_type: String,
    pub ref_id: String,
    pub title: String,
    pub summary: Option<String>,
    /// 多文档协同：限制 RAG 检索范围到这些文档 ID；
    /// 空数组或 None 表示检索整个容器
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doc_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackupJobInput {
    pub target_kind: String,
    pub target_config_json: String,
    pub include_attachments: bool,
    pub include_knowledge_files: bool,
    pub include_gateway_config: bool,
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceInput {
    pub source_type: String,
    pub path: String,
    pub credentials_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPolicyInput {
    pub duplicate_strategy: String, // skip | rename | overwrite
    pub merge_settings: bool,
    pub merge_apps: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProgramPolicyInput {
    pub program_name: String,
    pub allowed_provider_ids: Vec<String>,
    pub allowed_model_ids: Vec<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
    pub rate_limit_per_minute: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeBaseInput {
    pub name: String,
    pub description: Option<String>,
    pub embedding_provider: Option<String>,
    pub enabled: Option<bool>,
    /// KB 类型，默认 `indexed`。设为 `connected_vault` 时需提供 `vault_path`
    #[serde(default)]
    pub kind: crate::KbKind,
    /// ConnectedVault 类型时的 vault 根路径（绝对路径）
    #[serde(default)]
    pub vault_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateKnowledgeBaseInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub embedding_provider: Option<String>,
    pub enabled: Option<bool>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    #[serde(default)]
    pub update_icon: bool,
    pub embedding_dimensions: Option<i32>,
    #[serde(default)]
    pub update_embedding_dimensions: bool,
    pub retrieval_threshold: Option<f32>,
    #[serde(default)]
    pub update_retrieval_threshold: bool,
    pub retrieval_top_k: Option<i32>,
    #[serde(default)]
    pub update_retrieval_top_k: bool,
    pub chunk_size: Option<i32>,
    #[serde(default)]
    pub update_chunk_size: bool,
    pub chunk_overlap: Option<i32>,
    #[serde(default)]
    pub update_chunk_overlap: bool,
    pub separator: Option<String>,
    #[serde(default)]
    pub update_separator: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryNamespaceInput {
    pub name: String,
    pub scope: String,
    pub embedding_provider: Option<String>,
    pub embedding_dimensions: Option<i32>,
    pub retrieval_threshold: Option<f32>,
    pub retrieval_top_k: Option<i32>,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
}

/// 统一知识源创建输入（通过 sourceType 区分 knowledge/memory/wiki/obsidian_vault）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSourceInput {
    pub name: String,
    /// "knowledge" | "memory" | "wiki" | "obsidian_vault"
    ///
    /// - `knowledge`: 走 RAG 索引的默认 KB
    /// - `memory`: AI 长期记忆 namespace
    /// - `wiki`: 结构化笔记 + 文件夹根路径
    /// - `obsidian_vault`: ConnectedVault KB，指针指向外部 Obsidian vault，
    ///   不索引、不向量化，agent 通过 9 个 `obsidian_*` 工具直接读写 live 文件
    pub source_type: String,
    pub description: Option<String>,
    pub embedding_provider: Option<String>,
    /// memory 独有
    pub scope: Option<String>,
    /// wiki 独有
    pub root_path: Option<String>,
    /// obsidian_vault 独有：Obsidian vault 的绝对路径
    pub vault_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryNamespaceInput {
    pub name: Option<String>,
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub update_embedding_provider: bool,
    pub embedding_dimensions: Option<i32>,
    #[serde(default)]
    pub update_embedding_dimensions: bool,
    pub retrieval_threshold: Option<f32>,
    #[serde(default)]
    pub update_retrieval_threshold: bool,
    pub retrieval_top_k: Option<i32>,
    #[serde(default)]
    pub update_retrieval_top_k: bool,
    pub icon_type: Option<String>,
    pub icon_value: Option<String>,
    #[serde(default)]
    pub update_icon: bool,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMemoryItemInput {
    pub namespace_id: String,
    pub title: String,
    pub content: String,
    pub source: Option<String>,
    // 三层记忆系统：创建时可选指定 tier/importance/nature/tags/decay_rate/expires_at
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub importance: Option<f64>,
    #[serde(default)]
    pub memory_nature: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub decay_rate: Option<f64>,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemoryItemInput {
    pub title: Option<String>,
    pub content: Option<String>,
    // 三层记忆系统：更新时可选调整 tier/importance/nature/tags
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub importance: Option<f64>,
    #[serde(default)]
    pub memory_nature: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

// ── Skills ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub author: Option<String>,
    pub version: Option<String>,
    pub source: String,
    pub source_path: String,
    pub enabled: bool,
    pub has_update: bool,
    pub user_invocable: bool,
    pub argument_hint: Option<String>,
    pub when_to_use: Option<String>,
    pub group: Option<String>,
    #[serde(default)]
    pub manifest: Option<serde_json::Value>,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub info: SkillInfo,
    pub content: String,
    pub files: Vec<String>,
    pub manifest: Option<SkillManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifest {
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub installed_at: String,
    pub installed_via: Option<String>,
}

// ── Skill Frontend Extension ──

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillFrontendExtension {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub navigation: Vec<SkillNavItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pages: Vec<SkillPage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub commands: Vec<SkillUICommand>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SkillUIPanel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings_sections: Vec<SkillSettingsSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillNavItem {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub path: String,
    #[serde(default)]
    pub position: NavPosition,
    #[serde(default)]
    pub order: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NavPosition {
    #[default]
    Bottom,
    Top,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPage {
    pub id: String,
    pub path: String,
    pub title: String,
    #[serde(rename = "componentType")]
    pub component_type: SkillComponentType,
    #[serde(rename = "componentConfig")]
    pub component_config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SkillComponentType {
    Html,
    Iframe,
    React,
    WebComponent,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUICommand {
    pub id: String,
    pub label: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,
    pub action: SkillCommandAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SkillCommandAction {
    Navigate { path: String },
    InvokeBackend { command: String, args: serde_json::Value },
    EmitEvent { event: String, payload: serde_json::Value },
    Custom { handler_id: String, data: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUIPanel {
    pub id: String,
    pub title: String,
    #[serde(rename = "componentType")]
    pub component_type: SkillComponentType,
    #[serde(rename = "componentConfig")]
    pub component_config: serde_json::Value,
    #[serde(default)]
    pub position: UIPanelPosition,
    #[serde(default)]
    pub size: UIPanelSize,
    #[serde(default)]
    pub collapsible: bool,
    #[serde(default = "default_false")]
    pub default_collapsed: bool,
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum UIPanelPosition {
    #[default]
    Main,
    Sidebar,
    Header,
    Footer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum UIPanelSize {
    Small,
    #[default]
    Medium,
    Large,
    FullWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSettingsSection {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(rename = "componentType")]
    pub component_type: SkillComponentType,
    #[serde(rename = "componentConfig")]
    pub component_config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInfo {
    pub name: String,
    pub current_commit: String,
    pub latest_commit: String,
    pub source_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceSkill {
    pub name: String,
    pub description: String,
    pub repo: String,
    pub stars: i64,
    pub installs: i64,
    pub installed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_update: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub skill_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub version: i32,
    pub is_active: bool,
    pub ab_test_enabled: bool,
    pub ab_test_variant: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub format: Option<String>,
    pub metadata_json: Option<String>,
    pub usage_count: i32,
    pub is_favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePromptTemplateInput {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub format: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePromptTemplateInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub variables_schema: Option<String>,
    pub is_active: Option<bool>,
    pub ab_test_enabled: Option<bool>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub format: Option<String>,
    pub metadata_json: Option<String>,
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplateVersion {
    pub id: String,
    pub template_id: String,
    pub version: i32,
    pub content: String,
    pub variables_schema: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub changelog: Option<String>,
    pub created_at: i64,
}

/// 导入提示词模板的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPromptTemplateInput {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
    pub source_type: Option<String>,
    pub format: Option<String>,
    pub metadata_json: Option<String>,
}

/// 批量导入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPromptResult {
    pub imported: Vec<PromptTemplate>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

/// 从 URL 导入的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromUrlInput {
    pub url: String,
    pub category_filter: Option<String>,
    pub overwrite_existing: Option<bool>,
}

/// 导出格式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportPromptFormat {
    Json,
    Yaml,
    Markdown,
}

/// 导出的提示词条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedPrompt {
    pub name: String,
    pub description: Option<String>,
    pub content: String,
    pub variables_schema: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
    pub source: Option<String>,
}

// === Agent Profile ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub icon: String,
    pub agent_role: Option<String>,
    pub source: String,
    pub tags: Vec<String>,
    pub suggested_provider_id: Option<String>,
    pub suggested_model_id: Option<String>,
    pub suggested_temperature: Option<f64>,
    pub suggested_max_tokens: Option<u32>,
    pub search_enabled: Option<bool>,
    pub recommend_permission_mode: Option<String>,
    pub recommended_tools: Vec<String>,
    pub disallowed_tools: Vec<String>,
    pub recommended_workflows: Vec<String>,
    pub sort_order: i32,
    pub is_enabled: bool,
    pub expert_id: Option<String>,
    /// 业务岗位 ID（business_roles.id），表达「这个 profile 在组织中担什么岗位」
    pub business_role_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateAgentProfileInput {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub agent_role: Option<String>,
    pub source: Option<String>,
    pub tags: Option<Vec<String>>,
    pub suggested_provider_id: Option<String>,
    pub suggested_model_id: Option<String>,
    pub suggested_temperature: Option<f64>,
    pub suggested_max_tokens: Option<u32>,
    pub search_enabled: Option<bool>,
    pub recommend_permission_mode: Option<String>,
    pub recommended_tools: Option<Vec<String>>,
    pub disallowed_tools: Option<Vec<String>>,
    pub recommended_workflows: Option<Vec<String>>,
    /// 业务岗位 ID（business_roles.id）
    pub business_role_id: Option<String>,
}

// === Agent Role Def (DB-driven, importable) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleDef {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub active_domains: Vec<String>,
    pub max_concurrent: usize,
    pub timeout_seconds: u64,
    pub source: String,
    pub sort_order: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

// === Query Enhancement Types ===

/// 查询增强策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementStrategy {
    /// 不增强，直接使用原始查询
    None,
    /// 假设文档嵌入（HyDE）
    Hyde,
    /// 多查询改写
    MultiQuery,
    /// 查询分解
    Decomposition,
    /// 自动选择（基于查询特征）
    Auto,
}

/// 增强后的查询及其元数据
#[derive(Debug, Clone)]
pub struct EnhancedQuery {
    /// 增强后的查询文本
    pub text: String,
    /// 使用的策略
    pub strategy: EnhancementStrategy,
    /// 该查询的权重（用于结果合并）
    pub weight: f32,
}

/// 查询增强配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancementConfig {
    pub enabled: bool,
    pub strategy: EnhancementStrategy,
    /// 最大增强查询数（MultiQuery 的变体数）
    pub max_variants: usize,
    /// 是否合并 HyDE 和 MultiQuery 为一次 LLM 调用
    pub combined_call: bool,
}

impl Default for EnhancementConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            strategy: EnhancementStrategy::Auto,
            max_variants: 3,
            combined_call: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAgentProfileInput {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub category: Option<String>,
    pub icon: Option<String>,
    pub agent_role: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub is_enabled: Option<bool>,
    /// 业务岗位 ID（None 表示不修改；Some(None) 表示解除关联；Some(Some(id)) 表示关联到指定岗位）
    pub business_role_id: Option<Option<String>>,
}

// Re-export from sibling modules for convenience
pub use crate::rag_config::RAGPipelineConfig;
pub use crate::rag_config::RerankConfig;
pub use crate::rag_config::SelfRagConfig;
pub use crate::rag_config::{Note, NoteSearchResult};
