// SPDX-License-Identifier: AGPL-3.0-only

//! 依赖注入容器 —— ServiceRegistry
//!
//! 将 `repositories.rs` 中分散的 8 组 `OnceLock<RwLock<Option<Arc<T>>>>`
//! 全局可变状态集中到单一结构体，便于初始化管理、测试替换和未来迁移到真正 DI。

use std::sync::{Arc, OnceLock, RwLock};

use crate::repositories::{
    AgencyExpertRepository, AgentProfileRepository, AgentRoleRepository, BackgroundTaskRepository,
    ConversationRepository, DatabaseInitializer, GeneratedToolRepository,
    KnowledgeDocumentRepository, KnowledgeEntityRepository, KnowledgeFlowRepository,
    KnowledgeInterfaceRepository, LoopCheckpointRepository, MemoryRepository, MessageRepository,
    NoteBacklinkRepository, NoteRepository, PlatformConfigRepository, ProviderRepository,
    SessionRepository, SettingsRepository, SkillDirsProvider, StoredFileRepository,
    ToolExecutionRepository, TrajectoryRepository, WikiOperationRepository, WikiPageRepository,
    WikiRepository, WikiSourceRepository, WorkflowExecutionRepository, WorkflowTemplateRepository,
};

/// 全局服务注册表 —— 集中管理所有 repository 和 provider 的 DI 注入点。
///
/// 每个字段为 `OnceLock<RwLock<Option<Arc<T>>>>`，与原 scattered 模式保持
/// 相同的线程安全语义。
pub struct ServiceRegistry {
    pub note_repo: OnceLock<RwLock<Option<Arc<dyn NoteRepository>>>>,
    pub wiki_repo: OnceLock<RwLock<Option<Arc<dyn WikiRepository>>>>,
    pub wiki_page_repo: OnceLock<RwLock<Option<Arc<dyn WikiPageRepository>>>>,
    pub wiki_source_repo: OnceLock<RwLock<Option<Arc<dyn WikiSourceRepository>>>>,
    pub wiki_operation_repo: OnceLock<RwLock<Option<Arc<dyn WikiOperationRepository>>>>,
    pub backlink_repo: OnceLock<RwLock<Option<Arc<dyn NoteBacklinkRepository>>>>,
    pub settings_repo: OnceLock<RwLock<Option<Arc<dyn SettingsRepository>>>>,
    pub session_repo: OnceLock<RwLock<Option<Arc<dyn SessionRepository>>>>,
    pub provider_repo: OnceLock<RwLock<Option<Arc<dyn ProviderRepository>>>>,
    pub generated_tool_repo: OnceLock<RwLock<Option<Arc<dyn GeneratedToolRepository>>>>,
    pub platform_config_repo: OnceLock<RwLock<Option<Arc<dyn PlatformConfigRepository>>>>,
    pub conversation_repo: OnceLock<RwLock<Option<Arc<dyn ConversationRepository>>>>,
    pub message_repo: OnceLock<RwLock<Option<Arc<dyn MessageRepository>>>>,
    pub tool_execution_repo: OnceLock<RwLock<Option<Arc<dyn ToolExecutionRepository>>>>,
    pub memory_repo: OnceLock<RwLock<Option<Arc<dyn MemoryRepository>>>>,
    pub workflow_execution_repo: OnceLock<RwLock<Option<Arc<dyn WorkflowExecutionRepository>>>>,
    pub loop_checkpoint_repo: OnceLock<RwLock<Option<Arc<dyn LoopCheckpointRepository>>>>,
    pub workflow_template_repo: OnceLock<RwLock<Option<Arc<dyn WorkflowTemplateRepository>>>>,
    pub background_task_repo: OnceLock<RwLock<Option<Arc<dyn BackgroundTaskRepository>>>>,
    pub stored_file_repo: OnceLock<RwLock<Option<Arc<dyn StoredFileRepository>>>>,
    pub knowledge_entity_repo: OnceLock<RwLock<Option<Arc<dyn KnowledgeEntityRepository>>>>,
    pub knowledge_flow_repo: OnceLock<RwLock<Option<Arc<dyn KnowledgeFlowRepository>>>>,
    pub knowledge_interface_repo: OnceLock<RwLock<Option<Arc<dyn KnowledgeInterfaceRepository>>>>,
    pub knowledge_document_repo: OnceLock<RwLock<Option<Arc<dyn KnowledgeDocumentRepository>>>>,
    pub trajectory_repo: OnceLock<RwLock<Option<Arc<dyn TrajectoryRepository>>>>,
    pub agent_profile_repo: OnceLock<RwLock<Option<Arc<dyn AgentProfileRepository>>>>,
    pub agency_expert_repo: OnceLock<RwLock<Option<Arc<dyn AgencyExpertRepository>>>>,
    pub agent_role_repo: OnceLock<RwLock<Option<Arc<dyn AgentRoleRepository>>>>,
    pub db_init: OnceLock<RwLock<Option<Arc<dyn DatabaseInitializer>>>>,
    pub skill_dirs: OnceLock<RwLock<Option<Arc<dyn SkillDirsProvider>>>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self {
            note_repo: OnceLock::new(),
            wiki_repo: OnceLock::new(),
            wiki_page_repo: OnceLock::new(),
            wiki_source_repo: OnceLock::new(),
            wiki_operation_repo: OnceLock::new(),
            backlink_repo: OnceLock::new(),
            settings_repo: OnceLock::new(),
            session_repo: OnceLock::new(),
            provider_repo: OnceLock::new(),
            generated_tool_repo: OnceLock::new(),
            platform_config_repo: OnceLock::new(),
            conversation_repo: OnceLock::new(),
            message_repo: OnceLock::new(),
            tool_execution_repo: OnceLock::new(),
            memory_repo: OnceLock::new(),
            workflow_execution_repo: OnceLock::new(),
            loop_checkpoint_repo: OnceLock::new(),
            workflow_template_repo: OnceLock::new(),
            background_task_repo: OnceLock::new(),
            stored_file_repo: OnceLock::new(),
            knowledge_entity_repo: OnceLock::new(),
            knowledge_flow_repo: OnceLock::new(),
            knowledge_interface_repo: OnceLock::new(),
            knowledge_document_repo: OnceLock::new(),
            trajectory_repo: OnceLock::new(),
            agent_profile_repo: OnceLock::new(),
            agency_expert_repo: OnceLock::new(),
            agent_role_repo: OnceLock::new(),
            db_init: OnceLock::new(),
            skill_dirs: OnceLock::new(),
        }
    }

    // ── NoteRepository ──

    pub fn set_note_repository(&self, repo: Arc<dyn NoteRepository>) {
        self.note_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn note_repository(&self) -> Arc<dyn NoteRepository> {
        self.note_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone().expect(
            "NoteRepository not initialized. Call set_note_repository() during app startup.",
        )
    }

    // ── WikiRepository ──

    pub fn set_wiki_repository(&self, repo: Arc<dyn WikiRepository>) {
        self.wiki_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn wiki_repository(&self) -> Arc<dyn WikiRepository> {
        self.wiki_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WikiRepository not initialized.")
    }

    // ── WikiPageRepository ──

    pub fn set_wiki_page_repository(&self, repo: Arc<dyn WikiPageRepository>) {
        self.wiki_page_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn wiki_page_repository(&self) -> Arc<dyn WikiPageRepository> {
        self.wiki_page_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WikiPageRepository not initialized.")
    }

    // ── WikiSourceRepository ──

    pub fn set_wiki_source_repository(&self, repo: Arc<dyn WikiSourceRepository>) {
        self.wiki_source_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn wiki_source_repository(&self) -> Arc<dyn WikiSourceRepository> {
        self.wiki_source_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WikiSourceRepository not initialized.")
    }

    // ── WikiOperationRepository ──

    pub fn set_wiki_operation_repository(&self, repo: Arc<dyn WikiOperationRepository>) {
        self.wiki_operation_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn wiki_operation_repository(&self) -> Arc<dyn WikiOperationRepository> {
        self.wiki_operation_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WikiOperationRepository not initialized.")
    }

    // ── NoteBacklinkRepository ──

    pub fn set_note_backlink_repository(&self, repo: Arc<dyn NoteBacklinkRepository>) {
        self.backlink_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn note_backlink_repository(&self) -> Arc<dyn NoteBacklinkRepository> {
        self.backlink_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("NoteBacklinkRepository not initialized.")
    }

    // ── SettingsRepository ──

    pub fn set_settings_repository(&self, repo: Arc<dyn SettingsRepository>) {
        self.settings_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn settings_repository(&self) -> Arc<dyn SettingsRepository> {
        self.settings_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("SettingsRepository not initialized.")
    }

    // ── ProviderRepository ──

    pub fn set_provider_repository(&self, repo: Arc<dyn ProviderRepository>) {
        self.provider_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn provider_repository(&self) -> Arc<dyn ProviderRepository> {
        self.provider_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("ProviderRepository not initialized.")
    }

    // ── GeneratedToolRepository ──

    pub fn set_generated_tool_repository(&self, repo: Arc<dyn GeneratedToolRepository>) {
        self.generated_tool_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn generated_tool_repository(&self) -> Arc<dyn GeneratedToolRepository> {
        self.generated_tool_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("GeneratedToolRepository not initialized.")
    }

    // ── PlatformConfigRepository ──

    pub fn set_platform_config_repository(&self, repo: Arc<dyn PlatformConfigRepository>) {
        self.platform_config_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn platform_config_repository(&self) -> Arc<dyn PlatformConfigRepository> {
        self.platform_config_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("PlatformConfigRepository not initialized.")
    }

    // ── ConversationRepository ──

    pub fn set_conversation_repository(&self, repo: Arc<dyn ConversationRepository>) {
        self.conversation_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn conversation_repository(&self) -> Arc<dyn ConversationRepository> {
        self.conversation_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("ConversationRepository not initialized.")
    }

    // ── MessageRepository ──

    pub fn set_message_repository(&self, repo: Arc<dyn MessageRepository>) {
        self.message_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn message_repository(&self) -> Arc<dyn MessageRepository> {
        self.message_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("MessageRepository not initialized.")
    }

    // ── SessionRepository ──

    pub fn set_session_repository(&self, repo: Arc<dyn SessionRepository>) {
        self.session_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn session_repository(&self) -> Arc<dyn SessionRepository> {
        self.session_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("SessionRepository not initialized.")
    }

    // ── DatabaseInitializer ──

    pub fn set_database_initializer(&self, init: Arc<dyn DatabaseInitializer>) {
        self.db_init.get_or_init(|| RwLock::new(None)).write().unwrap().replace(init);
    }

    pub fn database_initializer(&self) -> Arc<dyn DatabaseInitializer> {
        self.db_init
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("DatabaseInitializer not initialized.")
    }

    // ── SkillDirsProvider ──

    pub fn set_skill_dirs_provider(&self, provider: Arc<dyn SkillDirsProvider>) {
        self.skill_dirs.get_or_init(|| RwLock::new(None)).write().unwrap().replace(provider);
    }

    pub fn skill_dirs_provider(&self) -> Arc<dyn SkillDirsProvider> {
        self.skill_dirs
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("SkillDirsProvider not initialized.")
    }

    // ── ToolExecutionRepository ──

    pub fn set_tool_execution_repository(&self, repo: Arc<dyn ToolExecutionRepository>) {
        self.tool_execution_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn tool_execution_repository(&self) -> Arc<dyn ToolExecutionRepository> {
        self.tool_execution_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("ToolExecutionRepository not initialized.")
    }

    // ── WorkflowExecutionRepository ──

    pub fn set_workflow_execution_repository(&self, repo: Arc<dyn WorkflowExecutionRepository>) {
        self.workflow_execution_repo
            .get_or_init(|| RwLock::new(None))
            .write()
            .unwrap()
            .replace(repo);
    }

    pub fn workflow_execution_repository(&self) -> Arc<dyn WorkflowExecutionRepository> {
        self.workflow_execution_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WorkflowExecutionRepository not initialized.")
    }

    // ── LoopCheckpointRepository ──

    pub fn set_loop_checkpoint_repository(&self, repo: Arc<dyn LoopCheckpointRepository>) {
        self.loop_checkpoint_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn loop_checkpoint_repository(&self) -> Arc<dyn LoopCheckpointRepository> {
        self.loop_checkpoint_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("LoopCheckpointRepository not initialized.")
    }

    // ── WorkflowTemplateRepository ──

    pub fn set_workflow_template_repository(&self, repo: Arc<dyn WorkflowTemplateRepository>) {
        self.workflow_template_repo
            .get_or_init(|| RwLock::new(None))
            .write()
            .unwrap()
            .replace(repo);
    }

    pub fn workflow_template_repository(&self) -> Arc<dyn WorkflowTemplateRepository> {
        self.workflow_template_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("WorkflowTemplateRepository not initialized.")
    }

    // ── MemoryRepository ──

    pub fn set_memory_repository(&self, repo: Arc<dyn MemoryRepository>) {
        self.memory_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn memory_repository(&self) -> Arc<dyn MemoryRepository> {
        self.memory_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("MemoryRepository not initialized.")
    }

    pub fn memory_repository_opt(&self) -> Option<Arc<dyn MemoryRepository>> {
        self.memory_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    // ── BackgroundTaskRepository ──

    pub fn set_background_task_repository(&self, repo: Arc<dyn BackgroundTaskRepository>) {
        self.background_task_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn background_task_repository(&self) -> Arc<dyn BackgroundTaskRepository> {
        self.background_task_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("BackgroundTaskRepository not initialized.")
    }

    // ── StoredFileRepository ──

    pub fn set_stored_file_repository(&self, repo: Arc<dyn StoredFileRepository>) {
        self.stored_file_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn stored_file_repository(&self) -> Arc<dyn StoredFileRepository> {
        self.stored_file_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("StoredFileRepository not initialized.")
    }

    // ── KnowledgeEntityRepository ──

    pub fn set_knowledge_entity_repository(&self, repo: Arc<dyn KnowledgeEntityRepository>) {
        self.knowledge_entity_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn knowledge_entity_repository(&self) -> Arc<dyn KnowledgeEntityRepository> {
        self.knowledge_entity_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("KnowledgeEntityRepository not initialized.")
    }

    // ── KnowledgeFlowRepository ──

    pub fn set_knowledge_flow_repository(&self, repo: Arc<dyn KnowledgeFlowRepository>) {
        self.knowledge_flow_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn knowledge_flow_repository(&self) -> Arc<dyn KnowledgeFlowRepository> {
        self.knowledge_flow_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("KnowledgeFlowRepository not initialized.")
    }

    // ── KnowledgeInterfaceRepository ──

    pub fn set_knowledge_interface_repository(&self, repo: Arc<dyn KnowledgeInterfaceRepository>) {
        self.knowledge_interface_repo
            .get_or_init(|| RwLock::new(None))
            .write()
            .unwrap()
            .replace(repo);
    }

    pub fn knowledge_interface_repository(&self) -> Arc<dyn KnowledgeInterfaceRepository> {
        self.knowledge_interface_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("KnowledgeInterfaceRepository not initialized.")
    }

    // ── KnowledgeDocumentRepository ──

    pub fn set_knowledge_document_repository(&self, repo: Arc<dyn KnowledgeDocumentRepository>) {
        self.knowledge_document_repo
            .get_or_init(|| RwLock::new(None))
            .write()
            .unwrap()
            .replace(repo);
    }

    pub fn knowledge_document_repository(&self) -> Arc<dyn KnowledgeDocumentRepository> {
        self.knowledge_document_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("KnowledgeDocumentRepository not initialized.")
    }

    // ── TrajectoryRepository ──

    pub fn set_trajectory_repository(&self, repo: Arc<dyn TrajectoryRepository>) {
        self.trajectory_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn trajectory_repository(&self) -> Arc<dyn TrajectoryRepository> {
        self.trajectory_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("TrajectoryRepository not initialized.")
    }

    // ── AgentProfileRepository ──

    pub fn set_agent_profile_repository(&self, repo: Arc<dyn AgentProfileRepository>) {
        self.agent_profile_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn agent_profile_repository(&self) -> Arc<dyn AgentProfileRepository> {
        self.agent_profile_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("AgentProfileRepository not initialized.")
    }

    // ── AgencyExpertRepository ──

    pub fn set_agency_expert_repository(&self, repo: Arc<dyn AgencyExpertRepository>) {
        self.agency_expert_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn agency_expert_repository(&self) -> Arc<dyn AgencyExpertRepository> {
        self.agency_expert_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("AgencyExpertRepository not initialized.")
    }

    // ── AgentRoleRepository ──

    pub fn set_agent_role_repository(&self, repo: Arc<dyn AgentRoleRepository>) {
        self.agent_role_repo.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
    }

    pub fn agent_role_repository(&self) -> Arc<dyn AgentRoleRepository> {
        self.agent_role_repo
            .get_or_init(|| RwLock::new(None))
            .read()
            .unwrap()
            .clone()
            .expect("AgentRoleRepository not initialized.")
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── ServiceRegistryProvider trait ──────────────────────────────────────

/// DI 容器抽象 trait —— consumer crate 通过此 trait 获取 repository，不依赖具体实现。
///
/// 与 `ServiceRegistry` struct 上的 inherent 方法不同，trait 方法返回 `Option<Arc<dyn T>>`：
/// - 未初始化时返回 `None`，不 panic
/// - 适合 consumer crate（agent / orchestrator / gateway 等）通过 `Arc<dyn ServiceRegistryProvider>`
///   进行依赖注入，避免对具体 struct 的直接依赖
///
/// struct 的 inherent 方法（如 `note_repository()` 返回 `Arc<dyn T>` 并 panic）保持不变，
/// 以维持向后兼容；新增的 trait 方法提供更宽容的接口。
pub trait ServiceRegistryProvider: Send + Sync {
    fn note_repository(&self) -> Option<Arc<dyn NoteRepository>>;
    fn wiki_repository(&self) -> Option<Arc<dyn WikiRepository>>;
    fn wiki_page_repository(&self) -> Option<Arc<dyn WikiPageRepository>>;
    fn wiki_source_repository(&self) -> Option<Arc<dyn WikiSourceRepository>>;
    fn wiki_operation_repository(&self) -> Option<Arc<dyn WikiOperationRepository>>;
    fn note_backlink_repository(&self) -> Option<Arc<dyn NoteBacklinkRepository>>;
    fn settings_repository(&self) -> Option<Arc<dyn SettingsRepository>>;
    fn session_repository(&self) -> Option<Arc<dyn SessionRepository>>;
    fn provider_repository(&self) -> Option<Arc<dyn ProviderRepository>>;
    fn generated_tool_repository(&self) -> Option<Arc<dyn GeneratedToolRepository>>;
    fn platform_config_repository(&self) -> Option<Arc<dyn PlatformConfigRepository>>;
    fn conversation_repository(&self) -> Option<Arc<dyn ConversationRepository>>;
    fn message_repository(&self) -> Option<Arc<dyn MessageRepository>>;
    fn tool_execution_repository(&self) -> Option<Arc<dyn ToolExecutionRepository>>;
    fn memory_repository(&self) -> Option<Arc<dyn MemoryRepository>>;
    fn workflow_execution_repository(&self) -> Option<Arc<dyn WorkflowExecutionRepository>>;
    fn loop_checkpoint_repository(&self) -> Option<Arc<dyn LoopCheckpointRepository>>;
    fn workflow_template_repository(&self) -> Option<Arc<dyn WorkflowTemplateRepository>>;
    fn background_task_repository(&self) -> Option<Arc<dyn BackgroundTaskRepository>>;
    fn stored_file_repository(&self) -> Option<Arc<dyn StoredFileRepository>>;
    fn knowledge_entity_repository(&self) -> Option<Arc<dyn KnowledgeEntityRepository>>;
    fn knowledge_flow_repository(&self) -> Option<Arc<dyn KnowledgeFlowRepository>>;
    fn knowledge_interface_repository(&self) -> Option<Arc<dyn KnowledgeInterfaceRepository>>;
    fn knowledge_document_repository(&self) -> Option<Arc<dyn KnowledgeDocumentRepository>>;
    fn trajectory_repository(&self) -> Option<Arc<dyn TrajectoryRepository>>;
    fn agent_profile_repository(&self) -> Option<Arc<dyn AgentProfileRepository>>;
    fn agency_expert_repository(&self) -> Option<Arc<dyn AgencyExpertRepository>>;
    fn agent_role_repository(&self) -> Option<Arc<dyn AgentRoleRepository>>;
    fn database_initializer(&self) -> Option<Arc<dyn DatabaseInitializer>>;
    fn skill_dirs_provider(&self) -> Option<Arc<dyn SkillDirsProvider>>;
}

impl ServiceRegistryProvider for ServiceRegistry {
    fn note_repository(&self) -> Option<Arc<dyn NoteRepository>> {
        self.note_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn wiki_repository(&self) -> Option<Arc<dyn WikiRepository>> {
        self.wiki_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn wiki_page_repository(&self) -> Option<Arc<dyn WikiPageRepository>> {
        self.wiki_page_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn wiki_source_repository(&self) -> Option<Arc<dyn WikiSourceRepository>> {
        self.wiki_source_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn wiki_operation_repository(&self) -> Option<Arc<dyn WikiOperationRepository>> {
        self.wiki_operation_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn note_backlink_repository(&self) -> Option<Arc<dyn NoteBacklinkRepository>> {
        self.backlink_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn settings_repository(&self) -> Option<Arc<dyn SettingsRepository>> {
        self.settings_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn session_repository(&self) -> Option<Arc<dyn SessionRepository>> {
        self.session_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn provider_repository(&self) -> Option<Arc<dyn ProviderRepository>> {
        self.provider_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn generated_tool_repository(&self) -> Option<Arc<dyn GeneratedToolRepository>> {
        self.generated_tool_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn platform_config_repository(&self) -> Option<Arc<dyn PlatformConfigRepository>> {
        self.platform_config_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn conversation_repository(&self) -> Option<Arc<dyn ConversationRepository>> {
        self.conversation_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn message_repository(&self) -> Option<Arc<dyn MessageRepository>> {
        self.message_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn tool_execution_repository(&self) -> Option<Arc<dyn ToolExecutionRepository>> {
        self.tool_execution_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn memory_repository(&self) -> Option<Arc<dyn MemoryRepository>> {
        self.memory_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn workflow_execution_repository(&self) -> Option<Arc<dyn WorkflowExecutionRepository>> {
        self.workflow_execution_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn loop_checkpoint_repository(&self) -> Option<Arc<dyn LoopCheckpointRepository>> {
        self.loop_checkpoint_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn workflow_template_repository(&self) -> Option<Arc<dyn WorkflowTemplateRepository>> {
        self.workflow_template_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn background_task_repository(&self) -> Option<Arc<dyn BackgroundTaskRepository>> {
        self.background_task_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn stored_file_repository(&self) -> Option<Arc<dyn StoredFileRepository>> {
        self.stored_file_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn knowledge_entity_repository(&self) -> Option<Arc<dyn KnowledgeEntityRepository>> {
        self.knowledge_entity_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn knowledge_flow_repository(&self) -> Option<Arc<dyn KnowledgeFlowRepository>> {
        self.knowledge_flow_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn knowledge_interface_repository(&self) -> Option<Arc<dyn KnowledgeInterfaceRepository>> {
        self.knowledge_interface_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn knowledge_document_repository(&self) -> Option<Arc<dyn KnowledgeDocumentRepository>> {
        self.knowledge_document_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn trajectory_repository(&self) -> Option<Arc<dyn TrajectoryRepository>> {
        self.trajectory_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn agent_profile_repository(&self) -> Option<Arc<dyn AgentProfileRepository>> {
        self.agent_profile_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn agency_expert_repository(&self) -> Option<Arc<dyn AgencyExpertRepository>> {
        self.agency_expert_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn agent_role_repository(&self) -> Option<Arc<dyn AgentRoleRepository>> {
        self.agent_role_repo.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn database_initializer(&self) -> Option<Arc<dyn DatabaseInitializer>> {
        self.db_init.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }

    fn skill_dirs_provider(&self) -> Option<Arc<dyn SkillDirsProvider>> {
        self.skill_dirs.get_or_init(|| RwLock::new(None)).read().unwrap().clone()
    }
}

/// 全局服务注册表实例 —— 向后兼容过渡方案。
///
/// 后续可逐步迁移所有调用方到显式 DI 注入。
pub static SERVICE_REGISTRY: OnceLock<RwLock<ServiceRegistry>> = OnceLock::new();

/// 获取全局 ServiceRegistry 的引用。
/// 若尚未初始化则自动创建默认实例。
pub fn get_service_registry() -> &'static RwLock<ServiceRegistry> {
    SERVICE_REGISTRY.get_or_init(|| RwLock::new(ServiceRegistry::new()))
}
