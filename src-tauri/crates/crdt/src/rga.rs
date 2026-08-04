// SPDX-License-Identifier: AGPL-3.0-only

//! RGA: Replicated Growable Array
//!
//! 可复制增长数组，用于有序数据的 CRDT 实现。
//! 每个元素有唯一 ID 和位置信息（基于左右邻居 ID），
//! 通过比较位置关系确定元素顺序。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// RGA 数组条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RGAEntry {
    /// 条目唯一 ID
    pub id: String,
    /// 条目值
    pub value: Value,
    /// 左邻居 ID（None 表示头部）
    pub left_id: Option<String>,
    /// 右邻居 ID（None 表示尾部）
    pub right_id: Option<String>,
    /// 创建站点 ID
    pub site_id: String,
    /// 创建逻辑时钟
    pub clock: u64,
    /// 是否已删除
    pub deleted: bool,
}

impl RGAEntry {
    /// 创建新条目
    pub fn new(value: Value, site_id: String, clock: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            value,
            left_id: None,
            right_id: None,
            site_id,
            clock,
            deleted: false,
        }
    }
}

/// RGA 实现
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RGA {
    /// 所有条目（包括已删除的）
    entries: Vec<RGAEntry>,
    /// 头部条目 ID
    head_id: Option<String>,
    /// 站点 ID
    site_id: String,
    /// 逻辑时钟
    logical_clock: u64,
}

impl RGA {
    /// 创建新的 RGA
    pub fn new(site_id: String) -> Self {
        Self {
            entries: Vec::new(),
            head_id: None,
            site_id,
            logical_clock: 0,
        }
    }

    /// 获取可见元素列表
    pub fn to_vec(&self) -> Vec<&RGAEntry> {
        let mut result = Vec::new();
        let mut current_id = self.head_id.clone();

        while let Some(id) = current_id {
            if let Some(entry) = self.find_entry(&id) {
                if !entry.deleted {
                    result.push(entry);
                }
                current_id = entry.right_id.clone();
            } else {
                break;
            }
        }
        result
    }

    /// 获取值列表
    pub fn values(&self) -> Vec<&Value> {
        self.to_vec().iter().map(|e| &e.value).collect()
    }

    /// 获取长度
    pub fn len(&self) -> usize {
        self.to_vec().len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 在指定位置插入（本地操作）
    pub fn insert(&mut self, index: usize, value: Value) -> String {
        self.logical_clock += 1;

        let (left_id, right_id) = self.find_position(index);
        let mut entry = RGAEntry::new(value, self.site_id.clone(), self.logical_clock);
        entry.left_id = left_id.clone();
        entry.right_id = right_id.clone();

        // 更新邻居链接
        if let Some(rid) = right_id {
            if let Some(right_entry) = self.find_entry_mut(&rid) {
                right_entry.left_id = Some(entry.id.clone());
            }
        } else {
            // 插入到末尾
            if self.head_id.is_none() {
                self.head_id = Some(entry.id.clone());
            }
        }

        if let Some(lid) = left_id {
            if let Some(left_entry) = self.find_entry_mut(&lid) {
                left_entry.right_id = Some(entry.id.clone());
            }
        } else {
            // 插入到头部
            self.head_id = Some(entry.id.clone());
        }

        let entry_id = entry.id.clone();
        self.entries.push(entry);
        entry_id
    }

    /// 删除指定位置的元素
    pub fn remove(&mut self, index: usize) -> bool {
        let visible = self.to_vec();
        if index >= visible.len() {
            return false;
        }

        let entry_id = visible[index].id.clone();
        self.mark_deleted(&entry_id)
    }

    /// 标记条目为已删除
    fn mark_deleted(&mut self, entry_id: &str) -> bool {
        // 先获取条目信息
        let (left_id, right_id) = {
            if let Some(entry) = self.entries.iter().find(|e| e.id == entry_id) {
                if entry.deleted {
                    return false;
                }
                (entry.left_id.clone(), entry.right_id.clone())
            } else {
                return false;
            }
        };

        // 标记为已删除
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == entry_id) {
            entry.deleted = true;
            entry.left_id = None;
            entry.right_id = None;
        } else {
            return false;
        }

        // 更新邻居链接
        if let Some(lid) = &left_id {
            if let Some(left_entry) = self.entries.iter_mut().find(|e| e.id == *lid) {
                left_entry.right_id = right_id.clone();
            }
        } else {
            // 删除的是头部
            self.head_id = right_id.clone();
        }

        if let Some(rid) = &right_id
            && let Some(right_entry) = self.entries.iter_mut().find(|e| e.id == *rid)
        {
            right_entry.left_id = left_id.clone();
        }

        true
    }

    /// 应用远程插入操作
    pub fn apply_insert(
        &mut self,
        entry: RGAEntry,
        left_id: Option<String>,
        right_id: Option<String>,
    ) {
        let mut new_entry = entry;
        new_entry.left_id = left_id;
        new_entry.right_id = right_id;

        // 更新邻居链接
        if let Some(rid) = &new_entry.right_id
            && let Some(right_entry) = self.find_entry_mut(rid)
        {
            right_entry.left_id = Some(new_entry.id.clone());
        }

        if let Some(lid) = &new_entry.left_id {
            if let Some(left_entry) = self.find_entry_mut(lid) {
                left_entry.right_id = Some(new_entry.id.clone());
            }
        } else {
            // 插入到头部
            self.head_id = Some(new_entry.id.clone());
        }

        self.entries.push(new_entry);
    }

    /// 应用远程删除操作
    pub fn apply_delete(&mut self, entry_id: &str) -> bool {
        self.mark_deleted(entry_id)
    }

    /// 查找位置（返回应该插入的左、右邻居 ID）
    fn find_position(&self, index: usize) -> (Option<String>, Option<String>) {
        if self.head_id.is_none() || index == 0 {
            // 找到第一个可见条目
            let mut current_id = self.head_id.clone();
            while let Some(id) = current_id {
                if let Some(entry) = self.find_entry(&id) {
                    if !entry.deleted {
                        return (None, Some(id));
                    }
                    current_id = entry.right_id.clone();
                } else {
                    break;
                }
            }
            return (None, None);
        }

        let mut count = 0;
        let mut current_id = self.head_id.clone();
        let mut prev_visible_id: Option<String> = None;

        while let Some(id) = current_id {
            if let Some(entry) = self.find_entry(&id) {
                if !entry.deleted {
                    if count == index {
                        return (prev_visible_id, Some(id));
                    }
                    count += 1;
                    prev_visible_id = Some(id);
                }
                current_id = entry.right_id.clone();
            } else {
                break;
            }
        }

        // 到达末尾
        (prev_visible_id, None)
    }

    /// 查找条目
    fn find_entry(&self, id: &str) -> Option<&RGAEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// 查找可变条目
    fn find_entry_mut(&mut self, id: &str) -> Option<&mut RGAEntry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// 合并另一个 RGA
    pub fn merge(&mut self, other: &RGA) {
        for entry in &other.entries {
            if self.find_entry(&entry.id).is_none() {
                if entry.deleted {
                    let new_entry = entry.clone();
                    self.entries.push(new_entry);
                } else {
                    let pos = self.find_position_for_merge(entry);
                    self.apply_insert(entry.clone(), pos.0, pos.1);
                }
            } else if entry.deleted {
                self.mark_deleted(&entry.id);
            }
        }
    }

    /// 为合并操作找到合适的位置
    fn find_position_for_merge(&self, entry: &RGAEntry) -> (Option<String>, Option<String>) {
        // 使用 entry 的 left_id 和 right_id
        let mut left_id = entry.left_id.clone();
        let mut right_id = entry.right_id.clone();

        // 如果 left_id 存在但找不到，尝试查找左邻居
        if let Some(lid) = &left_id
            && self.find_entry(lid).is_none()
        {
            left_id = None;
        }

        // 如果 right_id 存在但找不到，尝试查找右邻居
        if let Some(rid) = &right_id
            && self.find_entry(rid).is_none()
        {
            right_id = None;
        }

        // 如果两个邻居都找不到，比较 clock 值决定位置
        if left_id.is_none() && right_id.is_none() {
            // 找到合适的位置
            let mut current_id = self.head_id.clone();
            let mut prev_id = None;

            while let Some(id) = current_id {
                if let Some(existing) = self.find_entry(&id) {
                    if !existing.deleted {
                        // 比较逻辑时钟
                        if entry.clock < existing.clock
                            || (entry.clock == existing.clock && entry.site_id < existing.site_id)
                        {
                            return (prev_id, Some(id));
                        }
                        prev_id = Some(id);
                    }
                    current_id = existing.right_id.clone();
                } else {
                    break;
                }
            }

            // 插入到末尾
            (prev_id, None)
        } else {
            (left_id, right_id)
        }
    }

    /// 获取快照
    pub fn snapshot(&self) -> RGA {
        self.clone()
    }

    /// 从快照恢复
    pub fn from_snapshot(snapshot: RGA, site_id: String) -> Self {
        Self {
            entries: snapshot.entries,
            head_id: snapshot.head_id,
            site_id,
            logical_clock: snapshot.logical_clock,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_rga_basic() {
        let mut rga = RGA::new("siteA".to_string());
        assert!(rga.is_empty());

        rga.insert(0, json!("first"));
        rga.insert(1, json!("second"));
        rga.insert(2, json!("third"));

        assert_eq!(rga.len(), 3);
        assert_eq!(rga.values()[0], &json!("first"));
        assert_eq!(rga.values()[1], &json!("second"));
        assert_eq!(rga.values()[2], &json!("third"));
    }

    #[test]
    fn test_rga_insert_middle() {
        let mut rga = RGA::new("siteA".to_string());
        rga.insert(0, json!("a"));
        rga.insert(1, json!("c"));
        rga.insert(1, json!("b"));

        assert_eq!(rga.len(), 3);
        let values = rga.values();
        assert_eq!(*values[0], json!("a"));
        assert_eq!(*values[1], json!("b"));
        assert_eq!(*values[2], json!("c"));
    }

    #[test]
    fn test_rga_remove() {
        let mut rga = RGA::new("siteA".to_string());
        rga.insert(0, json!("a"));
        rga.insert(1, json!("b"));
        rga.insert(2, json!("c"));

        assert!(rga.remove(1));
        assert_eq!(rga.len(), 2);
        let values = rga.values();
        assert_eq!(*values[0], json!("a"));
        assert_eq!(*values[1], json!("c"));
    }

    #[test]
    fn test_rga_concurrent_insert() {
        let mut rga1 = RGA::new("siteA".to_string());
        rga1.insert(0, json!("fromA"));

        let mut rga2 = RGA::new("siteB".to_string());
        rga2.insert(0, json!("fromB"));

        // 合并两个 RGA
        rga1.merge(&rga2);

        assert_eq!(rga1.len(), 2);
        let values = rga1.values();
        // 两个元素都应该存在
        assert!(values.contains(&&json!("fromA")));
        assert!(values.contains(&&json!("fromB")));
    }
}