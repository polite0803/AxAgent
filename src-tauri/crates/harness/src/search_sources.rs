// SPDX-License-Identifier: AGPL-3.0-only

//! 搜索层访问 dao + document-parser 的 trait 抽象。
//!
//! search crate 不再直接依赖 axagent-dao / axagent-document-parser，
//! 改为依赖本文件定义的 5 个 trait，并由 wiring 层在启动时注入实现。

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use crate::core_error::Result;
use crate::types::{AppSettings, KnowledgeBase, KnowledgeEntity, MemoryNamespace, NoteLink, Wiki};

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
