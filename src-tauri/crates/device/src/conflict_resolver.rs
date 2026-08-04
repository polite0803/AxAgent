// SPDX-License-Identifier: AGPL-3.0-only

//! 冲突解决器实现
//!
//! 基于版本向量检测冲突，支持多种解决策略：
//! - LastWriteWins: 最后写入胜出（按时间戳）
//! - KeepLocal: 保留本地版本
//! - KeepRemote: 保留远程版本
//! - KeepBoth: 保留双方版本（标记为冲突待手动合并）

use std::collections::HashMap;

use axagent_harness::device_sync::{
    ChangeLogEntry, ConflictInfo, ConflictResolutionStrategy, EntityType,
};
use uuid::Uuid;

use crate::version_vector::{CausalOrder, VersionVector};

/// 冲突解决器
pub struct ConflictResolver;

impl ConflictResolver {
    /// 根据 ID 查找冲突记录
    pub fn find_conflict_by_id(
        changes: &[ChangeLogEntry],
        conflict_id: &str,
    ) -> Option<ConflictInfo> {
        // 首先检测所有冲突
        let conflicts = Self::detect_conflicts(changes);
        // 在冲突列表中查找匹配的 ID
        conflicts.into_iter().find(|c| c.id == conflict_id)
    }

    /// 检测变更日志中的冲突
    pub fn detect_conflicts(changes: &[ChangeLogEntry]) -> Vec<ConflictInfo> {
        let mut conflicts = Vec::new();

        // 按 (entity_type, entity_id) 分组
        let mut entity_changes: HashMap<(EntityType, String), Vec<&ChangeLogEntry>> =
            HashMap::new();
        for change in changes {
            let key = (change.entity_type, change.entity_id.clone());
            entity_changes.entry(key).or_default().push(change);
        }

        // 检查每个实体的变更是否存在冲突
        for ((entity_type, entity_id), entries) in &entity_changes {
            if entries.len() < 2 {
                continue;
            }

            // 比较版本向量
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let vv1 = VersionVector::from_entries(&entries[i].version_vector);
                    let vv2 = VersionVector::from_entries(&entries[j].version_vector);

                    if matches!(vv1.compare(&vv2), CausalOrder::Concurrent) {
                        let mut conflicting_devices = Vec::new();
                        conflicting_devices.push(entries[i].device_id.clone());
                        if !conflicting_devices.contains(&entries[j].device_id) {
                            conflicting_devices.push(entries[j].device_id.clone());
                        }

                        conflicts.push(ConflictInfo {
                            id: Uuid::new_v4().to_string(),
                            entity_type: *entity_type,
                            entity_id: entity_id.clone(),
                            conflicting_devices,
                            local_vector: entries[i].version_vector.clone(),
                            remote_vector: entries[j].version_vector.clone(),
                            local_data: entries[i].data.clone(),
                            remote_data: entries[j].data.clone(),
                            local_timestamp: entries[i].timestamp,
                            remote_timestamp: entries[j].timestamp,
                            detected_at: chrono::Utc::now().to_rfc3339(),
                            resolved: false,
                            resolution_applied: None,
                            resolved_at: None,
                        });
                    }
                }
            }
        }

        conflicts
    }

    /// 解决单个冲突
    pub fn resolve(
        conflict: &ConflictInfo,
        strategy: ConflictResolutionStrategy,
    ) -> ResolutionResult {
        let now = chrono::Utc::now().to_rfc3339();
        let conflict_id = Uuid::new_v4().to_string();

        match strategy {
            ConflictResolutionStrategy::KeepLocal => ResolutionResult {
                resolved_data: conflict.local_data.clone(),
                resolution_applied: "keep_local".to_string(),
                conflict_id,
                resolved_at: now,
            },
            ConflictResolutionStrategy::KeepRemote => ResolutionResult {
                resolved_data: conflict.remote_data.clone(),
                resolution_applied: "keep_remote".to_string(),
                conflict_id,
                resolved_at: now,
            },
            ConflictResolutionStrategy::LastWriteWins => {
                // 比较时间戳，选择较新的版本
                let (resolved_data, applied) =
                    if conflict.local_timestamp >= conflict.remote_timestamp {
                        (conflict.local_data.clone(), "last_write_wins_local")
                    } else {
                        (conflict.remote_data.clone(), "last_write_wins_remote")
                    };
                ResolutionResult {
                    resolved_data,
                    resolution_applied: applied.to_string(),
                    conflict_id,
                    resolved_at: now,
                }
            },
            ConflictResolutionStrategy::KeepBoth => {
                // 保留双方数据，组合成标记版本
                let merged = match (&conflict.local_data, &conflict.remote_data) {
                    (Some(local), Some(remote)) => {
                        Some(format!(r#"{{"local":{},"remote":{},"_merged":true}}"#, local, remote))
                    },
                    (Some(local), None) => Some(local.clone()),
                    (None, Some(remote)) => Some(remote.clone()),
                    (None, None) => None,
                };
                ResolutionResult {
                    resolved_data: merged,
                    resolution_applied: "keep_both".to_string(),
                    conflict_id,
                    resolved_at: now,
                }
            },
            ConflictResolutionStrategy::CustomMerge => ResolutionResult {
                resolved_data: None,
                resolution_applied: "custom_merge".to_string(),
                conflict_id,
                resolved_at: now,
            },
        }
    }

    /// 自动解决冲突（用于后台同步）
    pub fn auto_resolve(changes: &[ChangeLogEntry]) -> AutoResolveResult {
        Self::auto_resolve_with_strategy(changes, ConflictResolutionStrategy::LastWriteWins)
    }

    /// 使用指定策略自动解决冲突
    pub fn auto_resolve_with_strategy(
        changes: &[ChangeLogEntry],
        default_strategy: ConflictResolutionStrategy,
    ) -> AutoResolveResult {
        let conflicts = Self::detect_conflicts(changes);
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();

        for conflict in &conflicts {
            let result = Self::resolve(conflict, default_strategy);
            if result.resolved_data.is_some() {
                resolved.push(result);
            } else {
                unresolved.push(conflict.clone());
            }
        }

        AutoResolveResult {
            total_conflicts: conflicts.len(),
            auto_resolved: resolved.len(),
            needs_manual: unresolved,
        }
    }
}

/// 解决结果
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    pub resolved_data: Option<String>,
    pub resolution_applied: String,
    pub conflict_id: String,
    pub resolved_at: String,
}

/// 自动解决结果
#[derive(Debug)]
pub struct AutoResolveResult {
    pub total_conflicts: usize,
    pub auto_resolved: usize,
    pub needs_manual: Vec<ConflictInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axagent_harness::device_sync::{ChangeOperation, EntityType, VersionVectorEntry};

    fn make_change(
        entity_id: &str,
        device_id: &str,
        counter: u64,
        data: Option<&str>,
    ) -> ChangeLogEntry {
        ChangeLogEntry {
            id: Uuid::new_v4().to_string(),
            entity_type: EntityType::Conversation,
            entity_id: entity_id.to_string(),
            operation: ChangeOperation::Update,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            device_id: device_id.to_string(),
            version_vector: vec![VersionVectorEntry { device_id: device_id.to_string(), counter }],
            data: data.map(|s| s.to_string()),
            synced_to: vec![],
            is_synced: false,
        }
    }

    #[test]
    fn test_detect_no_conflict() {
        let changes = vec![
            make_change("conv-1", "device-a", 1, Some("data-a")),
            make_change("conv-1", "device-a", 2, Some("data-a2")), // 同一设备，递增
        ];

        let conflicts = ConflictResolver::detect_conflicts(&changes);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflict() {
        let changes = vec![
            make_change("conv-1", "device-a", 1, Some("data-a")),
            make_change("conv-1", "device-b", 1, Some("data-b")), // 不同设备，并发
        ];

        let conflicts = ConflictResolver::detect_conflicts(&changes);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_resolve_keep_local() {
        let conflict = ConflictInfo {
            id: "test-conflict-id".to_string(),
            entity_type: EntityType::Conversation,
            entity_id: "conv-1".to_string(),
            conflicting_devices: vec!["device-a".to_string(), "device-b".to_string()],
            local_vector: vec![VersionVectorEntry {
                device_id: "device-a".to_string(),
                counter: 1,
            }],
            remote_vector: vec![VersionVectorEntry {
                device_id: "device-b".to_string(),
                counter: 1,
            }],
            local_data: Some("local-data".to_string()),
            remote_data: Some("remote-data".to_string()),
            local_timestamp: 1000,
            remote_timestamp: 2000,
            detected_at: chrono::Utc::now().to_rfc3339(),
            resolved: false,
            resolution_applied: None,
            resolved_at: None,
        };

        let result = ConflictResolver::resolve(&conflict, ConflictResolutionStrategy::KeepLocal);
        assert_eq!(result.resolved_data, Some("local-data".to_string()));
        assert_eq!(result.resolution_applied, "keep_local");
    }

    #[test]
    fn test_auto_resolve() {
        let changes = vec![
            make_change("conv-1", "device-a", 1, Some("data-a")),
            make_change("conv-1", "device-b", 1, Some("data-b")),
        ];

        let result = ConflictResolver::auto_resolve(&changes);
        assert_eq!(result.total_conflicts, 1);
        assert_eq!(result.auto_resolved, 1); // LastWriteWins 可以自动解决
        assert!(result.needs_manual.is_empty());
    }
}
