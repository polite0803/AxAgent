// SPDX-License-Identifier: AGPL-3.0-only

//! 同步策略存储
//!
//! 管理同步策略的增删改查，支持多策略配置。

use std::collections::HashMap;

use axagent_harness::device_sync::{
    ConflictResolutionStrategy, EntityType, SyncPolicy, SyncPolicyUpdate,
};
use axagent_harness::util_fns::current_rfc3339;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 同步策略存储
#[derive(Default)]
pub struct PolicyStore {
    policies: RwLock<HashMap<String, SyncPolicy>>,
    active_policy_id: RwLock<Option<String>>,
}

impl PolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建策略
    pub async fn create_policy(
        &self,
        mut policy: SyncPolicy,
    ) -> SyncPolicy {
        if policy.id.is_empty() {
            policy.id = Uuid::new_v4().to_string();
        }
        policy.updated_at = current_rfc3339();
        self.policies
            .write()
            .await
            .insert(policy.id.clone(), policy.clone());
        policy
    }

    /// 获取单个策略
    pub async fn get_policy(&self, id: &str) -> Option<SyncPolicy> {
        self.policies.read().await.get(id).cloned()
    }

    /// 获取当前活动策略
    pub async fn get_active_policy(&self) -> Option<SyncPolicy> {
        let active_id = self.active_policy_id.read().await;
        if let Some(id) = active_id.as_ref() {
            self.policies.read().await.get(id).cloned()
        } else {
            None
        }
    }

    /// 设置活动策略
    pub async fn set_active_policy(&self, id: &str) -> Result<(), String> {
        let exists = self.policies.read().await.contains_key(id);
        if !exists {
            return Err("策略不存在".to_string());
        }
        *self.active_policy_id.write().await = Some(id.to_string());
        Ok(())
    }

    /// 列出所有策略
    pub async fn list_policies(&self) -> Vec<SyncPolicy> {
        let policies = self.policies.read().await;
        let mut list: Vec<_> = policies.values().cloned().collect();
        list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        list
    }

    /// 更新策略
    pub async fn update_policy(
        &self,
        id: &str,
        update: SyncPolicyUpdate,
    ) -> Result<SyncPolicy, String> {
        let mut policies = self.policies.write().await;
        let policy = policies
            .get_mut(id)
            .ok_or_else(|| "策略不存在".to_string())?;

        if let Some(name) = update.name {
            policy.name = name;
        }
        if let Some(strategy) = update.conflict_strategy {
            policy.conflict_strategy = strategy;
        }
        if let Some(interval) = update.auto_sync_interval_secs {
            policy.auto_sync_interval_secs = interval;
        }
        if let Some(scope) = update.sync_scope {
            policy.sync_scope = scope;
        }
        if let Some(auto_resolve) = update.auto_resolve_conflicts {
            policy.auto_resolve_conflicts = auto_resolve;
        }
        if let Some(threshold) = update.max_conflict_threshold {
            policy.max_conflict_threshold = threshold;
        }
        if let Some(retention_enabled) = update.change_log_retention_enabled {
            policy.change_log_retention_enabled = retention_enabled;
        }
        if let Some(retention_days) = update.change_log_retention_days {
            policy.change_log_retention_days = retention_days;
        }
        if let Some(enabled) = update.enabled {
            policy.enabled = enabled;
        }

        policy.updated_at = current_rfc3339();
        Ok(policy.clone())
    }

    /// 删除策略
    pub async fn delete_policy(&self, id: &str) -> Result<(), String> {
        let mut policies = self.policies.write().await;
        if !policies.contains_key(id) {
            return Err("策略不存在".to_string());
        }
        policies.remove(id);

        // 如果删除的是活动策略，清除活动状态
        let mut active_id = self.active_policy_id.write().await;
        if active_id.as_deref() == Some(id) {
            *active_id = None;
        }
        Ok(())
    }

    /// 获取默认策略
    pub fn default_policy() -> SyncPolicy {
        SyncPolicy {
            id: String::new(),
            name: "默认策略".to_string(),
            conflict_strategy: ConflictResolutionStrategy::LastWriteWins,
            auto_sync_interval_secs: 0,
            sync_scope: vec![
                EntityType::Conversation,
                EntityType::Message,
                EntityType::Setting,
            ],
            auto_resolve_conflicts: true,
            max_conflict_threshold: 100,
            change_log_retention_enabled: true,
            change_log_retention_days: 30,
            enabled: true,
            updated_at: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_and_get_policy() {
        let store = PolicyStore::new();
        let policy = PolicyStore::default_policy();
        let created = store.create_policy(policy).await;

        assert!(!created.id.is_empty());
        let fetched = store.get_policy(&created.id).await.unwrap();
        assert_eq!(fetched.name, "默认策略");
    }

    #[tokio::test]
    async fn test_update_policy() {
        let store = PolicyStore::new();
        let policy = PolicyStore::default_policy();
        let created = store.create_policy(policy).await;

        let updated = store
            .update_policy(
                &created.id,
                SyncPolicyUpdate {
                    name: Some("新名称".to_string()),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.name, "新名称");
    }

    #[tokio::test]
    async fn test_delete_policy() {
        let store = PolicyStore::new();
        let policy = PolicyStore::default_policy();
        let created = store.create_policy(policy).await;

        store.delete_policy(&created.id).await.unwrap();
        assert!(store.get_policy(&created.id).await.is_none());
    }
}