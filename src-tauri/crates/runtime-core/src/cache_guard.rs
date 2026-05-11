use std::sync::Arc;
use tokio::sync::RwLock;

use crate::prompt_cache::PromptCache;

pub struct CacheGuard {
    cache: Arc<PromptCache>,
    force_immediate: Arc<RwLock<bool>>,
}

impl CacheGuard {
    pub fn new(cache: Arc<PromptCache>) -> Self {
        Self {
            cache,
            force_immediate: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn set_force_immediate(&self, force: bool) {
        let mut fi = self.force_immediate.write().await;
        *fi = force;
    }

    pub async fn is_force_immediate(&self) -> bool {
        *self.force_immediate.read().await
    }

    pub async fn can_modify_system_prompt(&self) -> bool {
        if self.is_force_immediate().await {
            return true;
        }
        !self.cache.is_cache_valid().await
    }

    pub async fn can_modify_tools(&self) -> bool {
        if self.is_force_immediate().await {
            return true;
        }
        !self.cache.is_cache_valid().await
    }

    pub async fn can_reload_memory(&self) -> bool {
        if self.is_force_immediate().await {
            return true;
        }
        !self.cache.is_cache_valid().await
    }

    pub async fn guard_system_prompt_modification(&self) -> anyhow::Result<()> {
        if !self.can_modify_system_prompt().await {
            let pending = self.cache.has_pending_changes().await;
            if pending {
                anyhow::bail!(
                    "System prompt has pending changes. Changes will apply on next session. \
                     Use --now to apply immediately (this will invalidate the prompt cache)."
                );
            }
            anyhow::bail!(
                "Cannot modify system prompt while cache is valid. \
                 Changes must be deferred to next session or use --now to force."
            );
        }
        Ok(())
    }

    pub async fn guard_tool_modification(&self) -> anyhow::Result<()> {
        if !self.can_modify_tools().await {
            anyhow::bail!(
                "Cannot modify tools while cache is valid. \
                 Changes must be deferred to next session or use --now to force."
            );
        }
        Ok(())
    }

    pub async fn guard_memory_reload(&self) -> anyhow::Result<()> {
        if !self.can_reload_memory().await {
            anyhow::bail!(
                "Cannot reload memory mid-conversation. \
                 Use --now to force immediate reload (this will invalidate the prompt cache)."
            );
        }
        Ok(())
    }

    pub async fn guard_any_cache_sensitive_change(&self, component: &str) -> anyhow::Result<()> {
        if self.is_force_immediate().await {
            return Ok(());
        }
        if self.cache.is_cache_valid().await {
            anyhow::bail!(
                "Cannot modify {} while prompt cache is active. \
                 Use --now to force immediate change.",
                component
            );
        }
        Ok(())
    }
}
