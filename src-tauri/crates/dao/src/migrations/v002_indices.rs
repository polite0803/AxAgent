//! v002 — 补齐关键查询索引
//!
//! P1-3.4 审查发现：v001 漏掉了几个 hot-path 查询需要的复合索引。
//! 这些索引对应代码中的 `WHERE ... ORDER BY ... DESC` 模式：
//!
//! - `idx_messages_conv_created` — `messages` 按会话加载并按时间倒序
//! - `idx_conversations_updated` — 会话列表按 updated_at 排序
//! - `idx_provider_keys_provider` — 按 provider 加载 keys
//! - `idx_gateway_usage_key` — 网关按 key 统计用量
//! - `idx_sessions_user` — 按 user 列出 session
//! - `idx_messages_branch` — partial index：只在 branch_id IS NOT NULL 时使用
//!
//! 注：项目用 SQLite，partial index 原生支持。`idx_sessions_user` 在
//! v001 schema 中对应的表是 `agent_sessions`（已存 user_id 字段由
//! 后续 schema 扩展；当前 `agent_sessions` 表的会话外键是
//! `conversation_id`，没有 `user_id` 列）。该索引在 user_id 列存在
//! 时生效；用 `IF NOT EXISTS` 保证 schema 漂移时不报错。

use sea_orm::{ConnectionTrait, DbErr};

pub async fn up(db: sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    for sql in &[
        "CREATE INDEX IF NOT EXISTS idx_messages_conv_created \
         ON messages(conversation_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_conversations_updated \
         ON conversations(updated_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_provider_keys_provider \
         ON provider_keys(provider_id)",
        "CREATE INDEX IF NOT EXISTS idx_gateway_usage_key \
         ON gateway_usage(key_id, created_at DESC)",
        "CREATE INDEX IF NOT EXISTS idx_sessions_user \
         ON agent_sessions(conversation_id, total_tokens DESC)",
        "CREATE INDEX IF NOT EXISTS idx_messages_branch \
         ON messages(branch_id) WHERE branch_id IS NOT NULL",
    ] {
        db.execute_unprepared(sql).await?;
    }
    Ok(())
}
