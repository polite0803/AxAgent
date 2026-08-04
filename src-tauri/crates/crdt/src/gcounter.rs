// SPDX-License-Identifier: AGPL-3.0-only

//! GCounter: Grow-only Counter
//!
//! 增长计数器 CRDT，每个站点只能递增自己的计数器值，
//! 通过合并各站点的值实现分布式计数。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GCounter 实现
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCounter {
    /// 各站点的计数器值
    counters: HashMap<String, u64>,
    /// 站点 ID
    site_id: String,
}

impl GCounter {
    /// 创建新的 GCounter
    pub fn new(site_id: String) -> Self {
        Self {
            counters: HashMap::new(),
            site_id,
        }
    }

    /// 获取计数器总值
    pub fn value(&self) -> u64 {
        self.counters.values().sum()
    }

    /// 获取指定站点的值
    pub fn get(&self, site_id: &str) -> u64 {
        self.counters.get(site_id).copied().unwrap_or(0)
    }

    /// 递增计数器（本地操作）
    pub fn increment(&mut self, amount: u64) {
        if amount > 0 {
            let counter = self.counters.entry(self.site_id.clone()).or_insert(0);
            *counter += amount;
        }
    }

    /// 应用远程递增操作
    pub fn apply_remote_increment(&mut self, site_id: &str, amount: u64) {
        if amount > 0 {
            let counter = self.counters.entry(site_id.to_string()).or_insert(0);
            *counter = (*counter).max(amount);
        }
    }

    /// 合并另一个 GCounter
    pub fn merge(&mut self, other: &GCounter) {
        for (site_id, value) in &other.counters {
            let current = self.counters.entry(site_id.clone()).or_insert(0);
            *current = (*current).max(*value);
        }
    }

    /// 获取快照
    pub fn snapshot(&self) -> GCounterSnapshot {
        GCounterSnapshot {
            counters: self.counters.clone(),
        }
    }

    /// 从快照恢复
    pub fn from_snapshot(snapshot: GCounterSnapshot, site_id: String) -> Self {
        Self {
            counters: snapshot.counters,
            site_id,
        }
    }
}

/// GCounter 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GCounterSnapshot {
    pub counters: HashMap<String, u64>,
}

impl PartialEq for GCounter {
    fn eq(&self, other: &Self) -> bool {
        self.value() == other.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcounter_basic() {
        let mut counter = GCounter::new("siteA".to_string());
        assert_eq!(counter.value(), 0);

        counter.increment(5);
        assert_eq!(counter.value(), 5);
        assert_eq!(counter.get("siteA"), 5);
    }

    #[test]
    fn test_gcounter_merge() {
        let mut counter1 = GCounter::new("siteA".to_string());
        counter1.increment(10);

        let mut counter2 = GCounter::new("siteB".to_string());
        counter2.increment(20);

        counter1.merge(&counter2);
        assert_eq!(counter1.value(), 30);
        assert_eq!(counter1.get("siteA"), 10);
        assert_eq!(counter1.get("siteB"), 20);
    }

    #[test]
    fn test_gcounter_concurrent() {
        let mut counter1 = GCounter::new("siteA".to_string());
        counter1.increment(5);

        let mut counter2 = GCounter::new("siteA".to_string());
        counter2.increment(10);

        // 同一站点的合并取最大值
        counter1.merge(&counter2);
        assert_eq!(counter1.get("siteA"), 10);
        assert_eq!(counter1.value(), 10);
    }

    #[test]
    fn test_gcounter_snapshot() {
        let mut counter = GCounter::new("siteA".to_string());
        counter.increment(42);

        let snapshot = counter.snapshot();
        let restored = GCounter::from_snapshot(snapshot, "siteA".to_string());

        assert_eq!(restored.value(), 42);
    }
}