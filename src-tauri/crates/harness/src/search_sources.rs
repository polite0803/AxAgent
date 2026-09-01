// SPDX-License-Identifier: AGPL-3.0-only

//! 搜索层访问 dao + document-parser 的 trait 抽象。
//!
//! search crate 不再直接依赖 axagent-dao / axagent-document-parser，
//! 改为依赖本文件定义的 5 个 trait，并由 wiring 层在启动时注入实现。
//!
//! ## 统一知识源抽象（v2）
//!
//! `UnifiedKnowledgeSource` trait 提供了对 RAG/Wiki/Memory/Obsidian 四类知识源的统一访问接口，
//! 上层调用方不需要感知源类型差异，通过 `source_type()` 即可区分。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::core_error::Result;
use crate::types::{AppSettings, KnowledgeBase, KnowledgeEntity, MemoryNamespace, NoteLink, Wiki};

// ── 现有 trait ──────────────────────────────────────────────────

/// 知识库（knowledge_base 容器 + 文档元数据 + KG 实体）的访问入口。
#[async_trait]
pub trait KnowledgeSource: Send + Sync {
    async fn get_knowledge_base(&self, id: &str) -> Result<KnowledgeBase>;
    async fn get_document_titles(&self, doc_ids: &[String]) -> Result<HashMap<String, String>>;
    async fn search_entities(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>>;
}

/// 记忆命名空间（memory 容器）的访问入口。
#[async_trait]
pub trait MemorySource: Send + Sync {
    async fn get_namespace(&self, id: &str) -> Result<MemoryNamespace>;
}

/// Wiki 仓库（vault + 反向链接）的访问入口。
#[async_trait]
pub trait WikiSource: Send + Sync {
    async fn get_wiki(&self, id: &str) -> Result<Wiki>;
    async fn get_note_backlinks_by_vault(&self, vault_id: &str) -> Result<Vec<NoteLink>>;
    /// 批量查询笔记标题（R7：wiki 检索命中项的 citation document_name 回填）。
    /// 返回 note_id → title 映射；查不到的 id 不出现在结果中。
    async fn get_note_titles(&self, note_ids: &[String]) -> Result<HashMap<String, String>>;
}

/// 系统设置的访问入口。
#[async_trait]
pub trait SettingsSource: Send + Sync {
    async fn get_settings(&self) -> Result<AppSettings>;
}

/// 文档解析（从文件路径提取纯文本）的访问入口。
///
/// 与异步 dao trait 不同，文档解析是同步阻塞调用 — extract_text
/// 内部是纯文件 I/O + 解析，无需数据库句柄。
pub trait DocumentParser: Send + Sync {
    fn extract_text(&self, file_path: &Path, mime_type: &str) -> Result<String>;
}

// ── 统一知识源抽象（v2）──────────────────────────────────────────

/// 知识源类型标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnowledgeSourceType {
    /// RAG 知识库（knowledge_base + 文档 + KG 实体）
    KnowledgeBase,
    /// Wiki 仓库（vault + note + 反向链接）
    Wiki,
    /// 记忆命名空间（memory item + 关联）
    Memory,
    /// Obsidian Vault（外部 vault 文件系统）
    ObsidianVault,
}

impl KnowledgeSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KnowledgeBase => "knowledge_base",
            Self::Wiki => "wiki",
            Self::Memory => "memory",
            Self::ObsidianVault => "obsidian_vault",
        }
    }
}

/// 统一搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    /// 结果来源类型
    pub source_type: KnowledgeSourceType,
    /// 源 ID（kb_id / wiki_id / namespace_id / vault_id）
    pub source_id: String,
    /// 内容项 ID（doc_id / note_id / memory_id）
    pub content_id: String,
    /// 标题
    pub title: String,
    /// 摘要/snippet
    pub snippet: String,
    /// 相关性分数（0.0 ~ 1.0）
    pub score: f64,
    /// 内容类型标识
    pub content_type: String,
}

/// 统一内容项
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentItem {
    /// 来源类型
    pub source_type: KnowledgeSourceType,
    /// 源 ID
    pub source_id: String,
    /// 内容 ID
    pub content_id: String,
    /// 标题
    pub title: String,
    /// 正文内容（可能为 markdown / plain text）
    pub body: String,
    /// 前端元数据（frontmatter / 标签 / 创建时间等）
    pub metadata: HashMap<String, String>,
}

/// 统一知识源元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSourceMeta {
    /// 源类型
    pub source_type: KnowledgeSourceType,
    /// 源 ID
    pub source_id: String,
    /// 显示名称
    pub name: String,
    /// 文档/笔记/记忆总数
    pub item_count: u64,
    /// 最后更新时间（Unix 毫秒时间戳）
    pub last_updated_at: Option<i64>,
}

/// 统一知识源抽象 trait
///
/// 所有知识源（RAG/Wiki/Memory/Obsidian）实现此 trait，
/// 上层调用方通过该接口进行统一的搜索、获取、实体查询等操作，
/// 无需感知底层源类型差异。
///
/// # 使用示例
///
/// ```ignore
/// let sources: Vec<Arc<dyn UnifiedKnowledgeSource>> = registry.get_all();
/// for source in &sources {
///     let results = source.search("src_id", "查询", 10).await?;
///     // 处理结果...
/// }
/// ```
#[async_trait]
pub trait UnifiedKnowledgeSource: Send + Sync {
    /// 返回知识源类型
    fn source_type(&self) -> KnowledgeSourceType;

    /// 搜索内容（向量/关键词/混合）
    ///
    /// # 参数
    /// - `source_id`: 源 ID（kb_id / wiki_id / namespace_id / vault_id）
    /// - `query`: 查询文本
    /// - `top_k`: 返回最多结果数
    async fn search(&self, source_id: &str, query: &str, top_k: usize)
    -> Result<Vec<SearchResult>>;

    /// 获取内容项详情
    ///
    /// # 参数
    /// - `source_id`: 源 ID
    /// - `content_id`: 内容 ID（doc_id / note_id / memory_id）
    async fn get_content(&self, source_id: &str, content_id: &str) -> Result<ContentItem>;

    /// 搜索实体（知识图谱）
    ///
    /// # 参数
    /// - `source_id`: 源 ID
    /// - `query`: 查询文本
    /// - `top_k`: 返回最多结果数
    async fn search_entities(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>>;

    /// 获取源的元数据
    ///
    /// # 参数
    /// - `source_id`: 源 ID
    async fn get_source_meta(&self, source_id: &str) -> Result<KnowledgeSourceMeta>;

    /// 按 ID 获取知识源对象（KnowledgeBase / Wiki / MemoryNamespace 等）
    ///
    /// 返回通用的元数据视图，具体类型通过 source_type 区分。
    async fn get_source(&self, source_id: &str) -> Result<KnowledgeSourceMeta> {
        self.get_source_meta(source_id).await
    }
}
