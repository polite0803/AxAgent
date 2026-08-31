// SPDX-License-Identifier: AGPL-3.0-only

//! 会话状态 repository —— `SessionStateStore` 的 SQLite 实现。
//!
//! # 职责
//! - 承载 `CapabilityLoad` 写入的加载状态，供下一轮 Processor 读回注入
//! - TTL 过滤在读取侧完成（过期即视为不存在），清理靠 `purge_expired`
//!
//! # 与 harness 契约的关系
//! key 的构造与语义解释全在 `axagent_harness::session_state`，本模块只做存储。
//! 冗余的 `conversation_id` / `agent_id` 列从 key 解析填充，仅服务索引清理，
//! **不参与任何语义判断** —— 避免冗余列与 key 语义漂移。

use axagent_entities::session_states;
use axagent_harness::session_state::SessionStateEntry;
use axagent_harness::util_fns::now_ms;
use sea_orm::*;

/// 取 key 的作用域前缀部分（`/subject` 之前）。
///
/// `temp:skill:loaded:conv-42:agent-a/tool:read_file`
///   → `temp:skill:loaded:conv-42:agent-a`
fn scope_prefix(key: &str) -> Option<&str> {
    key.rsplit_once('/').map(|(prefix, _)| prefix)
}

/// 从 key 中解析出 agent 段 —— 作用域前缀的**最后**一段。
///
/// 必须**从右往左**取：namespace 自身含冒号（`skill:loaded`），
/// 从左按下标取段会随 namespace 变化而错位。
/// 解析失败返回 `None` —— 冗余列只是清理辅助，解析不出来不应阻断写入。
fn parse_agent_id(key: &str) -> Option<String> {
    let agent = scope_prefix(key)?.rsplit(':').next()?;
    if agent.is_empty() {
        None
    } else {
        Some(agent.to_string())
    }
}

/// 从 key 中解析出 conversation 段 —— 作用域前缀的**倒数第二**段。
fn parse_conversation_id(key: &str) -> Option<String> {
    let conv = scope_prefix(key)?.rsplit(':').nth(1)?;
    if conv.is_empty() {
        None
    } else {
        Some(conv.to_string())
    }
}

/// 从 key 中解析出 scope（首段）。
fn parse_scope(key: &str) -> String {
    key.split(':').next().unwrap_or("temp").to_string()
}

/// 写入（upsert）。`ttl_ms` 为 `None` 时 `expires_at_ms` 置 NULL（不过期）。
///
/// 幂等：同 key 重复写入覆盖 value 与时间戳。
pub async fn set(
    db: &DatabaseConnection,
    key: &str,
    value: &str,
    ttl_ms: Option<i64>,
) -> Result<(), DbErr> {
    let now = now_ms();
    let expires_at_ms = ttl_ms.map(|ttl| now.saturating_add(ttl));

    let am = session_states::ActiveModel {
        state_key: Set(key.to_string()),
        state_value: Set(value.to_string()),
        scope: Set(parse_scope(key)),
        conversation_id: Set(parse_conversation_id(key)),
        agent_id: Set(parse_agent_id(key)),
        updated_at_ms: Set(now),
        expires_at_ms: Set(expires_at_ms),
    };

    let _ = session_states::Entity::insert(am)
        .on_conflict(
            sea_query::OnConflict::column(session_states::Column::StateKey)
                .update_columns([
                    session_states::Column::StateValue,
                    session_states::Column::Scope,
                    session_states::Column::ConversationId,
                    session_states::Column::AgentId,
                    session_states::Column::UpdatedAtMs,
                    session_states::Column::ExpiresAtMs,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;

    Ok(())
}

/// 读取。过期条目视为不存在。
pub async fn get(db: &DatabaseConnection, key: &str) -> Result<Option<String>, DbErr> {
    let now = now_ms();
    let row = session_states::Entity::find_by_id(key.to_string()).one(db).await?;

    Ok(row.and_then(|m| match m.expires_at_ms {
        Some(exp) if exp <= now => None,
        _ => Some(m.state_value),
    }))
}

/// 删除。key 不存在时静默成功。
pub async fn delete(db: &DatabaseConnection, key: &str) -> Result<(), DbErr> {
    session_states::Entity::delete_by_id(key.to_string()).exec(db).await?;
    Ok(())
}

/// 按前缀列出条目（已过滤过期项）。
///
/// `LIKE` 的转义：`%` / `_` 在 key 里合法出现，必须转义，否则前缀会退化成通配符。
pub async fn list_by_prefix(
    db: &DatabaseConnection,
    prefix: &str,
) -> Result<Vec<SessionStateEntry>, DbErr> {
    let now = now_ms();
    let escaped = prefix.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let pattern = format!("{escaped}%");

    let rows = session_states::Entity::find()
        .filter(session_states::Column::StateKey.like(&pattern))
        .all(db)
        .await?;

    Ok(rows
        .into_iter()
        .filter(|m| !matches!(m.expires_at_ms, Some(exp) if exp <= now))
        .map(|m| SessionStateEntry {
            key: m.state_key,
            value: m.state_value,
            scope: m.scope,
            conversation_id: m.conversation_id,
            agent_id: m.agent_id,
            updated_at_ms: m.updated_at_ms,
            expires_at_ms: m.expires_at_ms,
        })
        .collect())
}

/// 物理清理过期条目，返回删除条数。
pub async fn purge_expired(db: &DatabaseConnection) -> Result<usize, DbErr> {
    let now = now_ms();
    let res = session_states::Entity::delete_many()
        .filter(session_states::Column::ExpiresAtMs.is_not_null())
        .filter(session_states::Column::ExpiresAtMs.lte(now))
        .exec(db)
        .await?;
    Ok(res.rows_affected as usize)
}

/// 按会话清理全部状态（会话结束/删除时调用）。
pub async fn delete_by_conversation(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<usize, DbErr> {
    let res = session_states::Entity::delete_many()
        .filter(session_states::Column::ConversationId.eq(conversation_id.to_string()))
        .exec(db)
        .await?;
    Ok(res.rows_affected as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_agent_id_from_scoped_key() {
        let k = "temp:skill:loaded:conv-42:agent-a/tool:read_file";
        assert_eq!(parse_agent_id(k).as_deref(), Some("agent-a"));
    }

    #[test]
    fn parse_conversation_id_from_scoped_key() {
        let k = "temp:skill:loaded:conv-42:agent-a/tool:read_file";
        assert_eq!(parse_conversation_id(k).as_deref(), Some("conv-42"));
    }

    /// 防回归：namespace 自身含冒号时，从左按下标取段会错位 —— 必须从右取。
    #[test]
    fn parse_is_robust_to_colons_in_namespace() {
        let k = "temp:skill:loaded:conv-42:agent-a/tool:read_file";
        assert_eq!(parse_agent_id(k).as_deref(), Some("agent-a"));
        assert_eq!(parse_conversation_id(k).as_deref(), Some("conv-42"));

        // 换一个同样含冒号的 namespace，结论不变
        let k2 = "session:a:b:c:conv-9:agent-b/sub";
        assert_eq!(parse_agent_id(k2).as_deref(), Some("agent-b"));
        assert_eq!(parse_conversation_id(k2).as_deref(), Some("conv-9"));
    }

    /// 无 `/subject` 的畸形 key 不应 panic，返回 None 即可。
    #[test]
    fn parse_returns_none_for_malformed_key() {
        assert_eq!(parse_agent_id("no-slash-here"), None);
        assert_eq!(parse_conversation_id("no-slash-here"), None);
    }

    #[test]
    fn parse_scope_from_scoped_key() {
        assert_eq!(parse_scope("temp:skill:loaded:c1:a/x"), "temp");
        assert_eq!(parse_scope("persistent:ns:c1:a/x"), "persistent");
    }

    #[test]
    fn like_metacharacters_are_escaped() {
        // 前缀含 % 与 _ 时不能退化成通配符
        let escaped = "a%b_c".replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
        assert_eq!(escaped, "a\\%b\\_c");
    }
}
