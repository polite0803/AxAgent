// SPDX-License-Identifier: AGPL-3.0-only

//! 检索命中反馈闭环命令。
//!
//! 此前 `retrieval_hits` 表只有写入路径（streaming.rs 的 record_hits），
//! 无任何读取方，形成"只写不读"的数据沼泽。本模块补齐读取/反馈/统计命令，
//! 让前端可以在引用 chip 上展示反馈按钮，并把反馈数据回流到 RAG 自适应优化。
//!
//! 命令清单：
//! - `list_retrieval_hits_by_message`：按消息 ID 列出命中（前端展示引用列表 + 反馈 UI）
//! - `list_retrieval_hits_by_conversation`：按会话 ID 列出命中（会话级分析）
//! - `update_retrieval_hit_feedback`：更新单条命中的用户反馈
//! - `get_retrieval_feedback_stats`：查询反馈统计（RAG 自适应优化的输入）

use crate::AppState;
use agent_macro::agent_command;
use axagent_dao::repo::retrieval_hit::{
    self, FEEDBACK_IRRELEVANT, FEEDBACK_NEGATIVE, FEEDBACK_POSITIVE, FeedbackStats, RetrievalHit,
};
use serde::Deserialize;
use tauri::State;

/// 校验 feedback 字符串是否合法。
fn validate_feedback(fb: Option<&str>) -> Result<(), String> {
    if let Some(fb) = fb {
        match fb {
            FEEDBACK_POSITIVE | FEEDBACK_NEGATIVE | FEEDBACK_IRRELEVANT => Ok(()),
            _ => Err(format!(
                "Invalid feedback value: {} (expected positive/negative/irrelevant)",
                fb
            )),
        }
    } else {
        Ok(())
    }
}

/// 按消息 ID 列出检索命中（前端展示引用列表 + 反馈 UI）。
#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "按消息列出检索命中")]
#[tauri::command]
pub async fn list_retrieval_hits_by_message(
    state: State<'_, AppState>,
    message_id: String,
) -> Result<Vec<RetrievalHit>, String> {
    retrieval_hit::list_hits_by_message(state.harness.db(), &message_id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 按会话 ID 列出检索命中（会话级分析、反馈统计）。
#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "按会话列出检索命中")]
#[tauri::command]
pub async fn list_retrieval_hits_by_conversation(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<RetrievalHit>, String> {
    retrieval_hit::list_hits_by_conversation(state.harness.db(), &conversation_id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

/// 更新单条命中的用户反馈。
///
/// `feedback` 取值：
/// - `"positive"`：正反馈（引用有用）
/// - `"negative"`：负反馈（引用错误/不相关）
/// - `"irrelevant"`：标记无关
/// - `None`：清除反馈
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "更新检索命中反馈")]
#[tauri::command]
pub async fn update_retrieval_hit_feedback(
    state: State<'_, AppState>,
    hit_id: String,
    feedback: Option<String>,
) -> Result<RetrievalHit, String> {
    validate_feedback(feedback.as_deref())?;
    retrieval_hit::update_hit_feedback(state.harness.db(), &hit_id, feedback.as_deref())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })
}

/// 反馈统计查询参数。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackStatsQuery {
    /// 可选：按知识库 ID 过滤（None = 全部 KB）
    pub knowledge_base_id: Option<String>,
    /// 可选：起始时间戳（Unix 秒），None = 不限时间范围
    pub since: Option<i64>,
}

/// 查询反馈统计（RAG 自适应优化的输入）。
#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "获取检索反馈统计")]
#[tauri::command]
pub async fn get_retrieval_feedback_stats(
    state: State<'_, AppState>,
    query: Option<FeedbackStatsQuery>,
) -> Result<FeedbackStats, String> {
    let (kb_id, since) = match query {
        Some(q) => (q.knowledge_base_id, q.since),
        None => (None, None),
    };
    retrieval_hit::get_feedback_stats(state.harness.db(), kb_id.as_deref(), since).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

/// 按引用定位更新反馈（前端 chip 反馈按钮调用）。
///
/// 前端 CiteRefNode 只有 (message_id, document_id, chunk_ref)，
/// 没有直接的 hit_id。本命令封装"按引用查找 + 更新反馈"两步操作，
/// 让前端一次调用完成反馈提交。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "按引用更新检索反馈")]
#[tauri::command]
pub async fn update_retrieval_hit_feedback_by_ref(
    state: State<'_, AppState>,
    message_id: String,
    document_id: String,
    chunk_ref: String,
    feedback: Option<String>,
) -> Result<bool, String> {
    validate_feedback(feedback.as_deref())?;

    // 按 (message_id, document_id, chunk_ref) 查找命中记录
    let hits = retrieval_hit::list_hits_by_message(state.harness.db(), &message_id).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )?;

    let target =
        hits.into_iter().find(|h| h.document_id == document_id && h.chunk_ref == chunk_ref);

    let Some(hit) = target else {
        // 未找到对应命中记录，返回 false（不报错，避免阻塞前端 UI）
        tracing::warn!(
            "[retrieval_feedback] 未找到命中记录 msg={} doc={} chunk={}",
            message_id,
            document_id,
            chunk_ref
        );
        return Ok(false);
    };

    retrieval_hit::update_hit_feedback(state.harness.db(), &hit.id, feedback.as_deref())
        .await
        .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    Ok(true)
}
