// SPDX-License-Identifier: AGPL-3.0-only

//! 反馈数据湖 DAO 实现
//!
//! 实现 `axagent_harness::FeedbackDataLake` trait，
//! 将反馈数据写入对应的数据库表。

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use sea_orm::*;

use axagent_entities::{memory_access_logs, retrieval_hits, tool_call_logs, wiki_edit_logs};
use axagent_harness::{
    FeedbackDataLake, FeedbackEventType, FeedbackQuery, FeedbackToolCallRecord, MemoryAccessRecord,
    Result, RetrievalHitRecord, WikiEditRecord,
};

use crate::repo::retrieval_hit;

/// 反馈数据湖 DAO 实现
pub struct FeedbackDataLakeDao {
    db: Arc<DatabaseConnection>,
}

impl FeedbackDataLakeDao {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn gen_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn now_ts() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
}

#[async_trait]
impl FeedbackDataLake for FeedbackDataLakeDao {
    // ── 检索命中 ──────────────────────────────────────────

    async fn insert_retrieval_hit(&self, record: RetrievalHitRecord) -> Result<()> {
        // 复用现有 retrieval_hit DAO，获取生成的 ID
        let generated_id = retrieval_hit::record_hit(
            &self.db,
            &record.conversation_id,
            &record.message_id,
            &record.knowledge_base_id,
            &record.document_id,
            &record.chunk_ref,
            record.score,
            &record.preview,
        )
        .await
        .map_err(|e| e.to_string())?;

        // 如果有反馈，使用正确的 ID 更新反馈字段
        if let Some(feedback) = record.feedback
            && (record.feedback_at.is_some()
                || feedback == "positive"
                || feedback == "negative"
                || feedback == "irrelevant")
            && let Err(e) =
                retrieval_hit::update_hit_feedback(&self.db, &generated_id, Some(&feedback)).await
        {
            tracing::warn!("更新检索命中反馈失败 id={}: {}", generated_id, e);
        }

        if record.used_in_response
            && let Err(e) =
                retrieval_hit::mark_used_in_response(&self.db, &generated_id, true).await
        {
            tracing::warn!("标记检索命中 used_in_response 失败 id={}: {}", generated_id, e);
        }

        Ok(())
    }

    async fn batch_insert_retrieval_hits(&self, records: Vec<RetrievalHitRecord>) -> Result<()> {
        // 分批处理，每批 50 条，避免单事务过大
        for chunk in records.chunks(50) {
            for record in chunk {
                self.insert_retrieval_hit(record.clone()).await?;
            }
        }
        Ok(())
    }

    async fn update_retrieval_hit_feedback(
        &self,
        hit_id: &str,
        feedback: Option<&str>,
        used_in_response: Option<bool>,
    ) -> Result<()> {
        if let Some(feedback_val) = feedback
            && let Err(e) =
                retrieval_hit::update_hit_feedback(&self.db, hit_id, Some(feedback_val)).await
        {
            tracing::warn!("更新检索命中反馈失败 id={}: {}", hit_id, e);
        }
        if let Some(used) = used_in_response
            && let Err(e) = retrieval_hit::mark_used_in_response(&self.db, hit_id, used).await
        {
            tracing::warn!("标记检索命中 used_in_response 失败 id={}: {}", hit_id, e);
        }
        Ok(())
    }

    // ── 工具调用 ──────────────────────────────────────────

    async fn insert_tool_call(&self, record: FeedbackToolCallRecord) -> Result<()> {
        let now = Self::now_ts();
        let id = if record.id.is_empty() {
            Self::gen_id()
        } else {
            record.id
        };

        let result_str = record.result.map(|r| serde_json::to_string(&r).unwrap_or_default());

        let am = tool_call_logs::ActiveModel {
            id: Set(id),
            conversation_id: Set(record.conversation_id),
            trajectory_id: Set(record.trajectory_id),
            step_index: Set(record.step_index),
            tool_name: Set(record.tool_name),
            arguments: Set(serde_json::to_string(&record.arguments).unwrap_or_default()),
            result: Set(result_str),
            success: Set(if record.success { 1 } else { 0 }),
            duration_ms: Set(record.duration_ms),
            related_source_id: Set(record.related_source_id),
            created_at: Set(if record.created_at > 0 {
                record.created_at
            } else {
                now
            }),
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn batch_insert_tool_calls(&self, records: Vec<FeedbackToolCallRecord>) -> Result<()> {
        // 分批处理，每批 50 条
        for chunk in records.chunks(50) {
            for record in chunk {
                self.insert_tool_call(record.clone()).await?;
            }
        }
        Ok(())
    }

    // ── 记忆访问 ──────────────────────────────────────────

    async fn insert_memory_access(&self, record: MemoryAccessRecord) -> Result<()> {
        let now = Self::now_ts();
        let id = if record.id.is_empty() {
            Self::gen_id()
        } else {
            record.id
        };

        let am = memory_access_logs::ActiveModel {
            id: Set(id),
            conversation_id: Set(record.conversation_id),
            namespace_id: Set(record.namespace_id),
            memory_id: Set(record.memory_id),
            access_type: Set(record.access_type),
            query: Set(record.query),
            content_snippet: Set(record.content_snippet),
            hit: Set(if record.hit { 1 } else { 0 }),
            created_at: Set(if record.created_at > 0 {
                record.created_at
            } else {
                now
            }),
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Wiki 编辑 ──────────────────────────────────────────

    async fn insert_wiki_edit(&self, record: WikiEditRecord) -> Result<()> {
        let now = Self::now_ts();
        let id = if record.id.is_empty() {
            Self::gen_id()
        } else {
            record.id
        };

        let am = wiki_edit_logs::ActiveModel {
            id: Set(id),
            conversation_id: Set(record.conversation_id),
            wiki_id: Set(record.wiki_id),
            note_id: Set(record.note_id),
            operation: Set(record.operation),
            before_snippet: Set(record.before_snippet),
            after_snippet: Set(record.after_snippet),
            reason: Set(record.reason),
            quality_score: Set(record.quality_score),
            created_at: Set(if record.created_at > 0 {
                record.created_at
            } else {
                now
            }),
        };

        am.insert(self.db.as_ref()).await.map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── 查询接口 ──────────────────────────────────────────

    async fn query_feedback(
        &self,
        filter: FeedbackQuery,
    ) -> Result<Vec<axagent_harness::FeedbackEvent>> {
        // 委托给各类型查询后合并
        let mut all_events = Vec::new();

        let types_to_query = filter.event_types.clone().unwrap_or_else(|| {
            vec![
                FeedbackEventType::RetrievalHit,
                FeedbackEventType::ToolCall,
                FeedbackEventType::MemoryAccess,
                FeedbackEventType::WikiEdit,
            ]
        });

        for event_type in &types_to_query {
            match event_type {
                FeedbackEventType::RetrievalHit => {
                    let hits = self.query_retrieval_hits(filter.clone()).await?;
                    for hit in hits {
                        let event = axagent_harness::FeedbackEvent {
                            id: hit.id.clone(),
                            event_type: FeedbackEventType::RetrievalHit,
                            conversation_id: Some(hit.conversation_id),
                            message_id: Some(hit.message_id),
                            user_id: None,
                            session_id: None,
                            source_id: Some(hit.knowledge_base_id),
                            source_type: Some("knowledge_base".to_string()),
                            payload: serde_json::json!({
                                "document_id": hit.document_id,
                                "chunk_ref": hit.chunk_ref,
                                "score": hit.score,
                                "feedback": hit.feedback,
                                "used_in_response": hit.used_in_response,
                            }),
                            created_at: hit.created_at,
                        };
                        all_events.push(event);
                    }
                },
                FeedbackEventType::ToolCall => {
                    let calls = self.query_tool_calls(filter.clone()).await?;
                    for call in calls {
                        let event = axagent_harness::FeedbackEvent {
                            id: call.id.clone(),
                            event_type: FeedbackEventType::ToolCall,
                            conversation_id: call.conversation_id,
                            message_id: None,
                            user_id: None,
                            session_id: None,
                            source_id: call.related_source_id,
                            source_type: None,
                            payload: serde_json::json!({
                                "tool_name": call.tool_name,
                                "arguments": call.arguments,
                                "success": call.success,
                                "duration_ms": call.duration_ms,
                            }),
                            created_at: call.created_at,
                        };
                        all_events.push(event);
                    }
                },
                FeedbackEventType::MemoryAccess => {
                    let accesses = self.query_memory_access(filter.clone()).await?;
                    for access in accesses {
                        let event = axagent_harness::FeedbackEvent {
                            id: access.id.clone(),
                            event_type: FeedbackEventType::MemoryAccess,
                            conversation_id: access.conversation_id,
                            message_id: None,
                            user_id: None,
                            session_id: None,
                            source_id: Some(access.namespace_id),
                            source_type: Some("memory".to_string()),
                            payload: serde_json::json!({
                                "memory_id": access.memory_id,
                                "access_type": access.access_type,
                                "hit": access.hit,
                            }),
                            created_at: access.created_at,
                        };
                        all_events.push(event);
                    }
                },
                FeedbackEventType::WikiEdit => {
                    let edits = self.query_wiki_edits(filter.clone()).await?;
                    for edit in edits {
                        let event = axagent_harness::FeedbackEvent {
                            id: edit.id.clone(),
                            event_type: FeedbackEventType::WikiEdit,
                            conversation_id: edit.conversation_id,
                            message_id: None,
                            user_id: None,
                            session_id: None,
                            source_id: Some(edit.wiki_id),
                            source_type: Some("wiki".to_string()),
                            payload: serde_json::json!({
                                "note_id": edit.note_id,
                                "operation": edit.operation,
                                "quality_score": edit.quality_score,
                            }),
                            created_at: edit.created_at,
                        };
                        all_events.push(event);
                    }
                },
            }
        }

        // 按时间倒序排序
        all_events.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        // 应用 limit/offset
        if let Some(limit) = filter.limit {
            let limit = limit as usize;
            let offset = filter.offset.unwrap_or(0) as usize;
            all_events = all_events.into_iter().skip(offset).take(limit).collect();
        }

        Ok(all_events)
    }

    async fn query_retrieval_hits(&self, filter: FeedbackQuery) -> Result<Vec<RetrievalHitRecord>> {
        let mut select = retrieval_hits::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(retrieval_hits::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(retrieval_hits::Column::KnowledgeBaseId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(retrieval_hits::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(retrieval_hits::Column::CreatedAt.lte(end));
        }

        select = select.order_by_desc(retrieval_hits::Column::CreatedAt);

        if let Some(limit) = filter.limit {
            select = select.limit(limit as u64);
            if let Some(offset) = filter.offset {
                select = select.offset(offset as u64);
            }
        }

        let models = select.all(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| RetrievalHitRecord {
                id: m.id,
                conversation_id: m.conversation_id,
                message_id: m.message_id,
                knowledge_base_id: m.knowledge_base_id,
                document_id: m.document_id,
                chunk_ref: m.chunk_ref,
                score: m.score,
                preview: m.preview,
                feedback: m.feedback,
                feedback_at: m.feedback_at,
                used_in_response: m.used_in_response != 0,
                score_after_rerank: m.score_after_rerank,
                created_at: m.created_at,
            })
            .collect())
    }

    async fn query_tool_calls(&self, filter: FeedbackQuery) -> Result<Vec<FeedbackToolCallRecord>> {
        let mut select = tool_call_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(tool_call_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(tool_call_logs::Column::RelatedSourceId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(tool_call_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(tool_call_logs::Column::CreatedAt.lte(end));
        }

        select = select.order_by_desc(tool_call_logs::Column::CreatedAt);

        if let Some(limit) = filter.limit {
            select = select.limit(limit as u64);
            if let Some(offset) = filter.offset {
                select = select.offset(offset as u64);
            }
        }

        let models = select.all(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| FeedbackToolCallRecord {
                id: m.id,
                conversation_id: m.conversation_id,
                trajectory_id: m.trajectory_id,
                step_index: m.step_index,
                tool_name: m.tool_name,
                arguments: serde_json::from_str(&m.arguments).unwrap_or_default(),
                result: m.result.and_then(|r| serde_json::from_str(&r).ok()),
                success: m.success != 0,
                duration_ms: m.duration_ms,
                related_source_id: m.related_source_id,
                created_at: m.created_at,
            })
            .collect())
    }

    async fn query_memory_access(&self, filter: FeedbackQuery) -> Result<Vec<MemoryAccessRecord>> {
        let mut select = memory_access_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(memory_access_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(memory_access_logs::Column::NamespaceId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(memory_access_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(memory_access_logs::Column::CreatedAt.lte(end));
        }

        select = select.order_by_desc(memory_access_logs::Column::CreatedAt);

        if let Some(limit) = filter.limit {
            select = select.limit(limit as u64);
            if let Some(offset) = filter.offset {
                select = select.offset(offset as u64);
            }
        }

        let models = select.all(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| MemoryAccessRecord {
                id: m.id,
                conversation_id: m.conversation_id,
                namespace_id: m.namespace_id,
                memory_id: m.memory_id,
                access_type: m.access_type,
                query: m.query,
                content_snippet: m.content_snippet,
                hit: m.hit != 0,
                created_at: m.created_at,
            })
            .collect())
    }

    async fn query_wiki_edits(&self, filter: FeedbackQuery) -> Result<Vec<WikiEditRecord>> {
        let mut select = wiki_edit_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(wiki_edit_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(wiki_edit_logs::Column::WikiId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(wiki_edit_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(wiki_edit_logs::Column::CreatedAt.lte(end));
        }

        select = select.order_by_desc(wiki_edit_logs::Column::CreatedAt);

        if let Some(limit) = filter.limit {
            select = select.limit(limit as u64);
            if let Some(offset) = filter.offset {
                select = select.offset(offset as u64);
            }
        }

        let models = select.all(self.db.as_ref()).await.map_err(|e| e.to_string())?;

        Ok(models
            .into_iter()
            .map(|m| WikiEditRecord {
                id: m.id,
                conversation_id: m.conversation_id,
                wiki_id: m.wiki_id,
                note_id: m.note_id,
                operation: m.operation,
                before_snippet: m.before_snippet,
                after_snippet: m.after_snippet,
                reason: m.reason,
                quality_score: m.quality_score,
                created_at: m.created_at,
            })
            .collect())
    }

    // ── 统计接口 ──────────────────────────────────────────

    async fn count_by_event_type(&self, filter: FeedbackQuery) -> Result<HashMap<String, u64>> {
        let mut result = HashMap::new();

        // 使用 SQL COUNT 统计各类型数量，避免全量加载
        let retrieval_count = Self::count_retrieval_hits(&self.db, &filter).await.unwrap_or(0);
        result.insert("retrieval_hit".to_string(), retrieval_count);

        let tool_call_count = Self::count_tool_calls(&self.db, &filter).await.unwrap_or(0);
        result.insert("tool_call".to_string(), tool_call_count);

        let memory_access_count = Self::count_memory_access(&self.db, &filter).await.unwrap_or(0);
        result.insert("memory_access".to_string(), memory_access_count);

        let wiki_edit_count = Self::count_wiki_edits(&self.db, &filter).await.unwrap_or(0);
        result.insert("wiki_edit".to_string(), wiki_edit_count);

        Ok(result)
    }

    async fn positive_feedback_rate(&self, knowledge_base_id: &str, since: i64) -> Result<f64> {
        let stats =
            retrieval_hit::get_feedback_stats(&self.db, Some(knowledge_base_id), Some(since))
                .await
                .map_err(|e| e.to_string())?;
        Ok(stats.positive_rate)
    }
}

// ── 辅助计数函数（独立于 trait 实现） ──────────────────────

impl FeedbackDataLakeDao {
    async fn count_retrieval_hits(db: &DatabaseConnection, filter: &FeedbackQuery) -> Result<u64> {
        let mut select = retrieval_hits::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(retrieval_hits::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(retrieval_hits::Column::KnowledgeBaseId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(retrieval_hits::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(retrieval_hits::Column::CreatedAt.lte(end));
        }

        select.count(db).await.map_err(Into::into)
    }

    async fn count_tool_calls(db: &DatabaseConnection, filter: &FeedbackQuery) -> Result<u64> {
        let mut select = tool_call_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(tool_call_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(tool_call_logs::Column::RelatedSourceId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(tool_call_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(tool_call_logs::Column::CreatedAt.lte(end));
        }

        select.count(db).await.map_err(Into::into)
    }

    async fn count_memory_access(db: &DatabaseConnection, filter: &FeedbackQuery) -> Result<u64> {
        let mut select = memory_access_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(memory_access_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(memory_access_logs::Column::NamespaceId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(memory_access_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(memory_access_logs::Column::CreatedAt.lte(end));
        }

        select.count(db).await.map_err(Into::into)
    }

    async fn count_wiki_edits(db: &DatabaseConnection, filter: &FeedbackQuery) -> Result<u64> {
        let mut select = wiki_edit_logs::Entity::find();

        if let Some(conv_id) = filter.conversation_id.as_deref() {
            select = select.filter(wiki_edit_logs::Column::ConversationId.eq(conv_id));
        }
        if let Some(source_id) = filter.source_id.as_deref() {
            select = select.filter(wiki_edit_logs::Column::WikiId.eq(source_id));
        }
        if let Some(start) = filter.start_time {
            select = select.filter(wiki_edit_logs::Column::CreatedAt.gte(start));
        }
        if let Some(end) = filter.end_time {
            select = select.filter(wiki_edit_logs::Column::CreatedAt.lte(end));
        }

        select.count(db).await.map_err(Into::into)
    }
}
