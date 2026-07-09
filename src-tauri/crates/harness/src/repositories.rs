// SPDX-License-Identifier: AGPL-3.0-only

//! 仓储 trait 定义 + 全局依赖注入点。
//!
//! 每个 trait 对应一个实体维度。consumer crate（agent/orchestrator/tools）
//! 通过 `repo_accessor()` 获取实现，不直接依赖 `axagent-dao` 或 `axagent-entities`。

use std::sync::Arc;

use async_trait::async_trait;

use crate::repo_dtos::*;
use crate::service_registry::get_service_registry;
use crate::types::AppSettings;
use crate::types::Conversation;
use crate::types::Message;
use crate::types::MessageRole;
use crate::types::ProviderConfig;
use crate::types::ProviderKey;
use std::collections::HashMap;

// ── NoteRepository ─────────────────────────────

#[async_trait]
pub trait NoteRepository: Send + Sync {
    async fn list_notes(&self, wiki_id: &str) -> Result<Vec<Note>, String>;
    async fn get_note_by_id(&self, id: &str) -> Result<Option<Note>, String>;
    async fn get_note_by_title(&self, wiki_id: &str, title: &str) -> Result<Option<Note>, String>;
    async fn create_note(&self, input: CreateNoteInput) -> Result<Note, String>;
    async fn update_note(&self, input: UpdateNoteInput) -> Result<Note, String>;
    async fn delete_note(&self, id: &str) -> Result<(), String>;
    async fn search_notes(&self, wiki_id: &str, query: &str) -> Result<Vec<Note>, String>;
    async fn count_notes(&self, wiki_id: &str) -> Result<i64, String>;
}

pub fn set_note_repository(repo: Arc<dyn NoteRepository>) {
    get_service_registry().read().unwrap().set_note_repository(repo);
}

pub fn note_repository() -> Arc<dyn NoteRepository> {
    get_service_registry().read().unwrap().note_repository()
}

// ── WikiRepository ─────────────────────────────

#[async_trait]
pub trait WikiRepository: Send + Sync {
    async fn list_wikis(&self) -> Result<Vec<Wiki>, String>;
    async fn get_wiki_by_id(&self, id: &str) -> Result<Option<Wiki>, String>;
    async fn create_wiki(&self, name: &str, description: Option<String>) -> Result<Wiki, String>;
    async fn delete_wiki(&self, id: &str) -> Result<(), String>;
}

pub fn set_wiki_repository(repo: Arc<dyn WikiRepository>) {
    get_service_registry().read().unwrap().set_wiki_repository(repo);
}

pub fn wiki_repository() -> Arc<dyn WikiRepository> {
    get_service_registry().read().unwrap().wiki_repository()
}

// ── WikiPageRepository ─────────────────────────

#[async_trait]
pub trait WikiPageRepository: Send + Sync {
    async fn list_pages(&self, wiki_id: &str) -> Result<Vec<WikiPage>, String>;
    async fn get_page_by_id(&self, id: &str) -> Result<Option<WikiPage>, String>;
    async fn get_page_by_title(
        &self,
        wiki_id: &str,
        title: &str,
    ) -> Result<Option<WikiPage>, String>;
    async fn create_page(
        &self,
        wiki_id: &str,
        title: &str,
        content: &str,
    ) -> Result<WikiPage, String>;
    async fn update_page(&self, id: &str, content: Option<String>) -> Result<WikiPage, String>;
    async fn delete_page(&self, id: &str) -> Result<(), String>;
}

pub fn set_wiki_page_repository(repo: Arc<dyn WikiPageRepository>) {
    get_service_registry().read().unwrap().set_wiki_page_repository(repo);
}

pub fn wiki_page_repository() -> Arc<dyn WikiPageRepository> {
    get_service_registry().read().unwrap().wiki_page_repository()
}

// ── WikiSourceRepository ───────────────────────

#[async_trait]
pub trait WikiSourceRepository: Send + Sync {
    async fn list_sources(&self, wiki_id: &str) -> Result<Vec<WikiSource>, String>;
    async fn create_source(
        &self,
        wiki_id: &str,
        url: &str,
        title: Option<String>,
    ) -> Result<WikiSource, String>;
    async fn delete_source(&self, id: &str) -> Result<(), String>;
}

pub fn set_wiki_source_repository(repo: Arc<dyn WikiSourceRepository>) {
    get_service_registry().read().unwrap().set_wiki_source_repository(repo);
}

pub fn wiki_source_repository() -> Arc<dyn WikiSourceRepository> {
    get_service_registry().read().unwrap().wiki_source_repository()
}

// ── NoteBacklinkRepository ─────────────────────

#[async_trait]
pub trait NoteBacklinkRepository: Send + Sync {
    async fn list_backlinks(&self, note_id: &str) -> Result<Vec<NoteBacklink>, String>;
    async fn create_backlink(
        &self,
        source_note_id: &str,
        target_note_id: &str,
        context: Option<String>,
    ) -> Result<NoteBacklink, String>;
}

pub fn set_note_backlink_repository(repo: Arc<dyn NoteBacklinkRepository>) {
    get_service_registry().read().unwrap().set_note_backlink_repository(repo);
}

pub fn note_backlink_repository() -> Arc<dyn NoteBacklinkRepository> {
    get_service_registry().read().unwrap().note_backlink_repository()
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
        total_time_ms: Option<i32>,
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
