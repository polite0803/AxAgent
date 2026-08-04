// SPDX-License-Identifier: AGPL-3.0-only

//! ORSet: Observed-Remove Set
//!
//! 基于标记的集合 CRDT，添加操作带唯一标签，
//! 只有当所有添加标签都被移除时，元素才真正从集合中删除。

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// ORSet 实现
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ORSet {
    /// 元素 → 标签集合映射
    elements: HashMap<String, HashSet<String>>,
    /// 站点 ID
    site_id: String,
}

impl ORSet {
    /// 创建新的 ORSet
    pub fn new(site_id: String) -> Self {
        Self { elements: HashMap::new(), site_id }
    }

    /// 获取当前元素集合
    pub fn elements(&self) -> HashSet<String> {
        self.elements
            .iter()
            .filter(|(_, tags)| !tags.is_empty())
            .map(|(elem, _)| elem.clone())
            .collect()
    }

    /// 检查元素是否存在
    pub fn contains(&self, element: &str) -> bool {
        self.elements.get(element).map(|tags| !tags.is_empty()).unwrap_or(false)
    }

    /// 添加元素（本地操作）
    pub fn add(&mut self, element: String) -> String {
        let tag = self.generate_tag();
        self.add_with_tag(element, tag.clone());
        tag
    }

    /// 使用特定标签添加元素
    fn add_with_tag(&mut self, element: String, tag: String) {
        self.elements.entry(element).or_default().insert(tag);
    }

    /// 移除元素（移除该元素的所有标签）
    pub fn remove(&mut self, element: &str) {
        if let Some(tags) = self.elements.get_mut(element) {
            tags.clear();
        }
    }

    /// 应用远程添加操作
    pub fn apply_add(&mut self, element: String, tag: String) {
        self.add_with_tag(element, tag);
    }

    /// 应用远程移除操作
    pub fn apply_remove(&mut self, element: &str, tag: &str) {
        if let Some(tags) = self.elements.get_mut(element) {
            tags.remove(tag);
        }
    }

    /// 合并另一个 ORSet
    pub fn merge(&mut self, other: &ORSet) {
        for (element, tags) in &other.elements {
            for tag in tags {
                if !tag.is_empty() {
                    self.add_with_tag(element.clone(), tag.clone());
                }
            }
        }
    }

    /// 生成唯一标签
    fn generate_tag(&self) -> String {
        format!("{}-{}", self.site_id, uuid::Uuid::new_v4())
    }

    /// 获取快照
    pub fn snapshot(&self) -> ORSetSnapshot {
        ORSetSnapshot { elements: self.elements.clone() }
    }

    /// 从快照恢复
    pub fn from_snapshot(snapshot: ORSetSnapshot, site_id: String) -> Self {
        Self { elements: snapshot.elements, site_id }
    }
}

/// ORSet 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ORSetSnapshot {
    pub elements: HashMap<String, HashSet<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orset_basic() {
        let mut set = ORSet::new("siteA".to_string());
        assert!(!set.contains("item1"));

        let _tag = set.add("item1".to_string());
        assert!(set.contains("item1"));

        set.remove("item1");
        assert!(!set.contains("item1"));
    }

    #[test]
    fn test_orset_concurrent_add() {
        let mut set1 = ORSet::new("siteA".to_string());
        let tag1 = set1.add("item".to_string());

        let mut set2 = ORSet::new("siteB".to_string());
        set2.apply_add("item".to_string(), "tagB".to_string());

        // 合并后元素应该存在（有两个标签）
        set1.merge(&set2);
        assert!(set1.contains("item"));

        // 移除一个标签后元素仍存在
        set1.apply_remove("item", &tag1);
        assert!(set1.contains("item"));

        // 移除所有标签后元素不存在
        set1.apply_remove("item", "tagB");
        assert!(!set1.contains("item"));
    }

    #[test]
    fn test_orset_merge() {
        let mut set1 = ORSet::new("siteA".to_string());
        set1.add("a".to_string());
        set1.add("b".to_string());

        let mut set2 = ORSet::new("siteB".to_string());
        set2.add("b".to_string());
        set2.add("c".to_string());

        set1.merge(&set2);
        let elements = set1.elements();
        assert_eq!(elements.len(), 3);
        assert!(elements.contains("a"));
        assert!(elements.contains("b"));
        assert!(elements.contains("c"));
    }
}
