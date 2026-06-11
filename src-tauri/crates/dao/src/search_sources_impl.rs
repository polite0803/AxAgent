// SPDX-License-Identifier: AGPL-3.0-only

//! `axagent_harness::search_sources::*` trait 的默认实现。
//!
//! 每个 struct 持有一个 `DatabaseConnection` 克隆，转发到现有
//! repo free function。wiring 层在启动时构造并注入。

use std::collections::HashMap;

use async_trait::async_trait;
use sea_orm::DatabaseConnection;

use axagent_harness::core_error::Result;
use axagent_harness::search_sources::{KnowledgeSource, MemorySource, SettingsSource, WikiSource};
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
