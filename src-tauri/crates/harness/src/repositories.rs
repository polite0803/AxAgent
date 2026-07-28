// SPDX-License-Identifier: AGPL-3.0-only

//! 仓储 trait 定义 + 全局依赖注入点。
//!
//! 每个 trait 对应一个实体维度。consumer crate（agent/orchestrator/tools）
//! 通过 `repo_accessor()` 获取实现，不直接依赖 `axagent-dao` 或 `axagent-entities`。

use std::sync::Arc;

use async_trait::async_trait;

use crate::repo_dtos::*;
// wiki 域 repository trait 以 wiki_dtos 为唯一权威定义（dao 实现、agent 消费、test_support 替身均认它），
// 此处仅 re-export，避免与内联重复定义造成两套不兼容类型。
use crate::service_registry::get_service_registry;
use crate::types::AppSettings;
use crate::types::Conversation;
use crate::types::Message;
use crate::types::MessageRole;
use crate::types::ProviderConfig;
use crate::types::ProviderKey;
pub use crate::wiki_dtos::{
    NoteBacklinkRepository, NoteRepository, WikiOperationRepository, WikiPageRepository,
    WikiRepository, WikiSourceRepository,
};
use std::collections::HashMap;

pub fn set_note_repository(repo: Arc<dyn NoteRepository>) {
    get_service_registry().read().unwrap().set_note_repository(repo);
}

pub fn note_repository() -> Arc<dyn NoteRepository> {
    get_service_registry().read().unwrap().note_repository()
}

pub fn set_wiki_repository(repo: Arc<dyn WikiRepository>) {
    get_service_registry().read().unwrap().set_wiki_repository(repo);
}

pub fn wiki_repository() -> Arc<dyn WikiRepository> {
    get_service_registry().read().unwrap().wiki_repository()
}

pub fn set_wiki_page_repository(repo: Arc<dyn WikiPageRepository>) {
    get_service_registry().read().unwrap().set_wiki_page_repository(repo);
}

pub fn wiki_page_repository() -> Arc<dyn WikiPageRepository> {
    get_service_registry().read().unwrap().wiki_page_repository()
}

pub fn set_wiki_source_repository(repo: Arc<dyn WikiSourceRepository>) {
    get_service_registry().read().unwrap().set_wiki_source_repository(repo);
}

pub fn wiki_source_repository() -> Arc<dyn WikiSourceRepository> {
    get_service_registry().read().unwrap().wiki_source_repository()
}

pub fn set_note_backlink_repository(repo: Arc<dyn NoteBacklinkRepository>) {
    get_service_registry().read().unwrap().set_note_backlink_repository(repo);
}

pub fn note_backlink_repository() -> Arc<dyn NoteBacklinkRepository> {
    get_service_registry().read().unwrap().note_backlink_repository()
}

// ── WikiOperationRepository ───────────────────

pub fn set_wiki_operation_repository(repo: Arc<dyn WikiOperationRepository>) {
    get_service_registry().read().unwrap().set_wiki_operation_repository(repo);
}

pub fn wiki_operation_repository() -> Arc<dyn WikiOperationRepository> {
    get_service_registry().read().unwrap().wiki_operation_repository()
}

// ── ProviderRepository ────────────────────────

#[async_trait]
pub trait ProviderRepository: Send + Sync {
    async fn list_providers(&self) -> Result<Vec<ProviderConfig>, String>;
    async fn get_provider(&self, id: &str) -> Result<ProviderConfig, String>;
    async fn get_active_key(&self, provider_id: &str) -> Result<ProviderKey, String>;
    async fn resolve_model_for_node(
        &self,
        node_model: Option<&str>,
        session_model: Option<&str>,
        session_provider_id: Option<&str>,
        profile_suggested_provider: Option<&str>,
    ) -> Result<(ProviderConfig, ProviderKey, String), String>;
}

pub fn set_provider_repository(repo: Arc<dyn ProviderRepository>) {
    get_service_registry().read().unwrap().set_provider_repository(repo);
}

pub fn provider_repository() -> Arc<dyn ProviderRepository> {
    get_service_registry().read().unwrap().provider_repository()
}

// ── PlatformConfigRepository ───────────────────

#[async_trait]
pub trait PlatformConfigRepository: Send + Sync {
    async fn load_session_routes(&self) -> HashMap<String, String>;
    async fn save_session_routes(&self, routes: &HashMap<String, String>) -> Result<(), String>;
    async fn save_platform_cursor(&self, platform: &str, cursor: i64) -> Result<(), String>;
}

pub fn set_platform_config_repository(repo: Arc<dyn PlatformConfigRepository>) {
    get_service_registry().read().unwrap().set_platform_config_repository(repo);
}

pub fn platform_config_repository() -> Arc<dyn PlatformConfigRepository> {
    get_service_registry().read().unwrap().platform_config_repository()
}

// ── ConversationRepository ─────────────────────

/// Input for creating a conversation.
pub struct CreateConversationInput {
    pub title: String,
    pub model_id: String,
    pub provider_id: String,
    pub system_prompt: Option<String>,
}

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn get_conversation(&self, id: &str) -> Result<Conversation, String>;
    async fn create_conversation(
        &self,
        input: CreateConversationInput,
    ) -> Result<Conversation, String>;
    async fn increment_message_count(&self, conversation_id: &str) -> Result<(), String>;
}

pub fn set_conversation_repository(repo: Arc<dyn ConversationRepository>) {
    get_service_registry().read().unwrap().set_conversation_repository(repo);
}

pub fn conversation_repository() -> Arc<dyn ConversationRepository> {
    get_service_registry().read().unwrap().conversation_repository()
}

// ── MessageRepository ──────────────────────────

/// Input for creating a message.
pub struct CreateMessageInput {
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
}

#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn create_message(&self, input: CreateMessageInput) -> Result<Message, String>;
    /// List all messages with id and attachments JSON (for migration / admin use).
    /// Returns `Vec<(message_id, attachments_json)>`.
    async fn list_all_message_attachments(&self) -> Result<Vec<(String, String)>, String>;
    /// Update the attachments JSON column.
    async fn update_message_attachments(
        &self,
        message_id: &str,
        attachments_json: &str,
    ) -> Result<(), String>;
}

pub fn set_message_repository(repo: Arc<dyn MessageRepository>) {
    get_service_registry().read().unwrap().set_message_repository(repo);
}

pub fn message_repository() -> Arc<dyn MessageRepository> {
    get_service_registry().read().unwrap().message_repository()
}

// ── GeneratedToolRepository ────────────────────

/// Input for inserting a generated tool into the database.
pub struct InsertGeneratedToolInput {
    pub tool_name: String,
    pub original_name: String,
    pub original_description: String,
    pub input_schema: String,
    pub output_schema: String,
    pub implementation: String,
    pub source_info: String,
    pub created_at: i64,
}

#[async_trait]
pub trait GeneratedToolRepository: Send + Sync {
    async fn insert_generated_tool(&self, input: InsertGeneratedToolInput) -> Result<(), String>;
}

pub fn set_generated_tool_repository(repo: Arc<dyn GeneratedToolRepository>) {
    get_service_registry().read().unwrap().set_generated_tool_repository(repo);
}

pub fn generated_tool_repository() -> Arc<dyn GeneratedToolRepository> {
    get_service_registry().read().unwrap().generated_tool_repository()
}

// ── SettingsRepository ─────────────────────────

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, String>;
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), String>;
    async fn get_settings(&self) -> Result<AppSettings, String>;
}

pub fn set_settings_repository(repo: Arc<dyn SettingsRepository>) {
    get_service_registry().read().unwrap().set_settings_repository(repo);
}

pub fn settings_repository() -> Arc<dyn SettingsRepository> {
    get_service_registry().read().unwrap().settings_repository()
}

// ── SessionRepository ──────────────────────────

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn list_sessions(&self) -> Result<Vec<SessionRecord>, String>;
    async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, String>;
    async fn delete_session(&self, id: &str) -> Result<(), String>;
}

pub fn set_session_repository(repo: Arc<dyn SessionRepository>) {
    get_service_registry().read().unwrap().set_session_repository(repo);
}

pub fn session_repository() -> Arc<dyn SessionRepository> {
    get_service_registry().read().unwrap().session_repository()
}

// ── ToolExecutionRepository ────────────────────

#[async_trait]
pub trait ToolExecutionRepository: Send + Sync {
    async fn create_tool_execution(
        &self,
        conversation_id: &str,
        message_id: Option<&str>,
        server_id: &str,
        tool_name: &str,
        input: Option<&str>,
    ) -> Result<crate::types::ToolExecution, String>;
    async fn update_tool_execution_status(
        &self,
        execution_id: &str,
        status: &str,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), String>;
}

pub fn set_tool_execution_repository(repo: Arc<dyn ToolExecutionRepository>) {
    get_service_registry().read().unwrap().set_tool_execution_repository(repo);
}

pub fn tool_execution_repository() -> Arc<dyn ToolExecutionRepository> {
    get_service_registry().read().unwrap().tool_execution_repository()
}

// ── MemoryRepository ───────────────────────────

#[async_trait]
pub trait MemoryRepository: Send + Sync {
    async fn list_namespaces(&self) -> Result<Vec<crate::types::MemoryNamespace>, String>;
    async fn add_item(
        &self,
        input: crate::types::CreateMemoryItemInput,
    ) -> Result<crate::types::MemoryItem, String>;
}

pub fn set_memory_repository(repo: Arc<dyn MemoryRepository>) {
    get_service_registry().read().unwrap().set_memory_repository(repo);
}

pub fn memory_repository() -> Arc<dyn MemoryRepository> {
    get_service_registry().read().unwrap().memory_repository()
}

pub fn try_memory_repository() -> Option<Arc<dyn MemoryRepository>> {
    get_service_registry().read().unwrap().memory_repository_opt()
}

#[async_trait]
pub trait DatabaseInitializer: Send + Sync {
    async fn run_initialization(&self) -> Result<(), String>;
}

pub fn set_database_initializer(init: Arc<dyn DatabaseInitializer>) {
    get_service_registry().read().unwrap().set_database_initializer(init);
}

pub fn database_initializer() -> Arc<dyn DatabaseInitializer> {
    get_service_registry().read().unwrap().database_initializer()
}

// ── SkillDirsProvider ──────────────────────────

pub trait SkillDirsProvider: Send + Sync {
    fn skill_dirs(&self) -> Vec<String>;
}

pub fn set_skill_dirs_provider(provider: Arc<dyn SkillDirsProvider>) {
    get_service_registry().read().unwrap().set_skill_dirs_provider(provider);
}

pub fn skill_dirs_provider() -> Arc<dyn SkillDirsProvider> {
    get_service_registry().read().unwrap().skill_dirs_provider()
}

// ── WorkflowExecutionRepository ────────────────

#[async_trait]
pub trait WorkflowExecutionRepository: Send + Sync {
    async fn create_workflow_execution(
        &self,
        id: &str,
        workflow_id: &str,
        input_params: Option<&str>,
    ) -> Result<(), String>;
    async fn update_workflow_execution_status(
        &self,
        id: &str,
        status: &str,
        output_result: Option<&str>,
        node_executions: Option<&str>,
        total_time_ms: Option<i64>,
    ) -> Result<bool, String>;
    async fn list_workflow_executions(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowExecutionData>, String>;
}

pub fn set_workflow_execution_repository(repo: Arc<dyn WorkflowExecutionRepository>) {
    get_service_registry().read().unwrap().set_workflow_execution_repository(repo);
}

pub fn workflow_execution_repository() -> Arc<dyn WorkflowExecutionRepository> {
    get_service_registry().read().unwrap().workflow_execution_repository()
}

// ── LoopCheckpointRepository ────────────────────

#[async_trait]
pub trait LoopCheckpointRepository: Send + Sync {
    async fn save_loop_checkpoint(
        &self,
        cp: &crate::workflow_types::LoopCheckpoint,
    ) -> Result<(), String>;
    async fn load_loop_checkpoint(
        &self,
        execution_id: &str,
        node_id: &str,
    ) -> Result<Option<crate::workflow_types::LoopCheckpoint>, String>;
    async fn delete_loop_checkpoint(&self, execution_id: &str, node_id: &str)
    -> Result<(), String>;
    async fn delete_loop_checkpoints_for_execution(&self, execution_id: &str)
    -> Result<(), String>;
}

pub fn set_loop_checkpoint_repository(repo: Arc<dyn LoopCheckpointRepository>) {
    get_service_registry().read().unwrap().set_loop_checkpoint_repository(repo);
}

pub fn loop_checkpoint_repository() -> Arc<dyn LoopCheckpointRepository> {
    get_service_registry().read().unwrap().loop_checkpoint_repository()
}

// ── WorkflowTemplateRepository ──────────────────

#[async_trait]
pub trait WorkflowTemplateRepository: Send + Sync {
    async fn get_workflow_template(&self, id: &str)
    -> Result<Option<WorkflowTemplateData>, String>;
}

pub fn set_workflow_template_repository(repo: Arc<dyn WorkflowTemplateRepository>) {
    get_service_registry().read().unwrap().set_workflow_template_repository(repo);
}

pub fn workflow_template_repository() -> Arc<dyn WorkflowTemplateRepository> {
    get_service_registry().read().unwrap().workflow_template_repository()
}

// ── AgentProfileRepository ─────────────────────

#[async_trait]
pub trait AgentProfileRepository: Send + Sync {
    async fn get_agent_profile(
        &self,
        id: &str,
    ) -> Result<Option<crate::types::AgentProfile>, String>;
}

pub fn set_agent_profile_repository(repo: Arc<dyn AgentProfileRepository>) {
    get_service_registry().read().unwrap().set_agent_profile_repository(repo);
}

pub fn agent_profile_repository() -> Arc<dyn AgentProfileRepository> {
    get_service_registry().read().unwrap().agent_profile_repository()
}

// ── AgencyExpertRepository ─────────────────────

#[async_trait]
pub trait AgencyExpertRepository: Send + Sync {
    async fn get_agency_expert(&self, id: &str) -> Result<Option<AgencyExpertDto>, String>;
    /// 列出所有专家（按 sort_order/name 排序），用于 LLM prompt 注入清单。
    /// 仅返回 enabled=true 的记录。
    async fn list_agency_experts(&self) -> Result<Vec<AgencyExpertDto>, String>;
}

pub fn set_agency_expert_repository(repo: Arc<dyn AgencyExpertRepository>) {
    get_service_registry().read().unwrap().set_agency_expert_repository(repo);
}

pub fn agency_expert_repository() -> Arc<dyn AgencyExpertRepository> {
    get_service_registry().read().unwrap().agency_expert_repository()
}

// ── AgentRoleRepository ────────────────────────

#[async_trait]
pub trait AgentRoleRepository: Send + Sync {
    async fn get_agent_role(&self, id: &str) -> Result<Option<AgentRoleDto>, String>;
}

pub fn set_agent_role_repository(repo: Arc<dyn AgentRoleRepository>) {
    get_service_registry().read().unwrap().set_agent_role_repository(repo);
}

pub fn agent_role_repository() -> Arc<dyn AgentRoleRepository> {
    get_service_registry().read().unwrap().agent_role_repository()
}

// ── BusinessRoleRepository ─────────────────────
// 业务岗位（CEO/CTO/产品经理 等）—— 与 AgentRole 抽象执行器类型区别。

#[async_trait]
pub trait BusinessRoleRepository: Send + Sync {
    async fn get_business_role(&self, id: &str) -> Result<Option<BusinessRoleDto>, String>;
    /// 列出所有业务岗位（按 sort_order 排序），用于 LLM prompt 注入清单。
    /// 仅返回 is_enabled=true 的记录。
    async fn list_business_roles(&self) -> Result<Vec<BusinessRoleDto>, String>;
}

pub fn set_business_role_repository(repo: Arc<dyn BusinessRoleRepository>) {
    get_service_registry().read().unwrap().set_business_role_repository(repo);
}

pub fn business_role_repository() -> Arc<dyn BusinessRoleRepository> {
    get_service_registry().read().unwrap().business_role_repository()
}

// ── BackgroundTaskRepository ──────────────────

#[async_trait]
pub trait BackgroundTaskRepository: Send + Sync {
    async fn spawn_task(&self, input: CreateBackgroundTaskInput) -> Result<BackgroundTask, String>;
    async fn get_task(&self, id: &str) -> Result<Option<BackgroundTask>, String>;
    async fn list_tasks(&self) -> Result<Vec<BackgroundTask>, String>;
    async fn stop_task(&self, id: &str) -> Result<(), String>;
    async fn update_status(&self, id: &str, status: &str) -> Result<(), String>;
    async fn get_output(&self, id: &str) -> Result<Option<String>, String>;
}

pub fn set_background_task_repository(repo: Arc<dyn BackgroundTaskRepository>) {
    get_service_registry().read().unwrap().set_background_task_repository(repo);
}

pub fn background_task_repository() -> Arc<dyn BackgroundTaskRepository> {
    get_service_registry().read().unwrap().background_task_repository()
}

// ── StoredFileRepository ──────────────────────

#[async_trait]
pub trait StoredFileRepository: Send + Sync {
    async fn create_stored_file(&self, input: CreateStoredFileInput) -> Result<StoredFile, String>;
    async fn get_stored_file(&self, id: &str) -> Result<StoredFile, String>;
    async fn list_all_stored_files(&self) -> Result<Vec<StoredFile>, String>;
    async fn update_storage_path(&self, id: &str, storage_path: &str) -> Result<(), String>;
}

pub fn set_stored_file_repository(repo: Arc<dyn StoredFileRepository>) {
    get_service_registry().read().unwrap().set_stored_file_repository(repo);
}

pub fn stored_file_repository() -> Arc<dyn StoredFileRepository> {
    get_service_registry().read().unwrap().stored_file_repository()
}

// ── KnowledgeEntityRepository ─────────────────

#[async_trait]
pub trait KnowledgeEntityRepository: Send + Sync {
    async fn insert_entity(
        &self,
        input: CreateKnowledgeEntityInput,
    ) -> Result<KnowledgeEntityDto, String>;
}

pub fn set_knowledge_entity_repository(repo: Arc<dyn KnowledgeEntityRepository>) {
    get_service_registry().read().unwrap().set_knowledge_entity_repository(repo);
}

pub fn knowledge_entity_repository() -> Arc<dyn KnowledgeEntityRepository> {
    get_service_registry().read().unwrap().knowledge_entity_repository()
}

// ── KnowledgeFlowRepository ───────────────────

#[async_trait]
pub trait KnowledgeFlowRepository: Send + Sync {
    async fn insert_flow(
        &self,
        input: CreateKnowledgeFlowInput,
    ) -> Result<KnowledgeFlowDto, String>;
}

pub fn set_knowledge_flow_repository(repo: Arc<dyn KnowledgeFlowRepository>) {
    get_service_registry().read().unwrap().set_knowledge_flow_repository(repo);
}

pub fn knowledge_flow_repository() -> Arc<dyn KnowledgeFlowRepository> {
    get_service_registry().read().unwrap().knowledge_flow_repository()
}

// ── KnowledgeInterfaceRepository ──────────────

#[async_trait]
pub trait KnowledgeInterfaceRepository: Send + Sync {
    async fn insert_interface(
        &self,
        input: CreateKnowledgeInterfaceInput,
    ) -> Result<KnowledgeInterfaceDto, String>;
}

pub fn set_knowledge_interface_repository(repo: Arc<dyn KnowledgeInterfaceRepository>) {
    get_service_registry().read().unwrap().set_knowledge_interface_repository(repo);
}

pub fn knowledge_interface_repository() -> Arc<dyn KnowledgeInterfaceRepository> {
    get_service_registry().read().unwrap().knowledge_interface_repository()
}

// ── KnowledgeDocumentRepository ───────────────

#[async_trait]
pub trait KnowledgeDocumentRepository: Send + Sync {
    async fn insert_document(
        &self,
        input: CreateKnowledgeDocumentInput,
    ) -> Result<KnowledgeDocumentDto, String>;
}

pub fn set_knowledge_document_repository(repo: Arc<dyn KnowledgeDocumentRepository>) {
    get_service_registry().read().unwrap().set_knowledge_document_repository(repo);
}

pub fn knowledge_document_repository() -> Arc<dyn KnowledgeDocumentRepository> {
    get_service_registry().read().unwrap().knowledge_document_repository()
}

// ── TrajectoryRepository ──────────────────────

/// 轨迹仓储 trait —— 封装所有 trajectory SeaORM 实体的 CRUD。
/// 使用 `DatabaseConnection` 作为后端，但 consumer 不直接引用 `axagent_entities`。
#[async_trait]
pub trait TrajectoryRepository: Send + Sync {
    /// 获取底层的数据库连接（供 trajectory/storage.rs 内部使用，
    /// 直到所有子操作迁移到 trait 方法）。
    fn db_connection(&self) -> &sea_orm::DatabaseConnection;
}

pub fn set_trajectory_repository(repo: Arc<dyn TrajectoryRepository>) {
    get_service_registry().read().unwrap().set_trajectory_repository(repo);
}

pub fn trajectory_repository() -> Arc<dyn TrajectoryRepository> {
    get_service_registry().read().unwrap().trajectory_repository()
}
