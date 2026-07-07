// SPDX-License-Identifier: AGPL-3.0-only

//! 仓储 trait 定义 + 全局依赖注入点。
//!
//! 每个 trait 对应一个实体维度。consumer crate（agent/orchestrator/tools）
//! 通过 `repo_accessor()` 获取实现，不直接依赖 `axagent-dao` 或 `axagent-entities`。

use std::sync::{Arc, OnceLock, RwLock};

use async_trait::async_trait;

use crate::repo_dtos::*;

// ── 全局注入容器 ──────────────────────────────

static NOTE_REPO: OnceLock<RwLock<Option<Arc<dyn NoteRepository>>>> = OnceLock::new();
static WIKI_REPO: OnceLock<RwLock<Option<Arc<dyn WikiRepository>>>> = OnceLock::new();
static WIKI_PAGE_REPO: OnceLock<RwLock<Option<Arc<dyn WikiPageRepository>>>> = OnceLock::new();
static WIKI_SOURCE_REPO: OnceLock<RwLock<Option<Arc<dyn WikiSourceRepository>>>> = OnceLock::new();
static BACKLINK_REPO: OnceLock<RwLock<Option<Arc<dyn NoteBacklinkRepository>>>> = OnceLock::new();
static SETTINGS_REPO: OnceLock<RwLock<Option<Arc<dyn SettingsRepository>>>> = OnceLock::new();
static SESSION_REPO: OnceLock<RwLock<Option<Arc<dyn SessionRepository>>>> = OnceLock::new();

fn get_or_init<T>() -> &'static RwLock<Option<Arc<T>>>
where
    T: 'static,
{
    // Each call allocates a new OnceLock — this is a placeholder.
    // In practice, each concrete type has its own static.
    panic!("get_or_init should not be called directly; use typed accessors")
}

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
    NOTE_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn note_repository() -> Arc<dyn NoteRepository> {
    NOTE_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("NoteRepository not initialized. Call set_note_repository() during app startup.")
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
    WIKI_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn wiki_repository() -> Arc<dyn WikiRepository> {
    WIKI_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("WikiRepository not initialized.")
}

// ── WikiPageRepository ─────────────────────────

#[async_trait]
pub trait WikiPageRepository: Send + Sync {
    async fn list_pages(&self, wiki_id: &str) -> Result<Vec<WikiPage>, String>;
    async fn get_page_by_id(&self, id: &str) -> Result<Option<WikiPage>, String>;
    async fn get_page_by_title(&self, wiki_id: &str, title: &str) -> Result<Option<WikiPage>, String>;
    async fn create_page(&self, wiki_id: &str, title: &str, content: &str) -> Result<WikiPage, String>;
    async fn update_page(&self, id: &str, content: Option<String>) -> Result<WikiPage, String>;
    async fn delete_page(&self, id: &str) -> Result<(), String>;
}

pub fn set_wiki_page_repository(repo: Arc<dyn WikiPageRepository>) {
    WIKI_PAGE_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn wiki_page_repository() -> Arc<dyn WikiPageRepository> {
    WIKI_PAGE_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("WikiPageRepository not initialized.")
}

// ── WikiSourceRepository ───────────────────────

#[async_trait]
pub trait WikiSourceRepository: Send + Sync {
    async fn list_sources(&self, wiki_id: &str) -> Result<Vec<WikiSource>, String>;
    async fn create_source(&self, wiki_id: &str, url: &str, title: Option<String>) -> Result<WikiSource, String>;
    async fn delete_source(&self, id: &str) -> Result<(), String>;
}

pub fn set_wiki_source_repository(repo: Arc<dyn WikiSourceRepository>) {
    WIKI_SOURCE_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn wiki_source_repository() -> Arc<dyn WikiSourceRepository> {
    WIKI_SOURCE_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("WikiSourceRepository not initialized.")
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
    BACKLINK_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn note_backlink_repository() -> Arc<dyn NoteBacklinkRepository> {
    BACKLINK_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("NoteBacklinkRepository not initialized.")
}

// ── SettingsRepository ─────────────────────────

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_setting(&self, key: &str) -> Result<Option<String>, String>;
    async fn set_setting(&self, key: &str, value: &str) -> Result<(), String>;
}

pub fn set_settings_repository(repo: Arc<dyn SettingsRepository>) {
    SETTINGS_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn settings_repository() -> Arc<dyn SettingsRepository> {
    SETTINGS_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("SettingsRepository not initialized.")
}

// ── SessionRepository ──────────────────────────

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn list_sessions(&self) -> Result<Vec<SessionRecord>, String>;
    async fn get_session(&self, id: &str) -> Result<Option<SessionRecord>, String>;
    async fn delete_session(&self, id: &str) -> Result<(), String>;
}

pub fn set_session_repository(repo: Arc<dyn SessionRepository>) {
    SESSION_REPO.get_or_init(|| RwLock::new(None)).write().unwrap().replace(repo);
}

pub fn session_repository() -> Arc<dyn SessionRepository> {
    SESSION_REPO
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("SessionRepository not initialized.")
}

// ── DatabaseInitializer ─────────────────────────

#[async_trait]
pub trait DatabaseInitializer: Send + Sync {
    async fn run_initialization(&self) -> Result<(), String>;
}

static DB_INIT: OnceLock<RwLock<Option<Arc<dyn DatabaseInitializer>>>> = OnceLock::new();

pub fn set_database_initializer(init: Arc<dyn DatabaseInitializer>) {
    DB_INIT.get_or_init(|| RwLock::new(None)).write().unwrap().replace(init);
}

pub fn database_initializer() -> Arc<dyn DatabaseInitializer> {
    DB_INIT
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("DatabaseInitializer not initialized.")
}

// ── SkillDirsProvider ──────────────────────────

pub trait SkillDirsProvider: Send + Sync {
    fn skill_dirs(&self) -> Vec<String>;
}

static SKILL_DIRS: OnceLock<RwLock<Option<Arc<dyn SkillDirsProvider>>>> = OnceLock::new();

pub fn set_skill_dirs_provider(provider: Arc<dyn SkillDirsProvider>) {
    SKILL_DIRS.get_or_init(|| RwLock::new(None)).write().unwrap().replace(provider);
}

pub fn skill_dirs_provider() -> Arc<dyn SkillDirsProvider> {
    SKILL_DIRS
        .get_or_init(|| RwLock::new(None))
        .read()
        .unwrap()
        .clone()
        .expect("SkillDirsProvider not initialized.")
}
