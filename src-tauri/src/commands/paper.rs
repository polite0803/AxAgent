// SPDX-License-Identifier: AGPL-3.0-only

//! Paper Overview Engine + Reading List 命令
//!
//! 暴露给前端的 Tauri 命令。所有命令返回 Result<T, String>，
//! 错误用 ErrorResponse 包装带错误码。

use crate::commands::error::{ErrorCategory, ErrorResponse};
use axagent_agent_macro::agent_command;
use axagent_harness::core_error::AxAgentError;
use axagent_harness::types::*;
use tauri::State;

use crate::AppState;

/// 把 AxAgentError 转换为前端可消费的 String（JSON 化的 ErrorResponse）
fn err_to_string(e: AxAgentError) -> String {
    String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
}

// ── Paper Overview ──────────────────────────────────────────────────────

#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "按知识库列出论文概览")]
#[tauri::command]
pub async fn list_paper_overviews_by_kb(
    state: State<'_, AppState>,
    knowledge_base_id: String,
) -> Result<Vec<PaperOverview>, String> {
    axagent_dao::repo::paper_overviews::list_by_kb(state.harness.db(), &knowledge_base_id)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "获取论文概览详情")]
#[tauri::command]
pub async fn get_paper_overview(
    state: State<'_, AppState>,
    id: String,
) -> Result<PaperOverview, String> {
    axagent_dao::repo::paper_overviews::get(state.harness.db(), &id).await.map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "按文档获取论文概览")]
#[tauri::command]
pub async fn get_paper_overview_by_document(
    state: State<'_, AppState>,
    document_id: String,
) -> Result<Option<PaperOverview>, String> {
    axagent_dao::repo::paper_overviews::get_by_document(state.harness.db(), &document_id)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "创建论文概览")]
#[tauri::command]
pub async fn create_paper_overview(
    state: State<'_, AppState>,
    input: CreatePaperOverviewInput,
) -> Result<PaperOverview, String> {
    axagent_dao::repo::paper_overviews::create(state.harness.db(), input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "更新论文概览")]
#[tauri::command]
pub async fn update_paper_overview(
    state: State<'_, AppState>,
    id: String,
    input: UpdatePaperOverviewInput,
) -> Result<PaperOverview, String> {
    axagent_dao::repo::paper_overviews::update(state.harness.db(), &id, input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "按文档插入或更新论文概览")]
#[tauri::command]
pub async fn upsert_paper_overview_by_document(
    state: State<'_, AppState>,
    input: CreatePaperOverviewInput,
) -> Result<PaperOverview, String> {
    axagent_dao::repo::paper_overviews::upsert_by_document(state.harness.db(), input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Dangerous, call_mode = StateInput, description = "删除论文概览")]
#[tauri::command]
pub async fn delete_paper_overview(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::paper_overviews::delete(state.harness.db(), &id).await.map_err(err_to_string)
}

// ── Reading List ────────────────────────────────────────────────────────

#[agent_command(domain = paper, safety = Safe, call_mode = StateOnly, description = "列出阅读列表")]
#[tauri::command]
pub async fn list_reading_lists(state: State<'_, AppState>) -> Result<Vec<ReadingList>, String> {
    axagent_dao::repo::reading_lists::list_all(state.harness.db()).await.map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "创建阅读列表")]
#[tauri::command]
pub async fn create_reading_list(
    state: State<'_, AppState>,
    input: CreateReadingListInput,
) -> Result<ReadingList, String> {
    axagent_dao::repo::reading_lists::create(state.harness.db(), input).await.map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "更新阅读列表")]
#[tauri::command]
pub async fn update_reading_list(
    state: State<'_, AppState>,
    id: String,
    input: UpdateReadingListInput,
) -> Result<ReadingList, String> {
    axagent_dao::repo::reading_lists::update(state.harness.db(), &id, input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Dangerous, call_mode = StateInput, description = "删除阅读列表")]
#[tauri::command]
pub async fn delete_reading_list(state: State<'_, AppState>, id: String) -> Result<(), String> {
    axagent_dao::repo::reading_lists::delete(state.harness.db(), &id).await.map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "重新排序阅读列表")]
#[tauri::command]
pub async fn reorder_reading_lists(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> Result<(), String> {
    axagent_dao::repo::reading_lists::reorder(state.harness.db(), &ids).await.map_err(err_to_string)
}

// ── Reading List Items ──────────────────────────────────────────────────

#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "列出阅读列表项")]
#[tauri::command]
pub async fn list_reading_list_items(
    state: State<'_, AppState>,
    reading_list_id: String,
) -> Result<Vec<ReadingListItem>, String> {
    axagent_dao::repo::reading_list_items::list_by_reading_list(
        state.harness.db(),
        &reading_list_id,
    )
    .await
    .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "创建阅读列表项")]
#[tauri::command]
pub async fn create_reading_list_item(
    state: State<'_, AppState>,
    input: CreateReadingListItemInput,
) -> Result<ReadingListItem, String> {
    axagent_dao::repo::reading_list_items::create(state.harness.db(), input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "更新阅读列表项")]
#[tauri::command]
pub async fn update_reading_list_item(
    state: State<'_, AppState>,
    id: String,
    input: UpdateReadingListItemInput,
) -> Result<ReadingListItem, String> {
    axagent_dao::repo::reading_list_items::update(state.harness.db(), &id, input)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Dangerous, call_mode = StateInput, description = "删除阅读列表项")]
#[tauri::command]
pub async fn delete_reading_list_item(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_dao::repo::reading_list_items::delete(state.harness.db(), &id)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "设置阅读列表项状态")]
#[tauri::command]
pub async fn set_reading_list_item_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
) -> Result<ReadingListItem, String> {
    axagent_dao::repo::reading_list_items::set_status(state.harness.db(), &id, &status)
        .await
        .map_err(err_to_string)
}

#[agent_command(domain = paper, safety = Caution, call_mode = StateInput, description = "重新排序阅读列表项")]
#[tauri::command]
pub async fn reorder_reading_list_items(
    state: State<'_, AppState>,
    reading_list_id: String,
    ids: Vec<String>,
) -> Result<(), String> {
    axagent_dao::repo::reading_list_items::reorder(state.harness.db(), &reading_list_id, &ids)
        .await
        .map_err(err_to_string)
}

// ── Paper QA Pipeline ───────────────────────────────────────────────────
//
// 端到端论文问答流水线。设计为"上下文准备"模式：
// 后端负责加载 overview + 检索单文档 chunks + 拼接 prompt，
// 前端拿到 prompt 后通过现有 chat 流程调用 LLM，避免重复实现 LLM 调用逻辑。

/// Paper QA 准备结果
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperQAPreparedContext {
    /// 论文概览（若存在）
    pub overview: Option<PaperOverview>,
    /// 检索到的文档 chunks（已按相关性排序）
    pub chunks: Vec<axagent_search::vector_store::VectorSearchResult>,
    /// 拼接好的 RAG context 文本（含 [cite:N] 标记）
    pub context_text: String,
    /// 建议发送给 LLM 的完整 prompt
    pub suggested_prompt: String,
    /// 文档所属 KB ID
    pub knowledge_base_id: String,
    /// 文档标题
    pub document_title: String,
}

/// 准备 Paper QA 上下文：加载 overview + 检索单文档 chunks + 拼接 prompt。
///
/// 前端调用此命令后，把 `suggested_prompt` 作为用户消息发送给 LLM即可。
/// 检索范围自动限制在 `document_id` 对应的单篇论文内（多文档协同的特例）。
#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "准备论文问答上下文")]
#[tauri::command]
pub async fn prepare_paper_qa_context(
    state: State<'_, AppState>,
    document_id: String,
    question: String,
    top_k: Option<usize>,
) -> Result<PaperQAPreparedContext, String> {
    let db = state.harness.db();

    // 1. 加载文档元数据，拿到 knowledge_base_id 与 title
    let doc = axagent_dao::repo::knowledge::get_document(db, &document_id)
        .await
        .map_err(err_to_string)?;
    let knowledge_base_id = doc.knowledge_base_id.clone();
    let document_title = doc.title.clone();

    // 2. 加载 paper overview（若不存在则跳过，QA 仍可基于 chunks 进行）
    let overview = axagent_dao::repo::paper_overviews::get_by_document(db, &document_id)
        .await
        .map_err(err_to_string)?;

    // 3. 在该文档范围内检索相关 chunks（doc_ids 过滤）
    let top_k = top_k.unwrap_or(8).min(30);
    let mut chunks = crate::indexing::search_knowledge_with_doc_filter(
        db,
        state.harness.master_key(),
        &state.vector_store,
        &knowledge_base_id,
        &question,
        top_k,
        Some(std::slice::from_ref(&document_id)),
    )
    .await
    .map_err(|e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)))?;

    // 应用 KB 的 retrieval_threshold 过滤（与 collect_rag_context 一致）
    let kb = axagent_dao::repo::knowledge::get_knowledge_base(db, &knowledge_base_id)
        .await
        .map_err(err_to_string)?;
    let default_max_distance = 2.0_f32;
    let threshold = kb.retrieval_threshold.unwrap_or(0.0);
    let effective_threshold = if threshold > 0.0 {
        threshold
    } else {
        default_max_distance
    };
    chunks.retain(|r| r.score <= effective_threshold);

    // 4. 拼接 RAG context 文本（含 [cite:N] 标记，前端可渲染为引用 chip）
    let mut context_parts: Vec<String> = Vec::new();
    if let Some(ref ov) = overview {
        let mut overview_text = format!("[Paper Overview: {}]\n", document_title);
        if let Some(ref abs) = ov.abstract_text {
            overview_text.push_str(&format!("Abstract: {}\n", abs));
        }
        if let Some(ref tldr) = ov.tl_dr {
            overview_text.push_str(&format!("TL;DR: {}\n", tldr));
        }
        if !ov.key_concepts.is_empty() {
            overview_text.push_str(&format!("Key Concepts: {}\n", ov.key_concepts.join(", ")));
        }
        if !ov.contributions.is_empty() {
            overview_text
                .push_str(&format!("Contributions:\n- {}\n", ov.contributions.join("\n- ")));
        }
        if !ov.methods.is_empty() {
            overview_text.push_str(&format!("Methods:\n- {}\n", ov.methods.join("\n- ")));
        }
        if !ov.limitations.is_empty() {
            overview_text.push_str(&format!("Limitations:\n- {}\n", ov.limitations.join("\n- ")));
        }
        context_parts.push(overview_text);
    }

    let chunk_section = if chunks.is_empty() {
        "[Paper Excerpts]\n(no relevant chunks retrieved)".to_string()
    } else {
        let mut s = String::from("[Paper Excerpts]\n");
        for (i, c) in chunks.iter().enumerate() {
            s.push_str(&format!("[cite:{}] {}\n---\n", i, c.content));
        }
        s
    };
    context_parts.push(chunk_section);
    let context_text = context_parts.join("\n\n");

    // 5. 构造建议 prompt
    let overview_hint = if overview.is_some() {
        "You have access to a structured overview and excerpts from the paper. "
    } else {
        "You have access to excerpts from the paper. "
    };
    let suggested_prompt = format!(
        "{context_text}\n\n---\nQuestion: {question}\n\nPlease answer the question based on the paper context above. {overview_hint}\
Cite specific passages using [cite:N] notation where N matches the excerpt index. \
If the context doesn't contain enough information, say so explicitly."
    );

    Ok(PaperQAPreparedContext {
        overview,
        chunks,
        context_text,
        suggested_prompt,
        knowledge_base_id,
        document_title,
    })
}

/// 生成 Paper Overview 的 prompt（前端调用 LLM 后用 upsert_paper_overview_by_document 持久化）。
///
/// 流程：
/// 1. 加载文档所有 chunks，拼接为完整文本（截断到 max_chars）
/// 2. 构造结构化抽取 prompt，要求 LLM 返回 JSON
/// 3. 前端拿到 prompt 后调用 LLM，解析 JSON，调用 upsert_paper_overview_by_document
#[agent_command(domain = paper, safety = Safe, call_mode = StateInput, description = "生成论文概览提示词")]
#[tauri::command]
pub async fn generate_paper_overview_prompt(
    state: State<'_, AppState>,
    document_id: String,
    max_chars: Option<usize>,
) -> Result<String, String> {
    let db = state.harness.db();

    // 加载文档元数据
    let doc = axagent_dao::repo::knowledge::get_document(db, &document_id)
        .await
        .map_err(err_to_string)?;

    // 加载该文档的所有 chunks，按 chunk_index 排序拼接
    // 通过 vector_store 查询（chunks 存储在 vec_<collection>_meta 表中）
    let collection_id = format!("kb_{}", doc.knowledge_base_id);
    let chunks =
        state.vector_store.list_document_chunks(&collection_id, &document_id).await.map_err(
            |e| String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)),
        )?;
    let mut full_text = String::new();
    let max_chars = max_chars.unwrap_or(12000);
    for chunk in chunks.iter() {
        if full_text.len() + chunk.content.len() > max_chars {
            let remaining = max_chars.saturating_sub(full_text.len());
            if remaining > 0 {
                // 安全截断到字符边界，避免 panic
                let boundary = remaining.min(chunk.content.len());
                let safe_end = chunk.content.floor_char_boundary(boundary);
                full_text.push_str(&chunk.content[..safe_end]);
            }
            full_text.push_str("\n... [truncated]\n");
            break;
        }
        full_text.push_str(&chunk.content);
        full_text.push_str("\n\n");
    }

    let prompt = format!(
        "You are a research paper analyst. Analyze the following paper text and extract a structured overview.\n\n\
Paper Title: {title}\n\n\
Paper Text:\n{full_text}\n\n\
---\n\n\
Return a JSON object with exactly these fields (use empty arrays/strings if not applicable):\n\
```json\n\
{{\n\
  \"overviewType\": \"paper\",\n\
  \"abstractText\": \"...\",\n\
  \"keyConcepts\": [\"concept1\", \"concept2\"],\n\
  \"methods\": [\"method1\", \"method2\"],\n\
  \"contributions\": [\"contribution1\", \"contribution2\"],\n\
  \"limitations\": [\"limitation1\", \"limitation2\"],\n\
  \"tlDr\": \"one-sentence summary\",\n\
  \"sections\": [\n\
    {{\"title\": \"Introduction\", \"summary\": \"...\"}},\n\
    {{\"title\": \"Methods\", \"summary\": \"...\"}}\n\
  ],\n\
  \"metadata\": {{\"authors\": [], \"publishedDate\": \"\", \"doi\": \"\", \"arxivId\": \"\"}}\n\
}}\n\
```\n\n\
Return ONLY the JSON object, no additional text.",
        title = doc.title,
        full_text = full_text
    );

    Ok(prompt)
}
