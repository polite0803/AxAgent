// SPDX-License-Identifier: AGPL-3.0-only

//! 依赖注入容器 —— ServiceRegistry
//!
//! 将 `repositories.rs` 中分散的 8 组 `OnceLock<RwLock<Option<Arc<T>>>>`
//! 全局可变状态集中到单一结构体，便于初始化管理、测试替换和未来迁移到真正 DI。

use std::sync::{Arc, OnceLock, RwLock};

use crate::repositories::{
    ConversationRepository, DatabaseInitializer, GeneratedToolRepository,
    MemoryRepository, MessageRepository, NoteBacklinkRepository, NoteRepository,
    PlatformConfigRepository, ProviderRepository, SessionRepository, SettingsRepository,
    SkillDirsProvider, ToolExecutionRepository,
    WikiPageRepository, WikiRepository, WikiSourceRepository,
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
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
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
