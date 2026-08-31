// SPDX-License-Identifier: AGPL-3.0-only

//! 会话状态表 —— 能力按需加载（CapabilityLoad）状态的持久化载体。
//!
//! # 定位
//! `axagent_harness::session_state::SessionStateStore` 的 SQLite 物化。
//! key 本身携带 scope / namespace / conversation_id / agent_id 四段语义
//! （构造规则见 harness 的 `scoped_key`），本表只做存储与 TTL 过滤，不解释语义。
//!
//! # 主键
//! 自然主键 = `state_key`，upsert 用 ON CONFLICT 覆盖 value 与时间戳。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "session_states")]
pub struct Model {
    /// 状态 key（自然主键，格式见 harness `scoped_key`）
    #[sea_orm(primary_key, auto_increment = false)]
    pub state_key: String,
    /// 状态值（JSON 字符串原文）
    pub state_value: String,
    /// 作用域（temp / session / persistent）
    pub scope: String,
    /// 会话 ID（冗余列，便于按会话批量清理）
    pub conversation_id: Option<String>,
    /// Agent 作用域（冗余列，便于按 Agent 批量清理与审计）
    pub agent_id: Option<String>,
    /// 最后更新时间戳（毫秒）
    pub updated_at_ms: i64,
    /// 过期时间戳（毫秒），NULL 表示不过期
    pub expires_at_ms: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
