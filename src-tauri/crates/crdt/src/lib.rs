// SPDX-License-Identifier: AGPL-3.0-only

//! CRDT 核心实现：Conflict-free Replicated Data Types
//!
//! 实现多设备同步所需的无冲突复制数据类型：
//! - LWWRegister: 最后写入胜出寄存器
//! - ORSet: 观察-删除集合
//! - RGA: 可复制增长数组
//! - GCounter: 增长计数器

pub mod gcounter;
pub mod lww_register;
pub mod or_set;
pub mod rga;
pub mod version_vector;

pub use gcounter::GCounter;
pub use lww_register::LWWRegister;
pub use or_set::ORSet;
pub use rga::{RGA, RGAEntry};
pub use version_vector::{VVEntry, VersionVector};

use serde::{Deserialize, Serialize};

/// CRDT 操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CRDTOperation {
    /// 设置寄存器值
    Set { id: String, value: serde_json::Value, timestamp: u64, site_id: String },
    /// 添加到集合
    Add { id: String, element: String, tag: String, site_id: String },
    /// 从集合移除
    Remove { id: String, element: String, tag: String },
    /// 插入到数组
    Insert {
        id: String,
        position: usize,
        value: serde_json::Value,
        left_origin: Option<String>,
        right_origin: Option<String>,
        site_id: String,
    },
    /// 从数组删除
    Delete { id: String, entry_id: String },
    /// 计数器递增
    Increment { id: String, site_id: String, amount: u64 },
}

/// CRDT 操作日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CRDTLogEntry {
    /// 操作唯一 ID
    pub id: String,
    /// 操作类型
    pub operation: CRDTOperation,
    /// 操作时间戳
    pub timestamp: u64,
    /// 来源设备 ID
    pub site_id: String,
    /// 目标实体 ID
    pub entity_id: String,
    /// 版本向量（操作执行时的版本）
    pub version_vector: Vec<VVEntry>,
}

impl CRDTLogEntry {
    /// 创建新的日志条目
    pub fn new(
        operation: CRDTOperation,
        entity_id: String,
        site_id: String,
        version_vector: Vec<VVEntry>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            operation,
            entity_id,
            site_id,
            version_vector,
        }
    }
}
