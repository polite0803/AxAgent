// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::search_sources::*` trait 的默认实现。
//!
//! 每个 struct 持有一个 `DatabaseConnection` 克隆，转发到现有
//! repo free function。wiring 层在启动时构造并注入。

use std::collections::HashMap;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::core_error::Result;
use axagent_harness::search_sources::{
    ContentItem, KnowledgeSource, KnowledgeSourceMeta, KnowledgeSourceType, MemorySource,
    SearchResult, SettingsSource, UnifiedKnowledgeSource, WikiSource,
};
use axagent_harness::types::{
    AppSettings, KnowledgeBase, KnowledgeEntity, MemoryNamespace, NoteLink, Wiki,
};

use crate::repo;

// ── KnowledgeSource ──

pub struct DefaultKnowledgeSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl KnowledgeSource for DefaultKnowledgeSource {
    async fn get_knowledge_base(&self, id: &str) -> Result<KnowledgeBase> {
        repo::knowledge::get_knowledge_base(&self.db, id).await
    }

    async fn get_document_titles(&self, doc_ids: &[String]) -> Result<HashMap<String, String>> {
        repo::knowledge::get_document_titles(&self.db, doc_ids).await
    }

    async fn search_entities(
        &self,
        kb_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        repo::knowledge_graph::search_entities(&self.db, kb_id, query, top_k).await
    }
}

// ── MemorySource ──

pub struct DefaultMemorySource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl MemorySource for DefaultMemorySource {
    async fn get_namespace(&self, id: &str) -> Result<MemoryNamespace> {
        repo::memory::get_namespace(&self.db, id).await
    }
}

// ── WikiSource ──

pub struct DefaultWikiSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl WikiSource for DefaultWikiSource {
    async fn get_wiki(&self, id: &str) -> Result<Wiki> {
        repo::wiki::get_wiki(&self.db, id).await
    }

    async fn get_note_backlinks_by_vault(&self, vault_id: &str) -> Result<Vec<NoteLink>> {
        repo::note::get_note_backlinks_by_vault(&self.db, vault_id).await
    }

    async fn get_note_titles(&self, note_ids: &[String]) -> Result<HashMap<String, String>> {
        let notes = repo::note::get_notes_by_ids(&self.db, note_ids).await?;
        Ok(notes.into_iter().map(|n| (n.id, n.title)).collect())
    }
}

// ── SettingsSource ──

pub struct DefaultSettingsSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl SettingsSource for DefaultSettingsSource {
    async fn get_settings(&self) -> Result<AppSettings> {
        repo::settings::get_settings(&self.db).await
    }
}

// ── UnifiedKnowledgeSource 实现 ──

/// RAG 知识库统一知识源实现
pub struct RagUnifiedSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl UnifiedKnowledgeSource for RagUnifiedSource {
    fn source_type(&self) -> KnowledgeSourceType {
        KnowledgeSourceType::KnowledgeBase
    }

    async fn search(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let entities =
            repo::knowledge_graph::search_entities(&self.db, source_id, query, top_k).await?;
        Ok(entities
            .into_iter()
            .map(|e| SearchResult {
                source_type: KnowledgeSourceType::KnowledgeBase,
                source_id: source_id.to_string(),
                content_id: e.id,
                title: e.name,
                snippet: e.description.unwrap_or_default(),
                score: e.confidence,
                content_type: e.entity_type,
            })
            .collect())
    }

    async fn get_content(&self, source_id: &str, content_id: &str) -> Result<ContentItem> {
        let entity = repo::knowledge_graph::get_entity_by_id(&self.db, content_id)
            .await?
            .ok_or_else(|| {
                axagent_harness::AxAgentError::NotFound(format!(
                    "Knowledge entity not found: {}",
                    content_id
                ))
            })?;
        Ok(ContentItem {
            source_type: KnowledgeSourceType::KnowledgeBase,
            source_id: source_id.to_string(),
            content_id: entity.id,
            title: entity.name,
            body: entity.description.unwrap_or_default(),
            metadata: serde_json::from_value(entity.properties.clone()).unwrap_or_default(),
        })
    }

    async fn search_entities(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        repo::knowledge_graph::search_entities(&self.db, source_id, query, top_k).await
    }

    async fn get_source_meta(&self, source_id: &str) -> Result<KnowledgeSourceMeta> {
        let kb = repo::knowledge::get_knowledge_base(&self.db, source_id).await?;
        Ok(KnowledgeSourceMeta {
            source_type: KnowledgeSourceType::KnowledgeBase,
            source_id: kb.id,
            name: kb.name,
            item_count: 0,
            last_updated_at: None,
        })
    }
}

/// Wiki 统一知识源实现
pub struct WikiUnifiedSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl UnifiedKnowledgeSource for WikiUnifiedSource {
    fn source_type(&self) -> KnowledgeSourceType {
        KnowledgeSourceType::Wiki
    }

    async fn search(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        // 数据库层面过滤 + 限制结果数量，避免全表加载
        let notes = repo::note::search_notes(&self.db, source_id, query, top_k).await?;

        let query_lower = query.to_lowercase();
        let mut scored: Vec<(f64, SearchResult)> = notes
            .into_iter()
            .map(|note| {
                let title_lower = note.title.to_lowercase();
                let content_lower = note.content.to_lowercase();

                let mut score = 0.0;
                if title_lower.contains(&query_lower) {
                    score += 1.0;
                }
                if content_lower.contains(&query_lower) {
                    score += 0.5;
                }
                if score == 0.0 {
                    score = 0.01;
                }

                let snippet = if content_lower.contains(&query_lower) {
                    let idx = content_lower.find(&query_lower).unwrap_or(0);
                    let start = idx.saturating_sub(100);
                    let end = (idx + query.len() + 200).min(note.content.len());
                    note.content[start..end].to_string()
                } else {
                    note.content.chars().take(300).collect()
                };

                (
                    score,
                    SearchResult {
                        source_type: KnowledgeSourceType::Wiki,
                        source_id: source_id.to_string(),
                        content_id: note.id,
                        title: note.title,
                        snippet,
                        score,
                        content_type: "note".to_string(),
                    },
                )
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, r)| r).collect())
    }

    async fn get_content(&self, source_id: &str, content_id: &str) -> Result<ContentItem> {
        let note = repo::note::get_note(&self.db, content_id).await?;
        Ok(ContentItem {
            source_type: KnowledgeSourceType::Wiki,
            source_id: source_id.to_string(),
            content_id: note.id,
            title: note.title,
            body: note.content,
            metadata: {
                let mut m = HashMap::new();
                m.insert("file_path".to_string(), note.file_path);
                if let Some(author) = Some(note.author).filter(|a| !a.is_empty()) {
                    m.insert("author".to_string(), author);
                }
                if let Some(page_type) = note.page_type {
                    m.insert("page_type".to_string(), page_type);
                }
                m
            },
        })
    }

    async fn search_entities(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        repo::knowledge_graph::search_entities(&self.db, source_id, query, top_k).await
    }

    async fn get_source_meta(&self, source_id: &str) -> Result<KnowledgeSourceMeta> {
        let wiki = repo::wiki::get_wiki(&self.db, source_id).await?;
        Ok(KnowledgeSourceMeta {
            source_type: KnowledgeSourceType::Wiki,
            source_id: wiki.id.clone(),
            name: wiki.name,
            item_count: wiki.note_count as u64,
            last_updated_at: Some(wiki.updated_at),
        })
    }
}

/// 记忆统一知识源实现
pub struct MemoryUnifiedSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl UnifiedKnowledgeSource for MemoryUnifiedSource {
    fn source_type(&self) -> KnowledgeSourceType {
        KnowledgeSourceType::Memory
    }

    async fn search(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        // 数据库层面过滤 + LIMIT，避免全表扫描
        let items = repo::memory::search_items(&self.db, source_id, query, top_k).await?;

        let query_lower = query.to_lowercase();

        let mut scored: Vec<(f64, SearchResult)> = items
            .into_iter()
            .map(|item| {
                let title_lower = item.title.to_lowercase();
                let content_lower = item.content.to_lowercase();

                let mut score = 0.0;
                if title_lower.contains(&query_lower) {
                    score += 1.0;
                }
                if content_lower.contains(&query_lower) {
                    score += 0.5;
                }
                // 记忆重要性加权
                score += item.importance * 0.3;
                if score == 0.0 {
                    score = 0.01;
                }

                let snippet = if content_lower.contains(&query_lower) {
                    let idx = content_lower.find(&query_lower).unwrap_or(0);
                    let start = idx.saturating_sub(100);
                    let end = (idx + query.len() + 200).min(item.content.len());
                    item.content[start..end].to_string()
                } else {
                    item.content.chars().take(300).collect()
                };

                (
                    score,
                    SearchResult {
                        source_type: KnowledgeSourceType::Memory,
                        source_id: source_id.to_string(),
                        content_id: item.id,
                        title: item.title,
                        snippet,
                        score,
                        content_type: format!("memory_{}", item.tier),
                    },
                )
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, r)| r).collect())
    }

    async fn get_content(&self, source_id: &str, content_id: &str) -> Result<ContentItem> {
        let item = repo::memory::get_item(&self.db, content_id).await?;
        Ok(ContentItem {
            source_type: KnowledgeSourceType::Memory,
            source_id: source_id.to_string(),
            content_id: item.id,
            title: item.title,
            body: item.content,
            metadata: {
                let mut m = HashMap::new();
                m.insert("tier".to_string(), item.tier);
                m.insert("importance".to_string(), format!("{:.2}", item.importance));
                m.insert("memory_nature".to_string(), item.memory_nature);
                m.insert("source".to_string(), item.source);
                if !item.tags.is_empty() {
                    m.insert("tags".to_string(), item.tags.join(","));
                }
                if let Some(conv_id) = item.source_conversation_id {
                    m.insert("source_conversation_id".to_string(), conv_id);
                }
                m
            },
        })
    }

    async fn search_entities(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        // 记忆项本身不是图谱实体，但可以返回空列表
        let _ = (source_id, query, top_k);
        Ok(vec![])
    }

    async fn get_source_meta(&self, source_id: &str) -> Result<KnowledgeSourceMeta> {
        let ns = repo::memory::get_namespace(&self.db, source_id).await?;
        let items = repo::memory::list_items(&self.db, source_id).await?;
        Ok(KnowledgeSourceMeta {
            source_type: KnowledgeSourceType::Memory,
            source_id: ns.id.clone(),
            name: ns.name,
            item_count: items.len() as u64,
            last_updated_at: items.first().and_then(|i| i.last_accessed),
        })
    }
}

/// Obsidian Vault 统一知识源实现
pub struct ObsidianUnifiedSource {
    pub db: DatabaseConnection,
}

#[async_trait]
impl UnifiedKnowledgeSource for ObsidianUnifiedSource {
    fn source_type(&self) -> KnowledgeSourceType {
        KnowledgeSourceType::ObsidianVault
    }

    async fn search(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        // 从 DB 中查找该 vault 下的 wiki notes，Obsidian Vault 的笔记已同步到 wiki 系统
        let notes = repo::note::search_notes(&self.db, source_id, query, top_k).await?;

        let query_lower = query.to_lowercase();
        let mut scored: Vec<(f64, SearchResult)> = notes
            .into_iter()
            .map(|note| {
                let title_lower = note.title.to_lowercase();
                let content_lower = note.content.to_lowercase();

                let mut score = 0.0;
                if title_lower.contains(&query_lower) {
                    score += 1.0;
                }
                if content_lower.contains(&query_lower) {
                    score += 0.5;
                }
                if score == 0.0 {
                    score = 0.01;
                }

                let snippet = if content_lower.contains(&query_lower) {
                    let idx = content_lower.find(&query_lower).unwrap_or(0);
                    let start = idx.saturating_sub(100);
                    let end = (idx + query.len() + 200).min(note.content.len());
                    note.content[start..end].to_string()
                } else {
                    note.content.chars().take(300).collect()
                };

                (
                    score,
                    SearchResult {
                        source_type: KnowledgeSourceType::ObsidianVault,
                        source_id: source_id.to_string(),
                        content_id: note.id,
                        title: note.title,
                        snippet,
                        score,
                        content_type: "obsidian_note".to_string(),
                    },
                )
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).map(|(_, r)| r).collect())
    }

    async fn get_content(&self, source_id: &str, content_id: &str) -> Result<ContentItem> {
        let note = repo::note::get_note(&self.db, content_id).await?;
        Ok(ContentItem {
            source_type: KnowledgeSourceType::ObsidianVault,
            source_id: source_id.to_string(),
            content_id: note.id,
            title: note.title,
            body: note.content,
            metadata: {
                let mut m = HashMap::new();
                m.insert("file_path".to_string(), note.file_path);
                if let Some(author) = Some(note.author).filter(|a| !a.is_empty()) {
                    m.insert("author".to_string(), author);
                }
                if let Some(page_type) = note.page_type {
                    m.insert("page_type".to_string(), page_type);
                }
                m
            },
        })
    }

    async fn search_entities(
        &self,
        source_id: &str,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<KnowledgeEntity>> {
        // Obsidian 笔记已同步为 Wiki 实体，复用知识图谱搜索
        repo::knowledge_graph::search_entities(&self.db, source_id, query, top_k).await
    }

    async fn get_source_meta(&self, source_id: &str) -> Result<KnowledgeSourceMeta> {
        // Obsidian Vault 通过 wiki 表注册
        let wiki = repo::wiki::get_wiki(&self.db, source_id).await?;
        Ok(KnowledgeSourceMeta {
            source_type: KnowledgeSourceType::ObsidianVault,
            source_id: wiki.id.clone(),
            name: format!("Obsidian Vault: {}", wiki.name),
            item_count: wiki.note_count as u64,
            last_updated_at: Some(wiki.updated_at),
        })
    }
}
