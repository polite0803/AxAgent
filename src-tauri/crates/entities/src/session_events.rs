// SPDX-License-Identifier: AGPL-3.0-only
//! session_events 表 —— 跨进程 Resume 事件流（PLAN-codex-parity P0-3）。
//!
//! 事件类型：TurnStarted / Message / ToolCall / ToolResult / Compacted / TurnEnded / Interrupted。
//! 与 messages 表（对话文本）互补：session_events 只存**执行态**事件，不存 LLM 输出。

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize, serde::Deserialize)]
#[sea_orm(table_name = "session_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 会话 ID（conversation_id — 跨进程恢复的 key）
    pub session_id: String,
    /// 同一 session 内的递增序号（用于事件流顺序 + 未配对检测）
    pub seq: i64,
    /// 事件类型（snake_case 字符串：turn_started / tool_call / ...）
    pub event_type: String,
    /// 事件 payload（JSON，nullable，不同事件类型结构不同）
    pub payload: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
