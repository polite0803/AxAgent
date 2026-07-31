// SPDX-License-Identifier: AGPL-3.0-only

use sea_orm::*;

use axagent_entities::retrieval_hits;
use axagent_harness::util_fns::gen_id;

/// 单条检索命中记录（DTO，向上层暴露）。
///
/// 与 `retrieval_hits::Model` 字段对齐，但作为 DAO 层的公开类型，
/// 便于上层（commands）不直接依赖 entities crate。
#[derive(Debug, Clone, serde::Serialize)]
pub struct RetrievalHit {
    pub id: String,
    pub conversation_id: String,
    pub message_id: String,
    pub knowledge_base_id: String,
    pub document_id: String,
    pub chunk_ref: String,
    pub score: f64,
    pub preview: String,
    pub feedback: Option<String>,
    pub feedback_at: Option<i64>,
    pub used_in_response: i32,
    pub score_after_rerank: Option<f64>,
    pub created_at: i64,
}

impl From<retrieval_hits::Model> for RetrievalHit {
    fn from(m: retrieval_hits::Model) -> Self {
        Self {
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
            used_in_response: m.used_in_response,
            score_after_rerank: m.score_after_rerank,
            created_at: m.created_at,
        }
    }
}

/// 反馈类型常量（与前端 i18n key 对齐）。
pub const FEEDBACK_POSITIVE: &str = "positive";
pub const FEEDBACK_NEGATIVE: &str = "negative";
pub const FEEDBACK_IRRELEVANT: &str = "irrelevant";

/// Record a single retrieval hit, 返回生成的 ID 供后续更新使用。
#[allow(clippy::too_many_arguments)]
pub async fn record_hit(
    db: &DatabaseConnection,
    conversation_id: &str,
    message_id: &str,
    knowledge_base_id: &str,
    document_id: &str,
    chunk_ref: &str,
    score: f64,
    preview: &str,
) -> Result<String, DbErr> {
    let id = gen_id();
    let now = chrono::Utc::now().timestamp_millis();
    let am = retrieval_hits::ActiveModel {
        id: Set(id.clone()),
        conversation_id: Set(conversation_id.to_string()),
        message_id: Set(message_id.to_string()),
        knowledge_base_id: Set(knowledge_base_id.to_string()),
        document_id: Set(document_id.to_string()),
        chunk_ref: Set(chunk_ref.to_string()),
        score: Set(score),
        preview: Set(preview.to_string()),
        feedback: Set(None),
        feedback_at: Set(None),
        used_in_response: Set(0),
        score_after_rerank: Set(None),
        created_at: Set(now),
    };
    am.insert(db).await?;
    Ok(id)
}

/// Record multiple retrieval hits in bulk.
pub async fn record_hits(
    db: &DatabaseConnection,
    conversation_id: &str,
    message_id: &str,
    hits: &[(String, String, String, f64, String)], // (kb_id, doc_id, chunk_ref, score, preview)
) -> Result<Vec<String>, DbErr> {
    let mut ids = Vec::with_capacity(hits.len());
    for (kb_id, doc_id, chunk_ref, score, preview) in hits {
        match record_hit(db, conversation_id, message_id, kb_id, doc_id, chunk_ref, *score, preview)
            .await
        {
            Ok(id) => ids.push(id),
            Err(e) => {
                tracing::warn!(
                    "[retrieval_hit] 记录失败 conv={} msg={} kb={} chunk={}: {}",
                    conversation_id,
                    message_id,
                    kb_id,
                    chunk_ref,
                    e
                );
            },
        }
    }
    Ok(ids)
}

/// 按消息 ID 列出检索命中（用于前端展示引用列表 + 反馈 UI）。
pub async fn list_hits_by_message(
    db: &DatabaseConnection,
    message_id: &str,
) -> Result<Vec<RetrievalHit>, DbErr> {
    let models = retrieval_hits::Entity::find()
        .filter(retrieval_hits::Column::MessageId.eq(message_id))
        .order_by_desc(retrieval_hits::Column::Score)
        .all(db)
        .await?;
    Ok(models.into_iter().map(RetrievalHit::from).collect())
}

/// 按会话 ID 列出检索命中（用于反馈统计、会话级分析）。
pub async fn list_hits_by_conversation(
    db: &DatabaseConnection,
    conversation_id: &str,
) -> Result<Vec<RetrievalHit>, DbErr> {
    let models = retrieval_hits::Entity::find()
        .filter(retrieval_hits::Column::ConversationId.eq(conversation_id))
        .order_by_desc(retrieval_hits::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(models.into_iter().map(RetrievalHit::from).collect())
}

/// 更新单条命中记录的用户反馈。
///
/// `feedback` 取值：`FEEDBACK_POSITIVE` / `FEEDBACK_NEGATIVE` / `FEEDBACK_IRRELEVANT`。
/// 传入 `None` 表示清除反馈。
pub async fn update_hit_feedback(
    db: &DatabaseConnection,
    hit_id: &str,
    feedback: Option<&str>,
) -> Result<RetrievalHit, DbErr> {
    let model = retrieval_hits::Entity::find_by_id(hit_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::Custom(format!("RetrievalHit {} not found", hit_id)))?;

    let mut am = model.into_active_model();
    let now = chrono::Utc::now().timestamp_millis();
    am.feedback = Set(feedback.map(|s| s.to_string()));
    am.feedback_at = Set(if feedback.is_some() { Some(now) } else { None });
    am.update(db).await?;

    // 重新查询返回完整记录
    let updated = retrieval_hits::Entity::find_by_id(hit_id).one(db).await?.ok_or_else(|| {
        DbErr::Custom(format!("RetrievalHit {} disappeared after update", hit_id))
    })?;
    Ok(RetrievalHit::from(updated))
}

/// 标记命中记录是否在最终回复中被引用。
pub async fn mark_used_in_response(
    db: &DatabaseConnection,
    hit_id: &str,
    used: bool,
) -> Result<(), DbErr> {
    let model = retrieval_hits::Entity::find_by_id(hit_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::Custom(format!("RetrievalHit {} not found", hit_id)))?;

    let mut am = model.into_active_model();
    am.used_in_response = Set(if used { 1 } else { 0 });
    am.update(db).await?;
    Ok(())
}

/// 反馈统计数据（用于 RAG 自适应优化的输入）。
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct FeedbackStats {
    /// 总命中数
    pub total_hits: i64,
    /// 正反馈数
    pub positive: i64,
    /// 负反馈数
    pub negative: i64,
    /// 标记无关数
    pub irrelevant: i64,
    /// 未反馈数
    pub no_feedback: i64,
    /// 被引用数（used_in_response=1）
    pub used_in_response: i64,
    /// 正反馈率（positive / total_with_feedback）
    pub positive_rate: f64,
}

/// 查询反馈统计（可选按知识库/时间范围过滤）。
///
/// - `knowledge_base_id` 为 None 时统计全部 KB
/// - `since` 为 None 时不限时间范围（Unix 毫秒）
/// - 使用 SQL 聚合统计，避免全表加载到内存
pub async fn get_feedback_stats(
    db: &DatabaseConnection,
    knowledge_base_id: Option<&str>,
    since: Option<i64>,
) -> Result<FeedbackStats, DbErr> {
    let mut select = retrieval_hits::Entity::find();

    if let Some(kb_id) = knowledge_base_id {
        select = select.filter(retrieval_hits::Column::KnowledgeBaseId.eq(kb_id));
    }
    if let Some(since_ts) = since {
        select = select.filter(retrieval_hits::Column::CreatedAt.gte(since_ts));
    }

    // 分别聚合各反馈类型，使用 SQL COUNT 避免全表加载
    let total_hits = select.clone().count(db).await? as i64;

    let positive = count_feedback_with_value(db, knowledge_base_id, since, FEEDBACK_POSITIVE)
        .await
        .unwrap_or(0);
    let negative = count_feedback_with_value(db, knowledge_base_id, since, FEEDBACK_NEGATIVE)
        .await
        .unwrap_or(0);
    let irrelevant = count_feedback_with_value(db, knowledge_base_id, since, FEEDBACK_IRRELEVANT)
        .await
        .unwrap_or(0);
    let used_in_response = count_used_in_response(db, knowledge_base_id, since).await.unwrap_or(0);

    let no_feedback = if total_hits > positive + negative + irrelevant {
        total_hits - positive - negative - irrelevant
    } else {
        0
    };

    let total_with_feedback = positive + negative + irrelevant;
    let positive_rate = if total_with_feedback > 0 {
        positive as f64 / total_with_feedback as f64
    } else {
        0.0
    };

    Ok(FeedbackStats {
        total_hits,
        positive,
        negative,
        irrelevant,
        no_feedback,
        used_in_response,
        positive_rate,
    })
}

async fn count_feedback_with_value(
    db: &DatabaseConnection,
    knowledge_base_id: Option<&str>,
    since: Option<i64>,
    value: &str,
) -> Result<i64, DbErr> {
    let mut select =
        retrieval_hits::Entity::find().filter(retrieval_hits::Column::Feedback.eq(value));
    if let Some(kb_id) = knowledge_base_id {
        select = select.filter(retrieval_hits::Column::KnowledgeBaseId.eq(kb_id));
    }
    if let Some(since_ts) = since {
        select = select.filter(retrieval_hits::Column::CreatedAt.gte(since_ts));
    }
    select.count(db).await.map(|c| c as i64)
}

async fn count_used_in_response(
    db: &DatabaseConnection,
    knowledge_base_id: Option<&str>,
    since: Option<i64>,
) -> Result<i64, DbErr> {
    let mut select =
        retrieval_hits::Entity::find().filter(retrieval_hits::Column::UsedInResponse.ne(0));
    if let Some(kb_id) = knowledge_base_id {
        select = select.filter(retrieval_hits::Column::KnowledgeBaseId.eq(kb_id));
    }
    if let Some(since_ts) = since {
        select = select.filter(retrieval_hits::Column::CreatedAt.gte(since_ts));
    }
    select.count(db).await.map(|c| c as i64)
}

/// 按知识库聚合正负反馈计数，用于反馈应用定时任务调整 chunk 权重。
///
/// 返回 (kb_id, positive_count, negative_count, irrelevant_count) 列表。
pub async fn aggregate_feedback_by_kb(
    db: &DatabaseConnection,
    since: Option<i64>,
) -> Result<Vec<(String, i64, i64, i64)>, DbErr> {
    let mut select =
        retrieval_hits::Entity::find().filter(retrieval_hits::Column::Feedback.is_not_null());

    if let Some(since_ts) = since {
        select = select.filter(retrieval_hits::Column::CreatedAt.gte(since_ts));
    }

    let models = select.all(db).await?;

    let mut agg: std::collections::HashMap<String, (i64, i64, i64)> =
        std::collections::HashMap::new();
    for m in models {
        let entry = agg.entry(m.knowledge_base_id.clone()).or_insert((0, 0, 0));
        match m.feedback.as_deref() {
            Some(FEEDBACK_POSITIVE) => entry.0 += 1,
            Some(FEEDBACK_NEGATIVE) => entry.1 += 1,
            Some(FEEDBACK_IRRELEVANT) => entry.2 += 1,
            _ => {},
        }
    }

    Ok(agg.into_iter().map(|(kb_id, (p, n, i))| (kb_id, p, n, i)).collect())
}
