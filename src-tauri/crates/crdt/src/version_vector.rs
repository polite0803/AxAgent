// SPDX-License-Identifier: AGPL-3.0-only

//! 版本向量实现
//!
//! 版本向量用于跟踪每个副本的操作历史，
//! 用于检测并发冲突和确定操作的因果顺序。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 版本向量条目
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VVEntry {
    /// 站点/设备 ID
    pub site_id: String,
    /// 计数器值
    pub counter: u64,
}

/// 版本向量
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionVector {
    /// 向量条目
    entries: HashMap<String, u64>,
}

impl VersionVector {
    /// 创建空版本向量
    pub fn new() -> Self {
        Self { entries: HashMap::new() }
    }

    /// 从条目列表创建
    pub fn from_entries(entries: Vec<VVEntry>) -> Self {
        let mut map = HashMap::new();
        for entry in entries {
            map.insert(entry.site_id, entry.counter);
        }
        Self { entries: map }
    }

    /// 转换为条目列表
    pub fn to_entries(&self) -> Vec<VVEntry> {
        self.entries
            .iter()
            .map(|(site_id, counter)| VVEntry { site_id: site_id.clone(), counter: *counter })
            .collect()
    }

    /// 获取指定站点的计数器值
    pub fn get(&self, site_id: &str) -> u64 {
        self.entries.get(site_id).copied().unwrap_or(0)
    }

    /// 递增指定站点的计数器
    pub fn increment(&mut self, site_id: &str) {
        let counter = self.entries.entry(site_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// 合并另一个版本向量（取最大值）
    pub fn merge(&mut self, other: &VersionVector) {
        for (site_id, counter) in &other.entries {
            let current = self.entries.entry(site_id.clone()).or_insert(0);
            *current = (*current).max(*counter);
        }
    }

    /// 检查是否包含另一个版本向量（所有条目 >= 另一个的对应条目）
    pub fn contains(&self, other: &VersionVector) -> bool {
        other.entries.iter().all(|(site_id, counter)| self.get(site_id) >= *counter)
    }

    /// 检查是否与另一个版本向量并发（互不包含）
    pub fn is_concurrent_with(&self, other: &VersionVector) -> bool {
        !self.contains(other) && !other.contains(self)
    }

    /// 检查是否小于另一个版本向量（严格因果前序）
    pub fn is_before(&self, other: &VersionVector) -> bool {
        self.contains(other) && self != other
    }

    /// 创建用于比较的克隆
    pub fn clone_entries(&self) -> HashMap<String, u64> {
        self.entries.clone()
    }
}

impl std::fmt::Display for VersionVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entries: Vec<String> =
            self.entries.iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
        write!(f, "[{}]", entries.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_vector_basic() {
        let mut vv1 = VersionVector::new();
        assert_eq!(vv1.get("siteA"), 0);

        vv1.increment("siteA");
        assert_eq!(vv1.get("siteA"), 1);

        vv1.increment("siteA");
        assert_eq!(vv1.get("siteA"), 2);
    }

    #[test]
    fn test_version_vector_merge() {
        let mut vv1 = VersionVector::new();
        vv1.increment("siteA");
        vv1.increment("siteA");

        let mut vv2 = VersionVector::new();
        vv2.increment("siteB");

        vv1.merge(&vv2);
        assert_eq!(vv1.get("siteA"), 2);
        assert_eq!(vv1.get("siteB"), 1);
    }

    #[test]
    fn test_version_vector_concurrent() {
        let mut vv1 = VersionVector::new();
        vv1.increment("siteA");

        let mut vv2 = VersionVector::new();
        vv2.increment("siteB");

        assert!(vv1.is_concurrent_with(&vv2));
    }

    #[test]
    fn test_version_vector_causal() {
        let mut vv1 = VersionVector::new();
        vv1.increment("siteA");
        vv1.increment("siteB");

        let mut vv2 = VersionVector::new();
        vv2.increment("siteA");

        assert!(vv1.is_before(&vv2) || vv2.is_before(&vv1) || !vv1.is_concurrent_with(&vv2));
    }
}
