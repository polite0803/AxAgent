// SPDX-License-Identifier: AGPL-3.0-only

//! MemoryStore 契约（记忆外溢/共享）

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAddRequest {
    pub content: String,
    pub namespace: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchItem {
    pub id: String,
    pub content: String,
    pub namespace: Option<String>,
    pub importance: u8,
    pub tags: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub query: String,
    pub namespace: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTreeItem {
    pub id: String,
    pub content: String,
    pub children: Vec<MemoryTreeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUpdateRequest {
    pub id: String,
    pub content: Option<String>,
    pub importance: Option<u8>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFeedbackRequest {
    pub memory_id: String,
    pub helpful: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryActionResultDto {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGroupedDto {
    pub date: String,
    pub items: Vec<MemorySearchItem>,
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn add_memory(&self, req: MemoryAddRequest) -> Result<MemoryActionResultDto, String>;
    async fn search_memories(
        &self,
        req: MemorySearchRequest,
    ) -> Result<Vec<MemorySearchItem>, String>;
    async fn get_memory_tree(&self) -> Result<Vec<MemoryTreeItem>, String>;
    async fn get_working_memory(&self) -> Result<Option<String>, String>;
    async fn get_grouped_memories(&self) -> Result<Vec<MemoryGroupedDto>, String>;
    async fn submit_feedback(
        &self,
        req: MemoryFeedbackRequest,
    ) -> Result<MemoryActionResultDto, String>;
    async fn delete_memory(&self, id: &str) -> Result<MemoryActionResultDto, String>;
    async fn update_memory(
        &self,
        req: MemoryUpdateRequest,
    ) -> Result<MemoryActionResultDto, String>;
}

pub struct NoopMemoryStore;

#[async_trait]
impl MemoryStore for NoopMemoryStore {
    async fn add_memory(&self, _req: MemoryAddRequest) -> Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn search_memories(
        &self,
        _req: MemorySearchRequest,
    ) -> Result<Vec<MemorySearchItem>, String> {
        Ok(vec![])
    }
    async fn get_memory_tree(&self) -> Result<Vec<MemoryTreeItem>, String> {
        Ok(vec![])
    }
    async fn get_working_memory(&self) -> Result<Option<String>, String> {
        Ok(None)
    }
    async fn get_grouped_memories(&self) -> Result<Vec<MemoryGroupedDto>, String> {
        Ok(vec![])
    }
    async fn submit_feedback(
        &self,
        _req: MemoryFeedbackRequest,
    ) -> Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn delete_memory(&self, _id: &str) -> Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
    async fn update_memory(
        &self,
        _req: MemoryUpdateRequest,
    ) -> Result<MemoryActionResultDto, String> {
        Ok(MemoryActionResultDto { success: true, error: None })
    }
}
