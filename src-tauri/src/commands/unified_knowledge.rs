// SPDX-License-Identifier: AGPL-3.0-only

//! 统一知识源搜索命令。
//!
//! 通过 `UnifiedKnowledgeSource` 接口，对 RAG/Wiki/Memory/Obsidian 四类知识源
//! 发起统一搜索，前端无需关心源类型差异。

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::{common, knowledge_source as ks_err};
use axagent_harness::search_sources::{KnowledgeSourceType, SearchResult};
use axagent_search::sources::unified_sources;
use serde::Deserialize;
use tauri::State;

/// 包装错误为带错误码的响应（与其它命令模块一致）。
fn command_error(e: impl std::fmt::Display, code: &str) -> String {
    ErrorResponse::err_with_detail(code, e.to_string())
}

/// 统一搜索请求参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedSearchRequest {
    /// 源类型过滤（None = 全部源）
    pub source_type: Option<String>,
    /// 源 ID（kb_id / wiki_id / namespace_id / vault_id）
    pub source_id: Option<String>,
    /// 查询文本
    pub query: String,
    /// 最多返回结果数（默认 10）
    pub top_k: Option<usize>,
}

/// 统一搜索 Tauri 命令
///
/// 遍历所有已注册的统一知识源，按条件搜索并聚合结果。
#[tauri::command]
pub async fn unified_knowledge_search(
    _state: State<'_, AppState>,
    request: UnifiedSearchRequest,
) -> Result<Vec<SearchResult>, String> {
    let top_k = request.top_k.unwrap_or(10);
    let source_type_filter = request.source_type.as_deref().and_then(|s| match s {
        "knowledge_base" => Some(KnowledgeSourceType::KnowledgeBase),
        "wiki" => Some(KnowledgeSourceType::Wiki),
        "memory" => Some(KnowledgeSourceType::Memory),
        "obsidian_vault" => Some(KnowledgeSourceType::ObsidianVault),
        _ => None,
    });

    let sources = unified_sources();
    if sources.is_empty() {
        tracing::warn!("[unified_knowledge_search] 无已注册的统一知识源");
        return Ok(vec![]);
    }

    let mut all_results: Vec<SearchResult> = Vec::new();
    let per_source = (top_k / sources.len().max(1)).max(3);

    for source in sources {
        // 按源类型过滤
        if let Some(ref filter) = source_type_filter
            && source.source_type() != *filter
        {
            continue;
        }

        // 如果指定了 source_id，只搜索匹配的源；否则搜索所有源
        let source_id = match request.source_id.as_deref() {
            Some(id) if !id.is_empty() => id,
            _ => "",
        };

        match source.search(source_id, &request.query, per_source).await {
            Ok(results) => all_results.extend(results),
            Err(e) => {
                tracing::warn!(
                    "[unified_knowledge_search] 源 {:?} 搜索失败: {}",
                    source.source_type(),
                    e
                );
            },
        }
    }

    // 全局排序 + 截断
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all_results.truncate(top_k);

    Ok(all_results)
}

/// 查询指定源的元数据
#[tauri::command]
pub async fn unified_source_meta(
    _state: State<'_, AppState>,
    source_type: String,
    source_id: String,
) -> Result<axagent_harness::search_sources::KnowledgeSourceMeta, String> {
    let target_type = match source_type.as_str() {
        "knowledge_base" => KnowledgeSourceType::KnowledgeBase,
        "wiki" => KnowledgeSourceType::Wiki,
        "memory" => KnowledgeSourceType::Memory,
        "obsidian_vault" => KnowledgeSourceType::ObsidianVault,
        other => {
            return Err(command_error(
                format!("未知的源类型: {}", other),
                ks_err::TYPE_UNSUPPORTED,
            ));
        },
    };

    let sources = unified_sources();
    for source in sources {
        if source.source_type() == target_type {
            return source
                .get_source_meta(&source_id)
                .await
                .map_err(|e| command_error(e, ks_err::NOT_FOUND));
        }
    }

    Err(command_error(format!("未找到类型为 {:?} 的已注册知识源", target_type), ks_err::NOT_FOUND))
}

/// 反馈数据湖查询参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackQueryRequest {
    pub event_types: Option<Vec<String>>,
    pub conversation_id: Option<String>,
    pub source_id: Option<String>,
    pub source_type: Option<String>,
    pub start_time: Option<i64>,
    pub end_time: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// 查询反馈数据湖
#[tauri::command]
pub async fn query_feedback_lake(
    _state: State<'_, AppState>,
    request: FeedbackQueryRequest,
) -> Result<Vec<axagent_harness::FeedbackEvent>, String> {
    let lake = axagent_harness::feedback_data_lake::global_feedback_lake()
        .ok_or_else(|| command_error("反馈数据湖未注册", common::INTERNAL))?;

    let event_types = request.event_types.map(|types| {
        types
            .into_iter()
            .filter_map(|t| match t.as_str() {
                "retrieval_hit" => Some(axagent_harness::FeedbackEventType::RetrievalHit),
                "tool_call" => Some(axagent_harness::FeedbackEventType::ToolCall),
                "memory_access" => Some(axagent_harness::FeedbackEventType::MemoryAccess),
                "wiki_edit" => Some(axagent_harness::FeedbackEventType::WikiEdit),
                _ => None,
            })
            .collect()
    });

    let filter = axagent_harness::FeedbackQuery {
        event_types,
        conversation_id: request.conversation_id,
        source_id: request.source_id,
        source_type: request.source_type,
        start_time: request.start_time,
        end_time: request.end_time,
        limit: request.limit,
        offset: request.offset,
    };

    lake.query_feedback(filter).await.map_err(|e| command_error(e, common::INTERNAL))
}

/// 获取反馈统计
#[tauri::command]
pub async fn get_feedback_stats(
    _state: State<'_, AppState>,
    knowledge_base_id: String,
    since: Option<i64>,
) -> Result<f64, String> {
    let lake = axagent_harness::feedback_data_lake::global_feedback_lake()
        .ok_or_else(|| command_error("反馈数据湖未注册", common::INTERNAL))?;

    lake.positive_feedback_rate(&knowledge_base_id, since.unwrap_or(0))
        .await
        .map_err(|e| command_error(e, common::INTERNAL))
}
