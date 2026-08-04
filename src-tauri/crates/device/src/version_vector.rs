// SPDX-License-Identifier: AGPL-3.0-only

//! 版本向量实现
//!
//! 用于检测多设备并发修改时的冲突。
//! 每个设备维护自己的计数器，通过比较版本向量来判断：
//! - 因果先后（一个版本是另一个的直接前驱）
//! - 并发冲突（两个版本不可比较）

use axagent_harness::device_sync::VersionVectorEntry;
use std::collections::HashMap;

/// 版本向量：每个设备有一个递增计数器
#[derive(Debug, Clone, Default)]
pub struct VersionVector {
    entries: HashMap<String, u64>,
}

impl VersionVector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从条目列表构建
    pub fn from_entries(entries: &[VersionVectorEntry]) -> Self {
        let mut map = HashMap::new();
        for entry in entries {
            map.insert(entry.device_id.clone(), entry.counter);
        }
        Self { entries: map }
    }

    /// 转换为条目列表
    pub fn to_entries(&self) -> Vec<VersionVectorEntry> {
        self.entries
            .iter()
            .map(|(device_id, counter)| VersionVectorEntry {
                device_id: device_id.clone(),
                counter: *counter,
            })
            .collect()
    }

    /// 获取指定设备的计数器
    pub fn get(&self, device_id: &str) -> u64 {
        self.entries.get(device_id).copied().unwrap_or(0)
    }

    /// 递增指定设备的计数器
    pub fn increment(&mut self, device_id: &str) {
        let counter = self.entries.entry(device_id.to_string()).or_insert(0);
        *counter += 1;
    }

    /// 合并另一个版本向量（取每个设备的最大值）
    pub fn merge(&mut self, other: &VersionVector) {
        for (device_id, counter) in &other.entries {
            let entry = self.entries.entry(device_id.clone()).or_insert(0);
            *entry = (*entry).max(*counter);
        }
    }

    /// 判断与另一个版本向量的关系
    /// Returns:
    ///   - `CausalOrder::Before` 如果 self 是 other 的因果前驱
    ///   - `CausalOrder::After` 如果 self 是 other 的因果后继
    ///   - `CausalOrder::Concurrent` 如果两者并发（冲突）
    pub fn compare(&self, other: &VersionVector) -> CausalOrder {
        let mut self_le = true; // self <= other (可能是前驱)
        let mut self_ge = true; // self >= other (可能是后继)
        let mut has_strict = false; // 存在严格不等

        // 检查所有设备
        let all_devices: std::collections::HashSet<_> =
            self.entries.keys().chain(other.entries.keys()).cloned().collect();

        for device_id in &all_devices {
            let a = self.get(device_id);
            let b = other.get(device_id);

            if a > b {
                self_le = false;
            }
            if a < b {
                self_ge = false;
            }
            if a != b {
                has_strict = true;
            }
        }

        match (self_le, self_ge, has_strict) {
            (true, true, false) => CausalOrder::Equal,
            (true, true, true) => CausalOrder::Before, // self < other (严格小于)
            (true, false, _) => CausalOrder::Before,   // self <= other 且存在 <
            (false, true, _) => CausalOrder::After,    // self >= other 且存在 >
            (false, false, _) => CausalOrder::Concurrent,
        }
    }

    /// 检查是否与另一个版本向量并发（即存在冲突）
    pub fn is_concurrent_with(&self, other: &VersionVector) -> bool {
        matches!(self.compare(other), CausalOrder::Concurrent)
    }
}

/// 因果顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalOrder {
    /// self 是 other 的因果前驱（self 先发生）
    Before,
    /// self 是 other 的因果后继（self 后发生）
    After,
    /// 两者相等
    Equal,
    /// 两者并发（不可比较，存在冲突）
    Concurrent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_vector() {
        let vv = VersionVector::new();
        assert_eq!(vv.get("device-a"), 0);
        assert!(matches!(vv.compare(&VersionVector::new()), CausalOrder::Equal));
    }

    #[test]
    fn test_increment() {
        let mut vv = VersionVector::new();
        vv.increment("device-a");
        vv.increment("device-a");
        assert_eq!(vv.get("device-a"), 2);
    }

    #[test]
    fn test_causal_order() {
        let mut vv1 = VersionVector::new();
        vv1.increment("device-a");

        let mut vv2 = vv1.clone();
        vv2.increment("device-a");

        assert!(matches!(vv1.compare(&vv2), CausalOrder::Before));
        assert!(matches!(vv2.compare(&vv1), CausalOrder::After));
    }

    #[test]
    fn test_concurrent() {
        let mut vv1 = VersionVector::new();
        vv1.increment("device-a");

        let mut vv2 = VersionVector::new();
        vv2.increment("device-b");

        assert!(matches!(vv1.compare(&vv2), CausalOrder::Concurrent));
        assert!(vv1.is_concurrent_with(&vv2));
    }

    #[test]
    fn test_merge() {
        let mut vv1 = VersionVector::new();
        vv1.increment("device-a");
        vv1.increment("device-b");

        let mut vv2 = VersionVector::new();
        vv2.increment("device-b");
        vv2.increment("device-c");

        vv1.merge(&vv2);
        assert_eq!(vv1.get("device-a"), 1);
        assert_eq!(vv1.get("device-b"), 1); // max(1, 1) = 1
        assert_eq!(vv1.get("device-c"), 1);
    }
}
