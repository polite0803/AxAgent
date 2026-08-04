// SPDX-License-Identifier: AGPL-3.0-only

//! LWWRegister: Last-Writer-Wins 寄存器
//!
//! 用于存储单个值的 CRDT，冲突解决策略为最后写入胜出。
//! 当两个副本并发修改时，时间戳更大的胜出。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// LWWRegister 实现
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LWWRegister {
    /// 当前值
    value: Option<Value>,
    /// 最后更新时间戳
    timestamp: u64,
    /// 最后更新的站点 ID
    site_id: String,
    /// 逻辑时钟（用于同时间戳场景下的确定性比较）
    logical_clock: u64,
}

impl LWWRegister {
    /// 创建新的 LWWRegister
    pub fn new(site_id: String) -> Self {
        Self { value: None, timestamp: 0, site_id, logical_clock: 0 }
    }

    /// 获取当前值
    pub fn get(&self) -> Option<&Value> {
        self.value.as_ref()
    }

    /// 设置值（本地操作）
    pub fn set(&mut self, value: Value, timestamp: u64) {
        self.logical_clock += 1;
        self.apply_update(value, timestamp, self.site_id.clone(), self.logical_clock);
    }

    /// 应用远程更新
    pub fn apply_update(
        &mut self,
        value: Value,
        timestamp: u64,
        site_id: String,
        logical_clock: u64,
    ) {
        // 比较逻辑：时间戳优先，然后是逻辑时钟，最后是站点 ID 作为确定性打破
        if self.should_replace(timestamp, logical_clock, &site_id) {
            self.value = Some(value);
            self.timestamp = timestamp;
            self.site_id = site_id;
            self.logical_clock = logical_clock;
        }
    }

    /// 判断是否应该用新值替换当前值
    fn should_replace(&self, timestamp: u64, logical_clock: u64, site_id: &str) -> bool {
        if timestamp > self.timestamp {
            return true;
        }
        if timestamp == self.timestamp && logical_clock > self.logical_clock {
            return true;
        }
        if timestamp == self.timestamp
            && logical_clock == self.logical_clock
            && site_id > &self.site_id
        {
            return true;
        }
        false
    }

    /// 获取当前状态快照
    pub fn snapshot(&self) -> LWWRegisterSnapshot {
        LWWRegisterSnapshot {
            value: self.value.clone(),
            timestamp: self.timestamp,
            site_id: self.site_id.clone(),
            logical_clock: self.logical_clock,
        }
    }

    /// 从快照恢复
    pub fn from_snapshot(snapshot: LWWRegisterSnapshot, site_id: String) -> Self {
        Self {
            value: snapshot.value,
            timestamp: snapshot.timestamp,
            site_id,
            logical_clock: snapshot.logical_clock,
        }
    }

    /// 与另一个 LWWRegister 合并
    pub fn merge(&mut self, other: &LWWRegister) {
        if let Some(value) = other.value.clone() {
            self.apply_update(value, other.timestamp, other.site_id.clone(), other.logical_clock);
        }
    }
}

impl Default for LWWRegister {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// LWWRegister 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LWWRegisterSnapshot {
    pub value: Option<Value>,
    pub timestamp: u64,
    pub site_id: String,
    pub logical_clock: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lww_register_basic() {
        let mut reg = LWWRegister::new("siteA".to_string());
        assert!(reg.get().is_none());

        reg.set(Value::String("hello".to_string()), 100);
        assert_eq!(reg.get(), Some(&Value::String("hello".to_string())));
    }

    #[test]
    fn test_lww_register_conflict_resolution() {
        let mut reg1 = LWWRegister::new("siteA".to_string());
        reg1.set(Value::String("value1".to_string()), 100);

        let mut reg2 = LWWRegister::new("siteB".to_string());
        reg2.set(Value::String("value2".to_string()), 200);

        // 合并：reg2 的时间戳更大，应该胜出
        reg1.merge(&reg2);
        assert_eq!(reg1.get(), Some(&Value::String("value2".to_string())));
    }

    #[test]
    fn test_lww_register_snapshot() {
        let mut reg = LWWRegister::new("siteA".to_string());
        reg.set(Value::String("test".to_string()), 123);

        let snapshot = reg.snapshot();
        let restored = LWWRegister::from_snapshot(snapshot, "siteA".to_string());

        assert_eq!(restored.get(), Some(&Value::String("test".to_string())));
    }
}
