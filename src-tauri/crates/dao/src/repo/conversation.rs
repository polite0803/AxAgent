// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;
use serde_json;

use axagent_entities::{
    agent_sessions, conversation_summaries, conversations, knowledge_attributes,
    knowledge_documents, knowledge_entities, knowledge_flows, knowledge_relations, messages,
};
use axagent_harness::constants;
use axagent_harness::core_error::{AxAgentError, Result};
use axagent_harness::types::{
    Conversation, ConversationSearchResult, ConversationSummary, KnowledgeDocument,
    UpdateConversationInput,
};
use axagent_harness::util_fns::{gen_id, now_ts};

pub fn conversation_from_entity(m: conversations::Model) -> Conversation {
    Conversation {
        id: m.id,
        title: m.title,
        model_id: m.model_id,
        provider_id: m.provider_id,
        system_prompt: m.system_prompt,
        temperature: m.temperature.map(|v| v as f32),
        max_tokens: m.max_tokens.map(|v| v as u32),
        top_p: m.top_p.map(|v| v as f32),
        frequency_penalty: m.frequency_penalty.map(|v| v as f32),
        search_enabled: m.search_enabled != 0,
        search_provider_id: m.search_provider_id,
        thinking_budget: m.thinking_budget,
        enabled_mcp_server_ids: parse_string_list(&m.enabled_mcp_server_ids),
        enabled_knowledge_base_ids: parse_string_list(&m.enabled_knowledge_base_ids),
        enabled_memory_namespace_ids: parse_string_list(&m.enabled_memory_namespace_ids),
        enabled_wiki_ids: parse_string_list(&m.enabled_wiki_ids),
        message_count: m.message_count as u32,
        is_pinned: m.is_pinned != 0,
        is_archived: m.is_archived != 0,
        context_compression: m.context_compression != 0,
        category_id: m.category_id,
        parent_conversation_id: m.parent_conversation_id,
        mode: m.mode,
        work_strategy: m.work_strategy,
        scenario: m.scenario,
        workspace_dir: None,
        enabled_skill_ids: parse_string_list(&m.enabled_skill_ids),
        agent_profile_id: m.agent_profile_id,
        workflow_template_id: m.workflow_template_id,
        session_type: m.session_type,
        workflow_status: m.workflow_status,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn parse_string_list(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_else(|e| {
        tracing::warn!("会话偏好 JSON 损坏: {e}");
        Vec::new()
    })
}

fn stringify_string_list(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|e| {
        tracing::warn!("会话偏好序列化失败: {e}");
        "[]".to_string()
    })
}

async fn attach_workspace_dirs(
    db: &DatabaseConnection,
    mut convs: Vec<Conversation>,
) -> Result<Vec<Conversation>> {
    if convs.is_empty() {
        return Ok(convs);
    }
    let ids: Vec<String> = convs.iter().map(|c| c.id.clone()).collect();
    let sessions = agent_sessions::Entity::find()
        .filter(agent_sessions::Column::ConversationId.is_in(ids))
        .all(db)
        .await?;
    let cwd_map: std::collections::HashMap<String, Option<String>> = sessions
        .into_iter()
        .map(|s| (s.conversation_id, s.cwd))
        .collect();
    for conv in &mut convs {
        conv.workspace_dir = cwd_map.get(&conv.id).cloned().flatten();
    }
    Ok(convs)
}

pub async fn list_conversations(db: &DatabaseConnection) -> Result<Vec<Conversation>> {
    let rows = conversations::Entity::find()
        .filter(conversations::Column::IsArchived.eq(0))
        .order_by_desc(conversations::Column::IsPinned)
        .order_by_desc(conversations::Column::UpdatedAt)
        .all(db)
        .await?;

    let convs: Vec<Conversation> = rows.into_iter().map(conversation_from_entity).collect();
    attach_workspace_dirs(db, convs).await
}

pub async fn list_archived_conversations(db: &DatabaseConnection) -> Result<Vec<Conversation>> {
    let rows = conversations::Entity::find()
        .filter(conversations::Column::IsArchived.ne(0))
        .order_by_desc(conversations::Column::UpdatedAt)
        .all(db)
        .await?;

    let convs: Vec<Conversation> = rows.into_iter().map(conversation_from_entity).collect();
    attach_workspace_dirs(db, convs).await
}

pub async fn get_conversation(db: &DatabaseConnection, id: &str) -> Result<Conversation> {
    let row = conversations::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", id)))?;

    Ok(conversation_from_entity(row))
}

pub async fn create_conversation(
    db: &DatabaseConnection,
    title: &str,
    model_id: &str,
    provider_id: &str,
    system_prompt: Option<&str>,
) -> Result<Conversation> {
    let id = gen_id();
    let now = now_ts();

    conversations::ActiveModel {
        id: Set(id.clone()),
        title: Set(title.to_string()),
        model_id: Set(model_id.to_string()),
        provider_id: Set(provider_id.to_string()),
        system_prompt: Set(system_prompt.map(|s| s.to_string())),
        message_count: Set(0),
        is_pinned: Set(0),
        enabled_skill_ids: Set("[]".to_string()),
        enabled_wiki_ids: Set("[]".to_string()),
        mode: Set("agent".to_string()),
        session_type: Set("conversation".to_string()),
        memory_status: Set("idle".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    get_conversation(db, &id).await
}

pub async fn update_conversation(
    db: &DatabaseConnection,
    id: &str,
    input: UpdateConversationInput,
) -> Result<Conversation> {
    let row = conversations::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", id)))?;

    let now = now_ts();
    let existing = conversation_from_entity(row.clone());

    let title = input.title.unwrap_or(existing.title);
    let provider_id = input.provider_id.unwrap_or(existing.provider_id);
    let model_id = input.model_id.unwrap_or(existing.model_id);
    let is_pinned = input.is_pinned.unwrap_or(existing.is_pinned);
    let is_archived = input.is_archived.unwrap_or(existing.is_archived);

    let mut am: conversations::ActiveModel = row.into();
    am.title = Set(title);
    am.provider_id = Set(provider_id);
    am.model_id = Set(model_id);
    am.is_pinned = Set(if is_pinned { 1 } else { 0 });
    am.is_archived = Set(if is_archived { 1 } else { 0 });
    if let Some(ref sp) = input.system_prompt {
        am.system_prompt = Set(if sp.is_empty() {
            None
        } else {
            Some(sp.clone())
        });
    }
    if let Some(temperature) = input.temperature {
        am.temperature = Set(temperature);
    }
    if let Some(max_tokens) = input.max_tokens {
        am.max_tokens = Set(max_tokens);
    }
    if let Some(top_p) = input.top_p {
        am.top_p = Set(top_p);
    }
    if let Some(frequency_penalty) = input.frequency_penalty {
        am.frequency_penalty = Set(frequency_penalty);
    }
    if let Some(search_enabled) = input.search_enabled {
        am.search_enabled = Set(if search_enabled { 1 } else { 0 });
    }
    if let Some(search_provider_id) = input.search_provider_id {
        am.search_provider_id = Set(search_provider_id);
    }
    if let Some(thinking_budget) = input.thinking_budget {
        am.thinking_budget = Set(thinking_budget);
    }
    if let Some(enabled_mcp_server_ids) = input.enabled_mcp_server_ids {
        am.enabled_mcp_server_ids = Set(stringify_string_list(&enabled_mcp_server_ids));
    }
    if let Some(enabled_knowledge_base_ids) = input.enabled_knowledge_base_ids {
        am.enabled_knowledge_base_ids = Set(stringify_string_list(&enabled_knowledge_base_ids));
    }
    if let Some(enabled_memory_namespace_ids) = input.enabled_memory_namespace_ids {
        am.enabled_memory_namespace_ids = Set(stringify_string_list(&enabled_memory_namespace_ids));
    }
    if let Some(context_compression) = input.context_compression {
        am.context_compression = Set(if context_compression { 1 } else { 0 });
    }
    if let Some(category_id) = input.category_id {
        am.category_id = Set(category_id);
    }
    if let Some(parent_conversation_id) = input.parent_conversation_id {
        am.parent_conversation_id = Set(parent_conversation_id);
    }
    if let Some(mode) = input.mode {
        am.mode = Set(mode);
    }
    if let Some(work_strategy) = input.work_strategy {
        am.work_strategy = Set(work_strategy);
    }
    if let Some(scenario) = input.scenario {
        am.scenario = Set(Some(scenario));
    }
    if let Some(enabled_skill_ids) = input.enabled_skill_ids {
        am.enabled_skill_ids = Set(stringify_string_list(&enabled_skill_ids));
    }
    if let Some(enabled_wiki_ids) = input.enabled_wiki_ids {
        am.enabled_wiki_ids = Set(stringify_string_list(&enabled_wiki_ids));
    }
    if let Some(agent_profile_id) = input.agent_profile_id {
        am.agent_profile_id = Set(agent_profile_id);
    }
    if let Some(workflow_template_id) = input.workflow_template_id {
        am.workflow_template_id = Set(workflow_template_id);
    }
    if let Some(session_type) = input.session_type {
        am.session_type = Set(session_type);
    }
    if let Some(workflow_status) = input.workflow_status {
        am.workflow_status = Set(workflow_status);
    }
    am.updated_at = Set(now);
    am.update(db).await?;

    get_conversation(db, id).await
}

pub async fn update_conversation_title(
    db: &DatabaseConnection,
    id: &str,
    title: &str,
) -> Result<()> {
    if let Some(row) = conversations::Entity::find_by_id(id).one(db).await? {
        let mut am: conversations::ActiveModel = row.into();
        am.title = Set(title.to_string());
        am.updated_at = Set(now_ts());
        am.update(db).await?;
    }
    Ok(())
}

pub async fn toggle_pin(db: &DatabaseConnection, id: &str) -> Result<Conversation> {
    let row = conversations::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", id)))?;

    let new_pinned = if row.is_pinned != 0 { 0 } else { 1 };
    let now = now_ts();

    let mut am: conversations::ActiveModel = row.into();
    am.is_pinned = Set(new_pinned);
    am.updated_at = Set(now);
    am.update(db).await?;

    get_conversation(db, id).await
}

pub async fn toggle_archive(db: &DatabaseConnection, id: &str) -> Result<Conversation> {
    let row = conversations::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", id)))?;

    let new_archived = if row.is_archived != 0 { 0 } else { 1 };
    let now = now_ts();

    let mut am: conversations::ActiveModel = row.into();
    am.is_archived = Set(new_archived);
    am.updated_at = Set(now);
    am.update(db).await?;

    get_conversation(db, id).await
}

/// Archive a conversation to a knowledge base.
///
/// This extracts all user/assistant messages from the conversation, formats them
/// into a structured text document, creates a knowledge document record (with
/// `doc_type = "conversation"`), and marks the conversation as archived.
pub async fn archive_to_knowledge_base(
    db: &DatabaseConnection,
    conversation_id: &str,
    knowledge_base_id: &str,
) -> Result<(Conversation, KnowledgeDocument)> {
    let conv = conversations::Entity::find_by_id(conversation_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", conversation_id)))?;

    if conv.is_archived != 0 {
        return Err(AxAgentError::Validation(format!(
            "Conversation {} is already archived",
            conversation_id
        )));
    }

    let existing_doc = knowledge_documents::Entity::find()
        .filter(knowledge_documents::Column::KnowledgeBaseId.eq(knowledge_base_id))
        .filter(knowledge_documents::Column::SourceConversationId.eq(conversation_id))
        .one(db)
        .await?;
    if existing_doc.is_some() {
        return Err(AxAgentError::Validation(format!(
            "Conversation {} already archived to knowledge base {}",
            conversation_id, knowledge_base_id
        )));
    }

    let conv_title = conv.title.clone();
    let conv_mode = conv.mode.clone();
    let conv_session_type = conv.session_type.clone();
    let conv_provider_id = conv.provider_id.clone();
    let conv_model_id = conv.model_id.clone();
    let conv_message_count = conv.message_count;
    let conv_created_at = conv.created_at;

    let all_msgs = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::IsActive.eq(1))
        .order_by_asc(messages::Column::CreatedAt)
        .all(db)
        .await?;

    let all_msgs_with_scaffold = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .order_by_asc(messages::Column::CreatedAt)
        .all(db)
        .await?;

    let mut text_parts: Vec<String> = Vec::new();
    text_parts.push(format!("# {}\n", conv_title));

    let mut flow_steps: Vec<serde_json::Value> = Vec::new();
    let mut step_index: u32 = 0;
    let mut first_user_content: Option<String> = None;

    for msg in &all_msgs {
        let role_label = match msg.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "system" => continue,
            _ => continue,
        };
        let content = if msg.content.len() > 8000 {
            format!("{}...(truncated)", &msg.content[..8000])
        } else {
            msg.content.clone()
        };
        text_parts.push(format!("## {}\n\n{}", role_label, content));

        if first_user_content.is_none() && msg.role == "user" {
            first_user_content = Some(if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            });
        }

        let preview = if msg.content.len() > 100 {
            format!("{}...", &msg.content[..100])
        } else {
            msg.content.clone()
        };
        flow_steps.push(serde_json::json!({
            "index": step_index,
            "role": msg.role,
            "contentPreview": preview,
        }));
        step_index += 1;
    }

    let document_content = text_parts.join("\n\n");
    let content_bytes = document_content.len() as i64;

    let doc_id = gen_id();
    let now = now_ts();

    let doc_am = knowledge_documents::ActiveModel {
        id: Set(doc_id.clone()),
        knowledge_base_id: Set(knowledge_base_id.to_string()),
        title: Set(format!("[Archive] {}", conv_title)),
        source_path: Set(format!("conversation://{}", conversation_id)),
        mime_type: Set("text/markdown".to_string()),
        size_bytes: Set(content_bytes),
        indexing_status: Set(constants::status::PENDING.to_string()),
        doc_type: Set("conversation".to_string()),
        index_error: Set(None),
        source_conversation_id: Set(Some(conversation_id.to_string())),
        created_at: Set(now),
        updated_at: Set(now),
    };
    doc_am.insert(db).await?;

    let entity_id = gen_id();
    let entity_props = serde_json::json!({
        "mode": conv_mode,
        "sessionType": conv_session_type,
        "messageCount": conv_message_count,
        "providerId": conv_provider_id,
        "modelId": conv_model_id,
        "createdAt": conv_created_at,
    });

    let entity_am = knowledge_entities::ActiveModel {
        id: Set(entity_id.clone()),
        knowledge_base_id: Set(knowledge_base_id.to_string()),
        name: Set(conv_title.clone()),
        entity_type: Set("conversation".to_string()),
        description: Set(first_user_content.clone()),
        source_path: Set(format!("conversation://{}", conversation_id)),
        source_language: Set(None),
        properties: Set(entity_props),
        lifecycle: Set(None),
        behaviors: Set(None),
        metadata: Set(Some(serde_json::json!({
            "archivedAt": now,
            "documentId": doc_id,
        }))),
        created_at: Set(now),
        updated_at: Set(now),
    };
    entity_am.insert(db).await?;

    let attr_defs = [
        ("title", "string", "Conversation title"),
        ("mode", "string", "Conversation mode (chat/agent/gateway)"),
        ("sessionType", "string", "Session type (conversation/workflow)"),
        ("messageCount", "integer", "Number of messages"),
        ("providerId", "string", "LLM provider ID"),
        ("modelId", "string", "LLM model ID"),
    ];

    for (name, data_type, desc) in &attr_defs {
        let attr_id = gen_id();
        let attr_am = knowledge_attributes::ActiveModel {
            id: Set(attr_id),
            knowledge_base_id: Set(knowledge_base_id.to_string()),
            entity_id: Set(entity_id.clone()),
            name: Set((*name).to_owned()),
            attribute_type: Set("property".to_string()),
            data_type: Set((*data_type).to_owned()),
            description: Set(Some((*desc).to_owned())),
            is_required: Set(false),
            default_value: Set(None),
            constraints: Set(None),
            validation_rules: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        attr_am.insert(db).await?;
    }

    if !flow_steps.is_empty() {
        let flow_id = gen_id();
        let flow_am = knowledge_flows::ActiveModel {
            id: Set(flow_id),
            knowledge_base_id: Set(knowledge_base_id.to_string()),
            name: Set(format!("{} - 对话流程", conv_title)),
            flow_type: Set("conversation".to_string()),
            description: Set(first_user_content.clone()),
            source_path: Set(format!("conversation://{}", conversation_id)),
            steps: Set(serde_json::json!(flow_steps)),
            decision_points: Set(None),
            error_handling: Set(None),
            preconditions: Set(None),
            postconditions: Set(None),
            metadata: Set(Some(serde_json::json!({
                "messageCount": conv_message_count,
                "mode": conv_mode,
                "entityId": entity_id,
            }))),
            created_at: Set(now),
            updated_at: Set(now),
        };
        flow_am.insert(db).await?;
    }

    struct Turn {
        user_msg: messages::Model,
        assistant_msgs: Vec<TurnAssistantMsg>,
        tool_msgs: Vec<messages::Model>,
    }

    struct TurnAssistantMsg {
        msg: messages::Model,
        tool_calls_json: Option<String>,
    }

    let mut turns: Vec<Turn> = Vec::new();
    let mut current_turn: Option<Turn> = None;

    for msg in &all_msgs_with_scaffold {
        match msg.role.as_str() {
            "user" => {
                if let Some(prev) = current_turn.take() {
                    turns.push(prev);
                }
                current_turn = Some(Turn {
                    user_msg: msg.clone(),
                    assistant_msgs: Vec::new(),
                    tool_msgs: Vec::new(),
                });
            },
            "assistant" => {
                if let Some(ref mut t) = current_turn {
                    let tc_json = msg.tool_calls_json.clone();
                    t.assistant_msgs.push(TurnAssistantMsg {
                        msg: msg.clone(),
                        tool_calls_json: tc_json,
                    });
                }
            },
            "tool" => {
                if let Some(ref mut t) = current_turn {
                    t.tool_msgs.push(msg.clone());
                }
            },
            _ => {},
        }
    }
    if let Some(prev) = current_turn.take() {
        turns.push(prev);
    }

    let mut prev_qa_entity_id: Option<String> = None;

    for (turn_idx, turn) in turns.iter().enumerate() {
        let q_preview = if turn.user_msg.content.len() > 100 {
            format!("{}...", &turn.user_msg.content[..100])
        } else {
            turn.user_msg.content.clone()
        };

        let final_answer = turn
            .assistant_msgs
            .iter()
            .rfind(|a| a.tool_calls_json.is_none() && !a.msg.content.trim().is_empty())
            .map(|a| a.msg.content.clone())
            .unwrap_or_else(|| {
                turn.assistant_msgs
                    .last()
                    .map(|a| a.msg.content.clone())
                    .unwrap_or_default()
            });

        let a_preview = if final_answer.len() > 200 {
            format!("{}...", &final_answer[..200])
        } else {
            final_answer.clone()
        };

        let tool_call_summaries: Vec<serde_json::Value> = turn
            .assistant_msgs
            .iter()
            .filter(|a| a.tool_calls_json.is_some())
            .filter_map(|a| {
                let tc: Vec<serde_json::Value> =
                    serde_json::from_str(a.tool_calls_json.as_ref()?).ok()?;
                Some(serde_json::json!({
                    "toolCalls": tc,
                    "contentPreview": if a.msg.content.len() > 100 {
                        format!("{}...", &a.msg.content[..100])
                    } else {
                        a.msg.content.clone()
                    },
                }))
            })
            .collect();

        let tool_result_summaries: Vec<serde_json::Value> = turn
            .tool_msgs
            .iter()
            .map(|m| {
                let preview = if m.content.len() > 200 {
                    format!("{}...", &m.content[..200])
                } else {
                    m.content.clone()
                };
                serde_json::json!({
                    "toolCallId": m.tool_call_id,
                    "contentPreview": preview,
                })
            })
            .collect();

        let qa_entity_id = gen_id();
        let mut qa_props = serde_json::json!({
            "question": q_preview,
            "answer": a_preview,
            "turnIndex": turn_idx,
        });
        if !tool_call_summaries.is_empty() {
            qa_props["toolCalls"] = serde_json::json!(tool_call_summaries);
        }
        if !tool_result_summaries.is_empty() {
            qa_props["toolResults"] = serde_json::json!(tool_result_summaries);
        }

        let qa_entity_am = knowledge_entities::ActiveModel {
            id: Set(qa_entity_id.clone()),
            knowledge_base_id: Set(knowledge_base_id.to_string()),
            name: Set(q_preview.clone()),
            entity_type: Set("qa_pair".to_string()),
            description: Set(Some(a_preview.clone())),
            source_path: Set(format!("conversation://{}", conversation_id)),
            source_language: Set(None),
            properties: Set(qa_props),
            lifecycle: Set(None),
            behaviors: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        qa_entity_am.insert(db).await?;

        let rel_id = gen_id();
        let rel_am = knowledge_relations::ActiveModel {
            id: Set(rel_id),
            knowledge_base_id: Set(knowledge_base_id.to_string()),
            source_entity_id: Set(entity_id.clone()),
            target_entity_id: Set(qa_entity_id.clone()),
            relation_type: Set("contains".to_string()),
            description: Set(Some(format!("Q&A pair #{}", turn_idx + 1))),
            properties: Set(None),
            metadata: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        rel_am.insert(db).await?;

        if let Some(prev_id) = &prev_qa_entity_id {
            let seq_rel_id = gen_id();
            let seq_rel_am = knowledge_relations::ActiveModel {
                id: Set(seq_rel_id),
                knowledge_base_id: Set(knowledge_base_id.to_string()),
                source_entity_id: Set(prev_id.clone()),
                target_entity_id: Set(qa_entity_id.clone()),
                relation_type: Set("follows".to_string()),
                description: Set(None),
                properties: Set(None),
                metadata: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            seq_rel_am.insert(db).await?;
        }
        prev_qa_entity_id = Some(qa_entity_id);
    }

    let new_archived = 1;
    let mut am: conversations::ActiveModel = conv.into();
    am.is_archived = Set(new_archived);
    let new_memory_status = match am.memory_status.as_ref() {
        s if s == "extracted" => "both",
        _ => "archived",
    };
    am.memory_status = Set(new_memory_status.to_string());
    am.updated_at = Set(now);
    am.update(db).await?;

    let updated_conv = get_conversation(db, conversation_id).await?;

    let doc_model = knowledge_documents::Entity::find_by_id(&doc_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("KnowledgeDocument {}", doc_id)))?;

    let doc = KnowledgeDocument {
        id: doc_model.id,
        knowledge_base_id: doc_model.knowledge_base_id,
        title: doc_model.title,
        source_path: doc_model.source_path,
        mime_type: doc_model.mime_type,
        size_bytes: doc_model.size_bytes,
        indexing_status: doc_model.indexing_status,
        doc_type: doc_model.doc_type,
        index_error: doc_model.index_error,
        source_conversation_id: doc_model.source_conversation_id,
    };

    Ok((updated_conv, doc))
}

/// Get the extracted text content for a conversation-archive document.
/// Used by the indexing pipeline to obtain the text for embedding.
pub async fn get_conversation_archive_text(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<String> {
    let conv = conversations::Entity::find_by_id(conversation_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", conversation_id)))?;

    let conv_title = conv.title.clone();

    let all_msgs = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .order_by_asc(messages::Column::CreatedAt)
        .all(db)
        .await?;

    let mut text_parts: Vec<String> = Vec::new();
    text_parts.push(format!("# {}\n", conv_title));

    for msg in &all_msgs {
        let role_label = match msg.role.as_str() {
            "user" => "User",
            "assistant" => "Assistant",
            "tool" => "Tool Result",
            "system" => continue,
            _ => continue,
        };
        let content = if msg.content.len() > 8000 {
            format!("{}...(truncated)", &msg.content[..8000])
        } else {
            msg.content.clone()
        };
        if msg.role == "assistant" {
            if let Some(ref tc_json) = msg.tool_calls_json {
                text_parts.push(format!(
                    "## {} (Tool Call)\n\n{}\n\nTool Calls: {}",
                    role_label, content, tc_json
                ));
            } else {
                text_parts.push(format!("## {}\n\n{}", role_label, content));
            }
        } else {
            text_parts.push(format!("## {}\n\n{}", role_label, content));
        }
    }

    Ok(text_parts.join("\n\n"))
}

pub async fn delete_conversation(db: &DatabaseConnection, id: &str) -> Result<()> {
    let result = conversations::Entity::delete_by_id(id).exec(db).await?;

    if result.rows_affected == 0 {
        return Err(AxAgentError::NotFound(format!("Conversation {}", id)));
    }
    Ok(())
}

/// Branch a conversation: copy settings + messages up to `until_message_id`.
/// If `as_child` is true, the new conversation is nested under the source (or its parent).
pub async fn branch_conversation(
    db: &DatabaseConnection,
    conversation_id: &str,
    until_message_id: &str,
    as_child: bool,
    custom_title: Option<&str>,
) -> Result<Conversation> {
    // 1. Load source conversation
    let source = conversations::Entity::find_by_id(conversation_id)
        .one(db)
        .await?
        .ok_or_else(|| AxAgentError::NotFound(format!("Conversation {}", conversation_id)))?;

    // 2. Load all active messages ordered by created_at
    let all_msgs = messages::Entity::find()
        .filter(messages::Column::ConversationId.eq(conversation_id))
        .filter(messages::Column::IsActive.eq(1))
        .order_by_asc(messages::Column::CreatedAt)
        .all(db)
        .await?;

    // 3. Find the target message index
    let target_idx = all_msgs
        .iter()
        .position(|m| m.id == until_message_id)
        .ok_or_else(|| {
            AxAgentError::NotFound(format!("Message {} in conversation", until_message_id))
        })?;

    // 4. Slice messages up to (and including) the target
    let candidate_msgs = &all_msgs[..=target_idx];

    // 5. Find last context-clear marker to determine effective start
    let start_idx = candidate_msgs
        .iter()
        .rposition(|m| {
            m.role == "system"
                && (m.content == "<!-- context-clear -->"
                    || m.content == "<!-- context-compressed -->")
        })
        .map(|idx| idx + 1) // skip the marker itself
        .unwrap_or(0);

    let effective_msgs = &candidate_msgs[start_idx..];

    // 6. Create new conversation with copied settings
    let new_id = gen_id();
    let now = now_ts();
    let branch_title = custom_title
        .map(|t| t.to_string())
        .unwrap_or_else(|| source.title.clone());

    // Determine parent_conversation_id
    let parent_id = if as_child {
        // If source already has a parent, new branch is a sibling (same parent)
        // Otherwise, source becomes the parent
        Some(
            source
                .parent_conversation_id
                .clone()
                .unwrap_or_else(|| source.id.clone()),
        )
    } else {
        None
    };

    conversations::ActiveModel {
        id: Set(new_id.clone()),
        title: Set(branch_title),
        model_id: Set(source.model_id.clone()),
        provider_id: Set(source.provider_id.clone()),
        system_prompt: Set(source.system_prompt.clone()),
        temperature: Set(source.temperature),
        max_tokens: Set(source.max_tokens),
        top_p: Set(source.top_p),
        frequency_penalty: Set(source.frequency_penalty),
        search_enabled: Set(source.search_enabled),
        search_provider_id: Set(source.search_provider_id.clone()),
        thinking_budget: Set(source.thinking_budget),
        enabled_mcp_server_ids: Set(source.enabled_mcp_server_ids.clone()),
        enabled_knowledge_base_ids: Set(source.enabled_knowledge_base_ids.clone()),
        enabled_memory_namespace_ids: Set(source.enabled_memory_namespace_ids.clone()),
        enabled_wiki_ids: Set(source.enabled_wiki_ids.clone()),
        message_count: Set(effective_msgs.len() as i32),
        is_pinned: Set(0),
        is_archived: Set(0),
        context_compression: Set(source.context_compression),
        category_id: Set(source.category_id.clone()),
        parent_conversation_id: Set(parent_id),
        research_mode: Set(source.research_mode),
        enabled_skill_ids: Set(source.enabled_skill_ids.clone()),
        agent_profile_id: Set(source.agent_profile_id.clone()),
        workflow_template_id: Set(source.workflow_template_id.clone()),
        session_type: Set(source.session_type.clone()),
        workflow_status: Set(source.workflow_status.clone()),
        mode: Set(source.mode.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // 7. Copy messages — assign new IDs and remap parent_message_id references
    let mut id_map = std::collections::HashMap::new();
    for msg in effective_msgs {
        let new_msg_id = gen_id();
        id_map.insert(msg.id.clone(), new_msg_id.clone());

        let new_parent = msg
            .parent_message_id
            .as_ref()
            .and_then(|pid| id_map.get(pid))
            .cloned();

        messages::ActiveModel {
            id: Set(new_msg_id),
            conversation_id: Set(new_id.clone()),
            role: Set(msg.role.clone()),
            content: Set(msg.content.clone()),
            provider_id: Set(msg.provider_id.clone()),
            model_id: Set(msg.model_id.clone()),
            token_count: Set(msg.token_count),
            prompt_tokens: Set(msg.prompt_tokens),
            completion_tokens: Set(msg.completion_tokens),
            attachments: Set(msg.attachments.clone()),
            thinking: Set(msg.thinking.clone()),
            created_at: Set(msg.created_at),
            parent_message_id: Set(new_parent),
            version_index: Set(msg.version_index),
            is_active: Set(1),
            tool_calls_json: Set(msg.tool_calls_json.clone()),
            tool_call_id: Set(msg.tool_call_id.clone()),
            status: Set(msg.status.clone()),
            tokens_per_second: Set(msg.tokens_per_second),
            first_token_latency_ms: Set(msg.first_token_latency_ms),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    get_conversation(db, &new_id).await
}

pub async fn search_conversations(
    db: &DatabaseConnection,
    query: &str,
) -> Result<Vec<ConversationSearchResult>> {
    #[derive(Debug, FromQueryResult)]
    struct FtsRow {
        conversation_id: String,
        preview: String,
    }

    let fts_rows = FtsRow::find_by_statement(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "SELECT m.conversation_id, snippet(messages_fts, 0, '', '', '...', 32) as preview \
         FROM messages_fts \
         JOIN messages m ON m.rowid = messages_fts.rowid \
         WHERE messages_fts MATCH ? \
         GROUP BY m.conversation_id \
         ORDER BY rank",
        [query.into()],
    ))
    .all(db)
    .await?;

    let mut results = Vec::with_capacity(fts_rows.len());
    for fts in fts_rows {
        if let Ok(conv) = get_conversation(db, &fts.conversation_id).await {
            results.push(ConversationSearchResult {
                conversation: conv,
                matched_message_preview: Some(fts.preview),
            });
        }
    }
    Ok(results)
}

pub async fn increment_message_count(db: &DatabaseConnection, conversation_id: &str) -> Result<()> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE conversations SET message_count = message_count + 1, updated_at = ? WHERE id = ?",
        vec![now_ts().into(), conversation_id.into()],
    ))
    .await?;
    Ok(())
}

pub async fn decrement_message_count(db: &DatabaseConnection, conversation_id: &str) -> Result<()> {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Sqlite,
        "UPDATE conversations SET message_count = MAX(0, message_count - 1), updated_at = ? WHERE id = ?",
        vec![now_ts().into(), conversation_id.into()],
    ))
    .await?;
    Ok(())
}

// ── Conversation summaries ──────────────────────────────────────────────

fn summary_from_entity(m: conversation_summaries::Model) -> ConversationSummary {
    ConversationSummary {
        id: m.id,
        conversation_id: m.conversation_id,
        summary_text: m.summary_text,
        compressed_until_message_id: m.compressed_until_message_id,
        token_count: m.token_count.map(|v| v as u32),
        model_used: m.model_used,
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

pub async fn get_summary(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<Option<ConversationSummary>> {
    let row = conversation_summaries::Entity::find()
        .filter(conversation_summaries::Column::ConversationId.eq(conversation_id))
        .order_by_desc(conversation_summaries::Column::UpdatedAt)
        .one(db)
        .await?;

    Ok(row.map(summary_from_entity))
}

pub async fn upsert_summary(
    db: &DatabaseConnection,
    conversation_id: &str,
    summary_text: &str,
    compressed_until_message_id: Option<&str>,
    token_count: Option<u32>,
    model_used: Option<&str>,
) -> Result<ConversationSummary> {
    let now = now_ts();

    let existing = conversation_summaries::Entity::find()
        .filter(conversation_summaries::Column::ConversationId.eq(conversation_id))
        .one(db)
        .await?;

    match existing {
        Some(row) => {
            let mut am: conversation_summaries::ActiveModel = row.into();
            am.summary_text = Set(summary_text.to_string());
            am.compressed_until_message_id =
                Set(compressed_until_message_id.map(|s| s.to_string()));
            am.token_count = Set(token_count.map(|v| v as i64));
            am.model_used = Set(model_used.map(|s| s.to_string()));
            am.updated_at = Set(now);
            am.update(db).await?;
        },
        None => {
            let id = gen_id();
            conversation_summaries::ActiveModel {
                id: Set(id),
                conversation_id: Set(conversation_id.to_string()),
                summary_text: Set(summary_text.to_string()),
                compressed_until_message_id: Set(
                    compressed_until_message_id.map(|s| s.to_string()),
                ),
                token_count: Set(token_count.map(|v| v as i64)),
                model_used: Set(model_used.map(|s| s.to_string())),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(db)
            .await?;
        },
    }

    get_summary(db, conversation_id).await?.ok_or_else(|| {
        AxAgentError::Database(sea_orm::DbErr::Custom(
            "Failed to read back upserted summary".into(),
        ))
    })
}

pub async fn delete_summary(db: &DatabaseConnection, conversation_id: &str) -> Result<()> {
    conversation_summaries::Entity::delete_many()
        .filter(conversation_summaries::Column::ConversationId.eq(conversation_id))
        .exec(db)
        .await?;
    Ok(())
}
