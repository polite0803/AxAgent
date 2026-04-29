use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptCacheState {
    pub system_prompt_hash: Option<String>,
    pub tools_hash: Option<String>,
    pub memory_hash: Option<String>,
    pub context_files_hash: Option<String>,
    pub cache_valid: bool,
    pub last_invalidation_reason: Option<String>,
    pub pending_changes: Vec<PendingChange>,
    pub tokens_saved_estimate: u64,
    pub cache_hits: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingChange {
    pub component: String,
    pub description: String,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PromptCache {
    state: Arc<RwLock<PromptCacheState>>,
}

impl PromptCache {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(PromptCacheState::default())),
        }
    }

    pub async fn record_system_prompt(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.system_prompt_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "system_prompt".to_string(),
                description: "System prompt modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
            state.cache_valid = false;
            state.last_invalidation_reason =
                Some("System prompt changed".to_string());
        }

        if state.system_prompt_hash.is_none() {
            state.cache_valid = true;
        }

        state.system_prompt_hash = Some(hash);
    }

    pub async fn record_tools(&self, tool_names: &[String]) {
        let hash = compute_hash(&tool_names.join(","));
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.tools_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "tools".to_string(),
                description: "Tool set modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
            state.cache_valid = false;
            state.last_invalidation_reason =
                Some("Tool set changed".to_string());
        }

        state.tools_hash = Some(hash);
    }

    pub async fn record_memory(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.memory_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "memory".to_string(),
                description: "Memory content modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
        }

        state.memory_hash = Some(hash);
    }

    pub async fn record_context_files(&self, content: &str) {
        let hash = compute_hash(content);
        let mut state = self.state.write().await;

        let old_hash_changed = match &state.context_files_hash {
            Some(old) if *old != hash => Some(old.clone()),
            _ => None,
        };

        if let Some(old_hash) = old_hash_changed {
            state.pending_changes.push(PendingChange {
                component: "context_files".to_string(),
                description: "Context files modified during session".to_string(),
                old_hash: Some(old_hash),
                new_hash: Some(hash.clone()),
            });
        }

        state.context_files_hash = Some(hash);
    }

    pub async fn is_cache_valid(&self) -> bool {
        self.state.read().await.cache_valid
    }

    pub async fn get_state(&self) -> PromptCacheState {
        self.state.read().await.clone()
    }

    pub async fn mark_cache_hit(&self, token_count: u64) {
        let mut state = self.state.write().await;
        state.cache_hits += 1;
        state.tokens_saved_estimate += token_count;
    }

    pub async fn invalidate(&self, reason: &str) {
        let mut state = self.state.write().await;
        state.cache_valid = false;
        state.last_invalidation_reason = Some(reason.to_string());
    }

    pub async fn invalidate_for_new_session(&self) {
        let mut state = self.state.write().await;
        state.system_prompt_hash = None;
        state.tools_hash = None;
        state.memory_hash = None;
        state.context_files_hash = None;
        state.cache_valid = false;
        state.last_invalidation_reason = None;
        state.pending_changes.clear();
    }

    pub async fn apply_pending_changes(&self) -> Vec<PendingChange> {
        let mut state = self.state.write().await;
        let changes = state.pending_changes.clone();
        state.pending_changes.clear();
        state.cache_valid = true;
        changes
    }

    pub async fn has_pending_changes(&self) -> bool {
        !self.state.read().await.pending_changes.is_empty()
    }

    pub async fn total_tokens_saved(&self) -> u64 {
        self.state.read().await.tokens_saved_estimate
    }

    pub async fn reset_stats(&self) {
        let mut state = self.state.write().await;
        state.cache_hits = 0;
        state.tokens_saved_estimate = 0;
    }
}

impl Default for PromptCache {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}
