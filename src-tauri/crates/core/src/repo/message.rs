use sea_orm::sea_query::Expr;
use sea_orm::*;
use std::collections::HashSet;

use crate::entity::messages;
use crate::error::{AxAgentError, Result};
use crate::types::{Attachment, ConversationStats, Message, MessagePage, MessageRole};
use crate::utils::{gen_id, now_ts};

fn parse_role(s: &str) -> MessageRole {
    match s {
        "system" => MessageRole::System,
        "user" => MessageRole::User,
        "tool" => MessageRole::Tool,
        _ => MessageRole::Assistant,
    }
}

fn role_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    }
}

fn parse_attachment_list(raw: &str) -> Result<Vec<Attachment>> {
    serde_json::from_str(raw)
        .map_err(|e| AxAgentError::Validation(format!("Invalid message attachments JSON: {e}")))
}

fn stringify_attachment_list(attachments: &[Attachment]) -> Result<String> {
    serde_json::to_string(attachments).map_err(|e| {
        AxAgentError::Validation(format!("Failed to serialize message attachments: {e}"))
    })
}

fn message_from_entity(m: messages::Model) -> Result<Message> {
    let blocks = m.parts.as_ref()
        .and_then(|p| serde_json::from_str::<Vec<crate::types::ContentBlock>>(p).ok());
    Ok(Message {
        id: m.id,
        conversation_id: m.conversation_id,
        role: parse_role(&m.role),
        content: m.content,
        provider_id: m.provider_id,
        model_id: m.model_id,
        token_count: m.token_count.map(|v| v as u32),
        prompt_tokens: m.prompt_tokens.map(|v| v as u32),
        completion_tokens: m.completion_tokens.map(|v| v as u32),
        attachments: parse_attachment_list(&m.attachments)?,
        thinking: m.thinking,
        created_at: m.created_at,
        parent_message_id: m.parent_message_id,
        version_index: m.version_index,
        is_active: m.is_active == 1,
        tool_calls_json: m.tool_calls_json,
        tool_call_id: m.tool_call_id,
        status: m.status,
        tokens_per_second: m.tokens_per_second,
        first_token_latency_ms: m.first_token_latency_ms,
        parts: m.parts,
        blocks,
    })
}

pub async fn list_messages(db: &DatabaseConnection, conversation_id: &str) -> Result<Vec<Message>> {
    let rows = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::IsActive.eq(1))
        .order_by_asc(messages::Column::CreatedAt)
        .all(db)
        .await?;

    rows.into_iter().map(message_from_entity).collect()
}

pub async fn list_messages_page(
    db: &DatabaseConnection,
    conversation_id: &str,
    limit: u64,
    before_message_id: Option<&str>,
) -> Result<MessagePage> {
    let total_active_count = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::IsActive.eq(1))
        .count(db)
        .await?;

    let mut query = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::IsActive.eq(1));

    if let Some(cursor_id) = before_message_id {
        let cursor = messages::Entity::find_by_id(cursor_id)
            .one(db)
            .await?
            .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", cursor_id)))?;

        query = query.filter(
            Condition::any()
                .add(messages::Column::CreatedAt.lt(cursor.created_at))
                .add(
                    Condition::all()
                        .add(messages::Column::CreatedAt.eq(cursor.created_at))
                        .add(messages::Column::Id.lt(cursor.id.clone())),
                ),
        );
    }

    let mut rows = query
        .order_by_desc(messages::Column::CreatedAt)
        .order_by_desc(messages::Column::Id)
        .limit(limit + 1)
        .all(db)
        .await?;

    let has_older = rows.len() > limit as usize;
    if has_older {
        rows.truncate(limit as usize);
    }
    rows.reverse();

    let messages = rows
        .into_iter()
        .map(message_from_entity)
        .collect::<Result<Vec<_>>>()?;
    let oldest_message_id = messages.first().map(|message| message.id.clone());

    Ok(MessagePage {
        messages,
        has_older,
        oldest_message_id,
        total_active_count,
    })
}

pub async fn create_message(
    db: &DatabaseConnection,
    conversation_id: &str,
    role: MessageRole,
    content: &str,
    attachments: &[Attachment],
    parent_message_id: Option<&str>,
    version_index: i32,
) -> Result<Message> {
    create_message_with_parts(db, conversation_id, role, content, attachments, parent_message_id, version_index, None).await
}

pub async fn create_message_with_parts(
    db: &DatabaseConnection,
    conversation_id: &str,
    role: MessageRole,
    content: &str,
    attachments: &[Attachment],
    parent_message_id: Option<&str>,
    version_index: i32,
    parts: Option<&str>,
) -> Result<Message> {
    let id = gen_id();
    let now = now_ts();
    let role_s = role_str(&role);
    let attachments_json = stringify_attachment_list(attachments)?;

    messages::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(conversation_id.to_string()),
        role: Set(role_s.to_string()),
        content: Set(content.to_string()),
        attachments: Set(attachments_json),
        created_at: Set(now),
        parent_message_id: Set(parent_message_id.map(|s| s.to_string())),
        version_index: Set(version_index),
        is_active: Set(1),
        parts: Set(parts.map(|s| s.to_string())),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let row = messages::Entity::find_by_id(&id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", id)))?;
    message_from_entity(row)
}

pub async fn update_message_content(
    db: &DatabaseConnection,
    id: &str,
    content: &str,
) -> Result<Message> {
    let row = messages::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", id)))?;

    let mut am: messages::ActiveModel = row.into();
    am.content = Set(content.to_string());
    am.update(db).await?;

    let row = messages::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", id)))?;
    message_from_entity(row)
}

/// Update token usage stats on an existing message.
pub async fn update_message_usage(
    db: &DatabaseConnection,
    id: &str,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
) -> Result<()> {
    let row = messages::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", id)))?;

    let mut am: messages::ActiveModel = row.into();
    if let Some(pt) = prompt_tokens {
        am.prompt_tokens = Set(Some(pt));
    }
    if let Some(ct) = completion_tokens {
        am.completion_tokens = Set(Some(ct));
    }
    am.update(db).await?;
    Ok(())
}

/// Efficiently update only the content field of a message for streaming.
///
/// This is optimized for streaming scenarios where we need to update content
/// frequently without the overhead of full entity fetching and conversion.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `id` - Message ID to update
/// * `content` - New content value
///
/// # Returns
///
/// Returns `Ok(())` on success, or error if message not found.
pub async fn update_message_content_fast(
    db: &DatabaseConnection,
    id: &str,
    content: &str,
) -> Result<()> {
    let update = messages::Entity::update_many()
        .col_expr(messages::Column::Content, Expr::value(content.to_string()))
        .filter(messages::Column::Id.eq(id));

    let result = update.exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Message {}", id)));
    }
    Ok(())
}

/// Update thinking content on an existing message.
pub async fn update_message_thinking(
    db: &DatabaseConnection,
    id: &str,
    thinking: Option<&str>,
) -> Result<()> {
    let update = messages::Entity::update_many()
        .col_expr(messages::Column::Thinking, Expr::value(thinking.map(|s| s.to_string())))
        .filter(messages::Column::Id.eq(id));

    let result = update.exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Message {}", id)));
    }
    Ok(())
}

/// Update parts (structured content blocks) on an existing message.
pub async fn update_message_parts(
    db: &DatabaseConnection,
    id: &str,
    parts: Option<&str>,
) -> Result<()> {
    let update = messages::Entity::update_many()
        .col_expr(messages::Column::Parts, Expr::value(parts.map(|s| s.to_string())))
        .filter(messages::Column::Id.eq(id));

    let result = update.exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Message {}", id)));
    }
    Ok(())
}

/// Append content to a message's existing content field.
///
/// This is useful for streaming scenarios where chunks arrive and need to be
/// appended without overwriting existing content.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `id` - Message ID to update
/// * `append_content` - Content to append
///
/// # Returns
///
/// Returns `Ok(())` on success, or error if message not found.
#[allow(clippy::unnecessary_to_owned)]
pub async fn append_message_content(
    db: &DatabaseConnection,
    id: &str,
    append_content: &str,
) -> Result<()> {
    let query = r#"UPDATE messages SET content = content || $1 WHERE id = $2"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        query.to_owned(),
        vec![append_content.into(), id.into()],
    ))
    .await?;

    Ok(())
}

fn compare_version_priority(left: &messages::Model, right: &messages::Model) -> std::cmp::Ordering {
    right
        .version_index
        .cmp(&left.version_index)
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| right.id.cmp(&left.id))
}

fn select_next_active_version(
    deleted: &messages::Model,
    versions: &[messages::Model],
) -> Option<messages::Model> {
    let remaining: Vec<messages::Model> = versions
        .iter()
        .filter(|version| version.id != deleted.id)
        .cloned()
        .collect();
    if remaining.is_empty() {
        return None;
    }

    let same_model: Vec<messages::Model> = if let Some(model_id) = deleted.model_id.as_ref() {
        remaining
            .iter()
            .filter(|version| version.model_id.as_ref() == Some(model_id))
            .cloned()
            .collect()
    } else {
        Vec::new()
    };

    let mut candidates = if same_model.is_empty() {
        remaining
    } else {
        same_model
    };
    candidates.sort_by(compare_version_priority);
    candidates.into_iter().next()
}

pub async fn delete_message(db: &DatabaseConnection, id: &str) -> Result<()> {
    let target = messages::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", id)))?;

    let txn = db.begin().await?;

    if target.role == "assistant" && target.is_active == 1 {
        if let Some(parent_message_id) = target.parent_message_id.as_ref() {
            let sibling_versions = messages::Entity::find()
                .filter(messages::Column::ConversationId.eq(&target.conversation_id))
                .filter(messages::Column::ParentMessageId.eq(parent_message_id))
                .filter(messages::Column::Role.eq("assistant"))
                .filter(messages::Column::VersionIndex.gte(0))
                .all(&txn)
                .await?;

            if let Some(next_version) = select_next_active_version(&target, &sibling_versions) {
                let mut next_active: messages::ActiveModel = next_version.into();
                next_active.is_active = Set(1);
                next_active.update(&txn).await?;
            }
        }
    }

    let result = messages::Entity::delete_by_id(id).exec(&txn).await?;
    txn.commit().await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Message {}", id)));
    }
    Ok(())
}

/// Delete all messages in a conversation.
pub async fn clear_conversation_messages(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<u64> {
    let result = messages::Entity::delete_many()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

/// Delete all messages in a conversation created after the given timestamp (inclusive).
pub async fn delete_messages_after(
    db: &DatabaseConnection,
    conversation_id: &str,
    created_at: i64,
) -> Result<u64> {
    let result = messages::Entity::delete_many()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::CreatedAt.gte(created_at))
        .exec(db)
        .await?;

    Ok(result.rows_affected)
}

pub async fn list_message_versions(
    db: &DatabaseConnection,
    conversation_id: &str,
    parent_message_id: &str,
) -> Result<Vec<Message>> {
    let rows = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::ParentMessageId.eq(parent_message_id))
        .filter(messages::Column::Role.eq("assistant"))
        .filter(messages::Column::VersionIndex.gte(0))
        .order_by_asc(messages::Column::VersionIndex)
        .all(db)
        .await?;

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let candidate_ids: Vec<String> = rows.iter().map(|row| row.id.clone()).collect();
    let tool_rows = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::Role.eq("tool"))
        .filter(messages::Column::ParentMessageId.is_in(candidate_ids.clone()))
        .all(db)
        .await?;

    let tool_parent_ids: HashSet<String> = tool_rows
        .into_iter()
        .filter_map(|row| row.parent_message_id)
        .collect();

    rows.into_iter()
        .filter(|row| !tool_parent_ids.contains(&row.id))
        .map(message_from_entity)
        .collect()
}

pub async fn set_active_version(
    db: &DatabaseConnection,
    conversation_id: &str,
    parent_message_id: &str,
    target_message_id: &str,
) -> Result<()> {
    // Deactivate all versions for this parent
    messages::Entity::update_many()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::ParentMessageId.eq(parent_message_id))
        .col_expr(messages::Column::IsActive, Expr::value(0))
        .exec(db)
        .await?;
    // Activate target version
    let row = messages::Entity::find_by_id(target_message_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Message {}", target_message_id)))?;
    let mut am: messages::ActiveModel = row.into();
    am.is_active = Set(1);
    am.update(db).await?;
    Ok(())
}

pub async fn delete_message_group(db: &DatabaseConnection, user_message_id: &str) -> Result<u64> {
    // Delete all assistant versions for this user message
    let ai_result = messages::Entity::delete_many()
        .filter(messages::Column::ParentMessageId.eq(user_message_id))
        .exec(db)
        .await?;
    // Delete the user message itself
    messages::Entity::delete_by_id(user_message_id)
        .exec(db)
        .await?;
    Ok(ai_result.rows_affected + 1)
}

/// Create a message with role "tool" for storing tool execution results.
pub async fn create_tool_result_message(
    db: &DatabaseConnection,
    conversation_id: &str,
    tool_call_id: &str,
    content: &str,
    parent_message_id: &str,
) -> Result<()> {
    let id = crate::utils::gen_id();
    crate::entity::messages::ActiveModel {
        id: Set(id),
        conversation_id: Set(conversation_id.to_string()),
        role: Set("tool".to_string()),
        content: Set(content.to_string()),
        provider_id: Set(None),
        model_id: Set(None),
        token_count: Set(None),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        attachments: Set("[]".to_string()),
        thinking: Set(None),
        created_at: Set(crate::utils::now_ts()),
        branch_id: Set(None),
        parent_message_id: Set(Some(parent_message_id.to_string())),
        // Tool-result scaffolding: excluded from history reload (paired with intermediate assistant).
        version_index: Set(-1),
        is_active: Set(0),
        tool_calls_json: Set(None),
        tool_call_id: Set(Some(tool_call_id.to_string())),
        status: Set("complete".to_string()),
        tokens_per_second: Set(None),
        first_token_latency_ms: Set(None),
        parts: Set(None),
    }
    .insert(db)
    .await?;
    Ok(())
}

/// Create an assistant message that contains tool_calls (intermediate message in tool loop).
pub async fn create_assistant_tool_call_message(
    db: &DatabaseConnection,
    conversation_id: &str,
    content: &str,
    tool_calls_json: Option<&str>,
    provider_id: &str,
    model_id: &str,
    parent_message_id: &str,
) -> Result<String> {
    let id = crate::utils::gen_id();
    crate::entity::messages::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(conversation_id.to_string()),
        role: Set("assistant".to_string()),
        content: Set(content.to_string()),
        provider_id: Set(Some(provider_id.to_string())),
        model_id: Set(Some(model_id.to_string())),
        token_count: Set(None),
        prompt_tokens: Set(None),
        completion_tokens: Set(None),
        attachments: Set("[]".to_string()),
        thinking: Set(None),
        created_at: Set(crate::utils::now_ts()),
        branch_id: Set(None),
        parent_message_id: Set(Some(parent_message_id.to_string())),
        // Intermediate tool-call scaffolding message. Excluded from user-visible AI version pagination.
        version_index: Set(-1),
        is_active: Set(0),
        tool_calls_json: Set(tool_calls_json.map(|s| s.to_string())),
        tool_call_id: Set(None),
        status: Set("complete".to_string()),
        tokens_per_second: Set(None),
        first_token_latency_ms: Set(None),
        parts: Set(None),
    }
    .insert(db)
    .await?;
    Ok(id)
}

pub async fn get_conversation_stats(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<ConversationStats> {
    use sea_orm::{FromQueryResult, Statement};

    #[derive(Debug, FromQueryResult)]
    struct StatsRow {
        total_messages: i64,
        total_user_messages: i64,
        total_assistant_messages: i64,
        total_prompt_tokens: i64,
        total_completion_tokens: i64,
        avg_tokens_per_second: Option<f64>,
        avg_first_token_latency_ms: Option<f64>,
        avg_response_time_ms: Option<f64>,
    }

    let sql = r#"
        SELECT
            COUNT(*) AS total_messages,
            SUM(CASE WHEN role = 'user' THEN 1 ELSE 0 END) AS total_user_messages,
            SUM(CASE WHEN role = 'assistant' THEN 1 ELSE 0 END) AS total_assistant_messages,
            COALESCE(SUM(prompt_tokens), 0) AS total_prompt_tokens,
            COALESCE(SUM(completion_tokens), 0) AS total_completion_tokens,
            AVG(CASE WHEN tokens_per_second IS NOT NULL AND tokens_per_second > 0 THEN tokens_per_second ELSE NULL END) AS avg_tokens_per_second,
            AVG(CASE WHEN first_token_latency_ms IS NOT NULL THEN first_token_latency_ms ELSE NULL END) AS avg_first_token_latency_ms,
            AVG(CASE
                WHEN first_token_latency_ms IS NOT NULL AND tokens_per_second IS NOT NULL AND tokens_per_second > 0 AND completion_tokens IS NOT NULL AND completion_tokens > 0
                THEN first_token_latency_ms + (completion_tokens * 1000.0 / tokens_per_second)
                ELSE NULL
            END) AS avg_response_time_ms
        FROM messages
        WHERE conversation_id = ? AND is_active = 1
    "#;

    let row = StatsRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        sql,
        vec![conversation_id.into()],
    ))
    .one(db)
    .await?
    .unwrap_or(StatsRow {
        total_messages: 0,
        total_user_messages: 0,
        total_assistant_messages: 0,
        total_prompt_tokens: 0,
        total_completion_tokens: 0,
        avg_tokens_per_second: None,
        avg_first_token_latency_ms: None,
        avg_response_time_ms: None,
    });

    let total_prompt = row.total_prompt_tokens as u64;
    let total_completion = row.total_completion_tokens as u64;

    Ok(ConversationStats {
        total_messages: row.total_messages as u64,
        total_user_messages: row.total_user_messages as u64,
        total_assistant_messages: row.total_assistant_messages as u64,
        total_prompt_tokens: total_prompt,
        total_completion_tokens: total_completion,
        total_tokens: total_prompt + total_completion,
        avg_tokens_per_second: row.avg_tokens_per_second,
        avg_first_token_latency_ms: row.avg_first_token_latency_ms,
        avg_response_time_ms: row.avg_response_time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_test_pool;
    use crate::repo::conversation;

    #[tokio::test]
    async fn create_message_round_trips_attachment_metadata() {
        let h = create_test_pool().await.unwrap();
        let db = &h.conn;

        let conv = conversation::create_conversation(db, "Attach Chat", "m1", "p1", None)
            .await
            .unwrap();

        let msg = create_message(
            db,
            &conv.id,
            MessageRole::User,
            "See attached",
            &[Attachment {
                id: "att-1".into(),
                file_name: "image.png".into(),
                file_type: "image/png".into(),
                file_path: "conv-1/image.png".into(),
                file_size: 3,
                data: None,
            }],
            None,
            0,
        )
        .await
        .unwrap();

        assert_eq!(msg.attachments.len(), 1);
        assert_eq!(msg.attachments[0].file_name, "image.png");
        assert_eq!(msg.attachments[0].file_type, "image/png");
        assert_eq!(msg.attachments[0].file_path, "conv-1/image.png");
        assert_eq!(msg.attachments[0].file_size, 3);
    }
}
