// SPDX-License-Identifier: AGPL-3.0-only

//! 仓储操作 DTO —— 消除 consumer→entities 的架构违规。
//! 这些 DTO 是 entities 模型在 harness 层的等价结构体，consumer
//! 通过 trait 方法接收/返回 DTO，不直接引用 `axagent_entities`。

use serde::{Deserialize, Serialize};

/// Settings DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsEntry {
    pub key: String,
    pub value: String,
}

/// Session DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<String>,
}

// ── WorkflowEngine 系列 ─────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionData {
    pub id: String,
    pub workflow_id: String,
    pub status: String,
    pub input_params: Option<String>,
    pub output_result: Option<String>,
    pub node_executions: Option<String>,
    /// 序列化后的 ExecutionStateSnapshot，用于崩溃后恢复
    pub execution_state_json: Option<String>,
    /// 暂停时间戳（毫秒），用于超时判断
    pub paused_at: Option<i64>,
    // workflow_executions.total_time_ms 列为 BIGINT，用 i64 匹配。
    pub total_time_ms: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// L2 集群 ID（三层路由第二层，对应 CapabilityCluster::cluster_id）
    pub cluster_id: Option<String>,
    /// 三层路由路径（格式 `/{domain}/{cluster}/{capability}`）
    pub route_path: Option<String>,
}

// ── BackgroundTask 系列 ─────────────────────

/// 后台任务 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundTask {
    pub id: String,
    pub title: String,
    pub description: String,
    pub task_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub status: String,
    pub output: String,
    pub exit_code: Option<i32>,
    pub conversation_id: Option<String>,
    pub created_by: Option<String>,
    /// 幂等键：防重复提交（唯一）
    pub idempotency_key: Option<String>,
    /// 重试/续跑计数
    pub attempt: i32,
    /// 断点位置（agent 任务 checkpoint）
    pub resume_from: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub finished_at: Option<i64>,
}

/// 创建后台任务的输入 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBackgroundTaskInput {
    pub title: String,
    pub description: String,
    pub task_type: String,
    pub command: Option<String>,
    pub prompt: Option<String>,
    pub created_by: Option<String>,
    pub idempotency_key: Option<String>,
}

/// 更新后台任务状态的输入 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBackgroundTaskInput {
    pub id: String,
    pub status: Option<String>,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub finished_at: Option<i64>,
}

// ── StoredFile 系列 ─────────────────────────

/// 已存储文件 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredFile {
    pub id: String,
    pub hash: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub conversation_id: Option<String>,
    pub created_at: String,
}

/// 创建存储文件的输入 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStoredFileInput {
    pub id: String,
    pub hash: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub storage_path: String,
    pub conversation_id: Option<String>,
}

// ── Knowledge 系列 ──────────────────────────

/// 知识实体 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeEntityDto {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub source_language: Option<String>,
    pub properties: serde_json::Value,
    pub lifecycle: Option<serde_json::Value>,
    pub behaviors: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建知识实体的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeEntityInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub entity_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub source_language: Option<String>,
    pub properties: serde_json::Value,
    pub lifecycle: Option<serde_json::Value>,
    pub behaviors: Option<serde_json::Value>,
}

/// 知识流程 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeFlowDto {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub flow_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub steps: serde_json::Value,
    pub decision_points: Option<serde_json::Value>,
    pub error_handling: Option<serde_json::Value>,
    pub preconditions: Option<serde_json::Value>,
    pub postconditions: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建知识流程的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeFlowInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub flow_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub steps: serde_json::Value,
    pub decision_points: Option<serde_json::Value>,
    pub error_handling: Option<serde_json::Value>,
    pub preconditions: Option<serde_json::Value>,
    pub postconditions: Option<serde_json::Value>,
}

/// 知识接口 DTO
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeInterfaceDto {
    pub id: String,
    pub knowledge_base_id: String,
    pub name: String,
    pub interface_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub error_codes: Option<serde_json::Value>,
    pub communication_pattern: Option<String>,
    pub version: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建知识接口的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeInterfaceInput {
    pub knowledge_base_id: String,
    pub name: String,
    pub interface_type: String,
    pub description: Option<String>,
    pub source_path: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub error_codes: Option<serde_json::Value>,
    pub communication_pattern: Option<String>,
}

/// 知识文档 DTO（用于 CRUD trait）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDocumentDto {
    pub id: String,
    pub knowledge_base_id: String,
    pub title: String,
    pub source_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub indexing_status: String,
    pub doc_type: String,
    pub index_error: Option<String>,
    pub source_conversation_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 创建知识文档的输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateKnowledgeDocumentInput {
    pub knowledge_base_id: String,
    pub title: String,
    pub source_path: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub doc_type: String,
}

// ── Agent 系列 ─────────────────────────────────

/// Agency Expert DTO（技能 / 领域专家）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgencyExpertDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
    pub system_prompt: String,
    pub color: Option<String>,
    pub source_dir: String,
    pub is_enabled: bool,
    pub imported_at: i64,
    pub recommended_workflows: Option<String>,
    pub recommended_tools: Option<String>,
    pub active_domains: Option<String>,
    /// 资历等级：junior / mid / senior / expert
    pub seniority: Option<String>,
    /// 擅长细分领域（JSON 数组字符串）
    pub specialties: Option<String>,
    /// 历史成功率（0.0 ~ 1.0）
    pub success_rate: Option<f64>,
    /// 平均执行延迟（毫秒）
    pub avg_latency_ms: Option<i64>,
    /// 平均 token 成本
    pub avg_token_cost: Option<i64>,
}

/// Agent Role DTO（岗位定义，来自 DB `agent_roles` 表）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRoleDto {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub default_tools: Vec<String>,
    pub active_domains: Vec<String>,
    pub max_concurrent: i32,
    pub timeout_seconds: i64,
    pub source: String,
}

/// Workflow Execution Stats DTO（工作流执行统计，驱动效果导向优化）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionStatsDto {
    pub id: String,
    pub mission_hash: Option<String>,
    pub template_id: Option<String>,
    pub execution_id: Option<String>,
    pub status: String,
    pub total_time_ms: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub error_message: Option<String>,
    pub user_rating: Option<f64>,
    pub created_at: i64,
}

// ── Knowledge integration DTOs (was command-local in knowledge_integration.rs) ──

/// Insight generated by knowledge integration analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationInsight {
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub source_ids: Vec<SourceRef>,
    pub confidence: f64,
    pub suggested_action: Option<String>,
}

/// Category of an integration insight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    Duplicate,
    Stale,
    Related,
    Gap,
}

/// Reference to a source item from an integration insight.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceRef {
    pub container_id: String,
    pub container_type: String,
    pub item_id: String,
    pub item_title: String,
}
