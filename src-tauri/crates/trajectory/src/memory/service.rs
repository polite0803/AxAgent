//! Memory service - core working memory and session management
//!
//! Replaces TypeScript `MemoryService.ts` with Rust implementation.

use crate::TrajectoryStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_memory_entries: usize,
    pub max_user_entries: usize,
    pub token_limit: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 50,
            max_user_entries: 100,
            token_limit: 4000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub memory_type: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct WorkingMemory {
    pub memory: HashMap<String, MemoryEntry>,
    pub user: HashMap<String, MemoryEntry>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub memory_count: usize,
    pub user_count: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub session_id: String,
    pub session_title: String,
    pub message_content: String,
    pub match_type: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryActionResult {
    pub success: bool,
    pub message: String,
    pub new_usage: Option<MemoryUsage>,
}

pub struct MemoryService {
    storage: Arc<TrajectoryStorage>,
    working_memory: RwLock<WorkingMemory>,
    #[allow(dead_code)]
    config: MemoryConfig,
}

impl MemoryService {
    pub fn new(storage: Arc<TrajectoryStorage>) -> anyhow::Result<Self> {
        Ok(Self {
            storage,
            working_memory: RwLock::new(WorkingMemory::default()),
            config: MemoryConfig::default(),
        })
    }

    pub fn initialize(&self) -> anyhow::Result<()> {
        self.storage.init_memory_tables()?;
        self.load_memories_from_storage()
    }

    fn load_memories_from_storage(&self) -> anyhow::Result<()> {
        let memories = self.storage.get_all_memories()?;

        let mut working = self.working_memory.write().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });

        for memory in memories {
            let entry = MemoryEntry {
                id: memory.id.clone(),
                content: memory.content.clone(),
                memory_type: memory.memory_type.clone(),
                updated_at: memory.updated_at,
            };

            match memory.memory_type.as_str() {
                "memory" => { working.memory.insert(entry.id.clone(), entry); }
                "user" => { working.user.insert(entry.id.clone(), entry); }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn add_memory(
        &self,
        target: &str,
        content: &str,
    ) -> MemoryActionResult {
        if content.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "内容不能为空".to_string(),
                new_usage: None,
            };
        }

        let memory_type = target.to_string();
        let entry = MemoryEntry {
            id: format!("mem_{}_{}", chrono::Utc::now().timestamp_millis(), uuid::Uuid::new_v4()),
            content: content.to_string(),
            memory_type: memory_type.clone(),
            updated_at: chrono::Utc::now().timestamp(),
        };

        if let Err(e) = self.storage.save_memory(&entry) {
            return MemoryActionResult {
                success: false,
                message: format!("保存失败: {}", e),
                new_usage: None,
            };
        }

        // Sync FTS5 index
        if let Err(e) = self.storage.index_memory_fts(&entry.id, &entry.memory_type, &entry.content, &[]) {
            tracing::warn!("Failed to sync FTS5 index for new memory: {}", e);
        }

        {
            let mut mem = self.working_memory.write().unwrap_or_else(|e| {
                tracing::warn!("Working memory lock poisoned, recovering: {}", e);
                e.into_inner()
            });
            match target {
                "memory" => { mem.memory.insert(entry.id.clone(), entry); }
                "user" => { mem.user.insert(entry.id.clone(), entry); }
                _ => return MemoryActionResult {
                    success: false,
                    message: "无效的记忆类型".to_string(),
                    new_usage: None,
                },
            }
        }

        MemoryActionResult {
            success: true,
            message: format!("已添加记忆: \"{}\"", &content[..content.len().min(30)]),
            new_usage: Some(self.get_memory_usage()),
        }
    }

    pub fn replace_memory(
        &self,
        target: &str,
        old_text: &str,
        new_text: &str,
    ) -> MemoryActionResult {
        if old_text.trim().is_empty() || new_text.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "旧文本和新文本都不能为空".to_string(),
                new_usage: None,
            };
        }

        let mut mem = self.working_memory.write().unwrap();
        let map = match target {
            "memory" => &mut mem.memory,
            "user" => &mut mem.user,
            _ => return MemoryActionResult {
                success: false,
                message: "无效的记忆类型".to_string(),
                new_usage: None,
            },
        };

        let mut found = None;
        for (id, entry) in map.iter() {
            if entry.content.contains(old_text) {
                found = Some(id.clone());
                break;
            }
        }

        if let Some(id) = found {
            let entry = MemoryEntry {
                id: id.clone(),
                content: new_text.to_string(),
                memory_type: target.to_string(),
                updated_at: chrono::Utc::now().timestamp(),
            };

            if let Err(e) = self.storage.save_memory(&entry) {
                return MemoryActionResult {
                    success: false,
                    message: format!("替换失败: {}", e),
                    new_usage: None,
                };
            }

            // Sync FTS5 index
            if let Err(e) = self.storage.index_memory_fts(&entry.id, &entry.memory_type, &entry.content, &[]) {
                tracing::warn!("Failed to sync FTS5 index for replaced memory: {}", e);
            }

            map.insert(id, entry);

            MemoryActionResult {
                success: true,
                message: "已替换记忆".to_string(),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到要替换的记忆".to_string(),
                new_usage: None,
            }
        }
    }

    pub fn remove_memory(&self, target: &str, text: &str) -> MemoryActionResult {
        if text.trim().is_empty() {
            return MemoryActionResult {
                success: false,
                message: "要删除的文本不能为空".to_string(),
                new_usage: None,
            };
        }

        let mut mem = self.working_memory.write().unwrap();
        let map = match target {
            "memory" => &mut mem.memory,
            "user" => &mut mem.user,
            _ => return MemoryActionResult {
                success: false,
                message: "无效的记忆类型".to_string(),
                new_usage: None,
            },
        };

        let mut found = None;
        for (id, entry) in map.iter() {
            if entry.content.contains(text) {
                found = Some(id.clone());
                break;
            }
        }

        if let Some(id) = found {
            if let Err(e) = self.storage.delete_memory(&id) {
                return MemoryActionResult {
                    success: false,
                    message: format!("删除失败: {}", e),
                    new_usage: None,
                };
            }

            // Sync FTS5 index - remove from FTS
            if let Err(e) = self.storage.delete_memory_fts(&id) {
                tracing::warn!("Failed to remove memory from FTS5 index: {}", e);
            }

            map.remove(&id);

            MemoryActionResult {
                success: true,
                message: "已删除记忆".to_string(),
                new_usage: Some(self.get_memory_usage()),
            }
        } else {
            MemoryActionResult {
                success: false,
                message: "未找到要删除的记忆".to_string(),
                new_usage: None,
            }
        }
    }

    pub fn get_memory_usage(&self) -> MemoryUsage {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });
        let memory_count = mem.memory.len();
        let user_count = mem.user.len();

        let working_tokens: usize = mem.memory
            .values()
            .map(|e| e.content.len() / 4)
            .sum();

        let user_tokens: usize = mem.user
            .values()
            .map(|e| e.content.len() / 4)
            .sum();

        MemoryUsage {
            memory_count,
            user_count,
            total_tokens: working_tokens + user_tokens,
        }
    }

    pub fn get_working_memory(&self) -> WorkingMemory {
        self.working_memory.read().unwrap().clone()
    }

    pub fn format_for_prompt(&self) -> String {
        let mem = self.working_memory.read().unwrap_or_else(|e| {
            tracing::warn!("Working memory lock poisoned, recovering: {}", e);
            e.into_inner()
        });
        let mut sections = Vec::new();

        if !mem.memory.is_empty() {
            sections.push("## Working Memory\n".to_string());
            for entry in mem.memory.values() {
                sections.push(format!("- {}", entry.content));
            }
        }

        if !mem.user.is_empty() {
            sections.push("\n## User Preferences\n".to_string());
            for entry in mem.user.values() {
                sections.push(format!("- {}", entry.content));
            }
        }

        sections.join("\n")
    }
}