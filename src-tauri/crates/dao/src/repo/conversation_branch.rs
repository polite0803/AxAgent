// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::{conversation_branches, messages};
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{BranchComparison, ConversationBranch, MessageSummary};
use axagent_harness::util_fns::{gen_id, now_datetime_str};

fn model_to_branch(m: conversation_branches::Model) -> ConversationBranch {
    ConversationBranch {
        id: m.id,
        conversation_id: m.conversation_id,
        parent_message_id: m.parent_message_id,
        branch_label: m.branch_label,
        branch_index: m.branch_index,
        compared_message_ids_json: m.compared_message_ids_json,
        created_at: m.created_at,
    }
}

pub async fn list_branches(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<Vec<ConversationBranch>> {
    let models = conversation_branches::Entity::find()
        .filter(conversation_branches::Column::ConversationId.eq(conversation_id))
        .order_by_asc(conversation_branches::Column::BranchIndex)
        .all(db)
        .await?;

    Ok(models.into_iter().map(model_to_branch).collect())
}

pub async fn get_branch(db: &DatabaseConnection, id: &str) -> Result<ConversationBranch> {
    let model = conversation_branches::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("ConversationBranch {}", id)))?;

    Ok(model_to_branch(model))
}

pub async fn create_branch(
    db: &DatabaseConnection,
    conversation_id: &str,
    parent_message_id: &str,
    label: &str,
) -> Result<ConversationBranch> {
    let id = gen_id();
    let now = now_datetime_str();

    let backend = db.get_database_backend();
    let (count_sql, count_values): (&str, Vec<sea_orm::Value>) = match backend {
        sea_orm::DatabaseBackend::Postgres => (
            "SELECT COALESCE(MAX(branch_index), -1) + 1 FROM conversation_branches WHERE conversation_id = $1",
            vec![conversation_id.into()],
        ),
        _ => (
            "SELECT COALESCE(MAX(branch_index), -1) + 1 FROM conversation_branches WHERE conversation_id = ?",
            vec![conversation_id.into()],
        ),
    };

    let count_result =
        db.query_one_raw(Statement::from_sql_and_values(backend, count_sql, count_values)).await?;

    let next_index: i32 =
        count_result.and_then(|row| row.try_get_by_index::<i32>(0).ok()).unwrap_or(0);

    let am = conversation_branches::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(conversation_id.to_string()),
        parent_message_id: Set(parent_message_id.to_string()),
        branch_label: Set(label.to_string()),
        branch_index: Set(next_index),
        compared_message_ids_json: Set(None),
        created_at: Set(now),
    };

    am.insert(db).await?;

    get_branch(db, &id).await
}

// ── 2.6 P1:分支对比 ──

/// 取消息内容前 200 字符(UTF-8 安全截断)。
fn preview_content(content: &str) -> String {
    const PREVIEW_LEN: usize = 200;
    if content.len() <= PREVIEW_LEN {
        return content.to_string();
    }
    let mut boundary = PREVIEW_LEN;
    while boundary > 0 && !content.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &content[..boundary])
}

fn message_to_summary(m: messages::Model) -> MessageSummary {
    MessageSummary {
        id: m.id,
        role: m.role,
        content_preview: preview_content(&m.content),
        created_at: m.created_at,
        parent_message_id: m.parent_message_id,
    }
}

/// 对比两个分支的消息差异。
///
/// 算法:
/// 1. 读取两个分支的元数据(branch_a, branch_b),它们的 `parent_message_id` 是分叉点
/// 2. 读取两个分支各自的消息列表(按 `messages.branch_id` 过滤)
/// 3. 共享前缀 = 从会话起点到分叉点的消息(按 `conversation_id` + `created_at` 早于分叉点 + `is_active=1`)
/// 4. only_in_a / only_in_b = 各自分支的消息
/// 5. diverge_at = 两个分支的 `parent_message_id`(应当相同,若不同则取 branch_a 的)
pub async fn compare_branches(
    db: &DatabaseConnection,
    branch_a_id: &str,
    branch_b_id: &str,
) -> Result<BranchComparison> {
    let branch_a = get_branch(db, branch_a_id).await?;
    let branch_b = get_branch(db, branch_b_id).await?;

    // 分叉点:两个分支共享的最近父消息(空字符串视为无分叉点)
    let diverge_at: Option<String> = if branch_a.parent_message_id.is_empty() {
        None
    } else {
        Some(branch_a.parent_message_id.clone())
    };

    // 读取共享前缀(从会话起点到 diverge_at,不含分叉点之后的分支消息)
    // 策略:取 conversation_id 下所有 is_active=1 的消息,过滤 created_at <= diverge 消息的 created_at
    //       且 branch_id IS NULL(主分支消息)
    let common_prefix: Vec<MessageSummary> = match diverge_at.as_ref() {
        None => Vec::new(),
        Some(diverge_msg_id) => {
            let diverge_msg = messages::Entity::find_by_id(diverge_msg_id)
                .one(db)
                .await?
                .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", diverge_msg_id)))?;

            let mut query = messages::Entity::find()
                .filter(messages::Column::ConversationId.eq(&branch_a.conversation_id))
                .filter(messages::Column::IsActive.eq(1))
                .filter(messages::Column::BranchId.is_null());

            // diverge 消息本身也算入 common_prefix(它是分叉点,两个分支都共享)
            query = query.filter(
                Condition::any()
                    .add(messages::Column::CreatedAt.lt(diverge_msg.created_at))
                    .add(messages::Column::Id.eq(diverge_msg.id.clone())),
            );

            let rows = query
                .order_by_asc(messages::Column::CreatedAt)
                .order_by_asc(messages::Column::Id)
                .all(db)
                .await?;

            rows.into_iter().map(message_to_summary).collect::<Vec<_>>()
        },
    };

    // 读取 branch_a 的消息(按 branch_id 过滤)
    let only_in_a = messages::Entity::find()
        .filter(messages::Column::BranchId.eq(branch_a_id))
        .filter(messages::Column::IsActive.eq(1))
        .order_by_asc(messages::Column::CreatedAt)
        .order_by_asc(messages::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(message_to_summary)
        .collect::<Vec<_>>();

    // 读取 branch_b 的消息
    let only_in_b = messages::Entity::find()
        .filter(messages::Column::BranchId.eq(branch_b_id))
        .filter(messages::Column::IsActive.eq(1))
        .order_by_asc(messages::Column::CreatedAt)
        .order_by_asc(messages::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(message_to_summary)
        .collect::<Vec<_>>();

    Ok(BranchComparison { branch_a, branch_b, common_prefix, only_in_a, only_in_b, diverge_at })
}

#[cfg(test)]
mod tests_2_6 {
    use super::*;

    #[test]
    fn test_preview_content_short() {
        let s = preview_content("hello world");
        assert_eq!(s, "hello world");
    }

    #[test]
    fn test_preview_content_long_ascii() {
        let long = "a".repeat(500);
        let s = preview_content(&long);
        assert!(s.ends_with("..."));
        assert!(s.len() < 210); // 200 + "..."
    }

    #[test]
    fn test_preview_content_utf8_safe() {
        // 中文字符每个 3 字节,200 字节处可能落在字符中间
        let long = "中".repeat(100); // 300 字节
        let s = preview_content(&long);
        assert!(s.ends_with("..."));
        // 截断后不应产生无效 UTF-8(Rust 字符串保证)
        assert!(s.chars().count() > 0);
    }

    #[test]
    fn test_message_to_summary_basic() {
        let m = messages::Model {
            id: "msg1".to_string(),
            conversation_id: "c1".to_string(),
            role: "user".to_string(),
            content: "hello".to_string(),
            provider_id: None,
            model_id: None,
            token_count: None,
            prompt_tokens: None,
            completion_tokens: None,
            attachments: "[]".to_string(),
            thinking: None,
            created_at: 1000,
            branch_id: None,
            parent_message_id: None,
            version_index: 0,
            is_active: 1,
            tool_calls_json: None,
            tool_call_id: None,
            status: "complete".to_string(),
            tokens_per_second: None,
            first_token_latency_ms: None,
            cache_creation_tokens: None,
            cache_read_tokens: None,
            parts: None,
            quoted_message_id: None,
            decision: None,
        };
        let s = message_to_summary(m);
        assert_eq!(s.id, "msg1");
        assert_eq!(s.role, "user");
        assert_eq!(s.content_preview, "hello");
        assert_eq!(s.created_at, 1000);
        assert_eq!(s.parent_message_id, None);
    }
}
