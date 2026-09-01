// SPDX-License-Identifier: AGPL-3.0-only

//! LightRAG 知识图谱增强命令
//!
//! 跨文档实体抽取 + 图查询增强 RAG context

use crate::app_state::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};

/// 手动实体抽取串行限流：LLM 抽取是慢操作（每次调 LLM + 嵌入），
/// 并发触发会把共享连接池占满（曾导致 Connection pool timed out，
/// 所有 DB 命令瞬时失败）。信号量限制并发为 1，多余触发排队等待。
static MANUAL_EXTRACT_SEMAPHORE: tokio::sync::OnceCell<std::sync::Arc<tokio::sync::Semaphore>> =
    tokio::sync::OnceCell::const_new();
async fn manual_extract_permit() -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    let sem = MANUAL_EXTRACT_SEMAPHORE
        .get_or_init(|| async { std::sync::Arc::new(tokio::sync::Semaphore::new(1)) })
        .await
        .clone();
    sem.acquire_owned().await.map_err(|e| format!("实体抽取并发控制失败: {e}"))
}
use axagent_agent_macro::agent_command;
use axagent_harness::core_error::AxAgentError;
use axagent_harness::prompt_provider::PromptLang;
use axagent_harness::util_fns::truncate_to_char_boundary;
use axagent_harness::{
    ExtractEntitiesResult, ExtractedEntity, ExtractedRelation, GraphEnhancedSearchInput,
    GraphEnhancedSearchResult,
};
use tauri::State;

/// 单次 LLM 抽取的最大文本长度（字节）。超过则按 char boundary 截断，
/// 避免 context 爆炸。16000 字节约对应 4-8k tokens，留出空间给 system prompt + JSON 输出。
const MAX_EXTRACT_TEXT_BYTES: usize = 16_000;

/// 单次命令最多处理的文档数（防止 LLM 调用过多导致超时/费用失控）。
const MAX_DOCUMENTS_PER_CALL: usize = 20;

/// 将 `AxAgentError` 转换为统一错误响应字符串（不可恢复分类）。
fn err_to_string(e: AxAgentError) -> String {
    String::from(ErrorResponse::from_error(e, ErrorCategory::Unrecoverable))
}

/// 图查询增强：根据查询关键词检索 KB 内实体，扩展 1-hop 邻居关系，
/// 返回可直接注入 RAG context 的文本片段。
#[agent_command(domain = knowledge, safety = Safe, call_mode = StateInput, description = "知识图谱增强搜索")]
#[tauri::command]
pub async fn graph_enhanced_search(
    state: State<'_, AppState>,
    input: GraphEnhancedSearchInput,
) -> Result<GraphEnhancedSearchResult, String> {
    let top_k = input.top_k.unwrap_or(10).min(50);
    let include_neighbors = input.include_neighbors.unwrap_or(true);
    let chunks = axagent_dao::repo::knowledge_graph::graph_enhanced_search(
        state.harness.db(),
        &input.knowledge_base_id,
        &input.query,
        top_k,
        include_neighbors,
    )
    .await
    .map_err(err_to_string)?;

    let total_hits = chunks.len();
    let context_text = axagent_dao::repo::knowledge_graph::build_graph_context_text(
        &input.knowledge_base_id,
        &chunks,
    );

    Ok(GraphEnhancedSearchResult { entities: chunks, context_text, total_hits })
}

/// 跨文档实体抽取：由调用方传入 chunks 内容与已存在的实体列表，
/// 由 LLM 抽取实体/关系并写入 DB。
///
/// 注意：本命令只负责 DB 写入；LLM 抽取由 wiring 层（agent crate）完成，
/// 调用方需要先把 chunks 通过 LLM 抽取为 [`ExtractedEntity`] / [`ExtractedRelation`]，
/// 再调用本命令持久化。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "批量写入实体和关系")]
#[tauri::command]
pub async fn batch_upsert_entities_and_relations(
    state: State<'_, AppState>,
    knowledge_base_id: String,
    entities: Vec<ExtractedEntity>,
    relations: Vec<ExtractedRelation>,
) -> Result<ExtractEntitiesResult, String> {
    axagent_dao::repo::knowledge_graph::batch_upsert_entities_and_relations(
        state.harness.db(),
        &knowledge_base_id,
        "knowledge_base",
        "",
        entities,
        relations,
    )
    .await
    .map_err(err_to_string)
}

/// 跨文档实体抽取（全流程）：传入 KB ID 与文档 ID 列表，
/// 自动从 vector_store 加载 chunks，调用 LLM 抽取，写入 DB。
///
/// 流程：
/// 1. 限制 `document_ids.len() <= 20`
/// 2. 对每个 document_id，从 `state.vector_store` 加载 chunks
/// 3. 拼接 chunk 内容，截断到 16k 字节防止 context 爆炸
/// 4. 通过 `build_llm_bridge_from_db` 构建 LLM Bridge
/// 5. 使用 kit::prompts 中的 `entity_extraction` 提示词模板调用 LLM
/// 6. 解析 JSON（清理 markdown fences）→ `ExtractedEntity` / `ExtractedRelation`
/// 7. 调用 `batch_upsert_entities_and_relations` 写入 DB
/// 8. 返回 `ExtractEntitiesResult`
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "从文档抽取实体")]
#[tauri::command]
pub async fn extract_entities_from_documents(
    state: State<'_, AppState>,
    knowledge_base_id: String,
    document_ids: Vec<String>,
) -> Result<ExtractEntitiesResult, String> {
    // 1. 文档数限制
    if document_ids.is_empty() {
        return Ok(ExtractEntitiesResult {
            new_entities: Vec::new(),
            updated_entities: Vec::new(),
            new_relations: Vec::new(),
            skipped_chunks: 0,
            elapsed_ms: 0,
        });
    }
    if document_ids.len() > MAX_DOCUMENTS_PER_CALL {
        return Err(ErrorResponse::from_error(
            format!(
                "document_ids 数量超限：{} > {}，请分批调用",
                document_ids.len(),
                MAX_DOCUMENTS_PER_CALL
            ),
            ErrorCategory::Unrecoverable,
        )
        .to_string());
    }

    let started = std::time::Instant::now();
    let db = state.harness.db();
    let collection_id = format!("kb_{}", knowledge_base_id);

    // 2. 加载所有文档的 chunks 并拼接
    let mut all_text = String::new();
    let mut skipped_chunks: u32 = 0;
    for doc_id in &document_ids {
        let chunks = state
            .vector_store
            .list_document_chunks(&collection_id, doc_id)
            .await
            .map_err(err_to_string)?;
        if chunks.is_empty() {
            skipped_chunks += 1;
            continue;
        }
        for chunk in chunks {
            all_text.push_str(&chunk.content);
            all_text.push_str("\n\n");
            // 截断到上限，避免单次 LLM 调用文本过长
            if all_text.len() >= MAX_EXTRACT_TEXT_BYTES {
                let truncated = truncate_to_char_boundary(&all_text, MAX_EXTRACT_TEXT_BYTES);
                all_text = truncated.to_string();
                break;
            }
        }
        if all_text.len() >= MAX_EXTRACT_TEXT_BYTES {
            break;
        }
    }

    if all_text.trim().is_empty() {
        return Ok(ExtractEntitiesResult {
            new_entities: Vec::new(),
            updated_entities: Vec::new(),
            new_relations: Vec::new(),
            skipped_chunks,
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
    }

    // 3. 加载已有实体列表（用于 LLM 提示，便于去重/合并）
    let existing_entities =
        axagent_dao::repo::knowledge_graph::get_all_entities_by_kb(db, &knowledge_base_id)
            .await
            .map_err(err_to_string)?;
    let existing_names: Vec<String> =
        existing_entities.iter().take(50).map(|e| e.name.clone()).collect();

    // 4. 构建提示词（zh-CN 为默认/回退语言）
    let system_prompt = axagent_kit::prompts::PromptRegistry::get(
        "entity_extraction.system_prompt",
        PromptLang::ZhCN,
    );
    let user_template = axagent_kit::prompts::PromptRegistry::get(
        "entity_extraction.user_template",
        PromptLang::ZhCN,
    );
    // 在 user 提示中加入已有实体提示，便于 LLM 复用而非重复抽取
    let existing_hint = if existing_names.is_empty() {
        String::new()
    } else {
        format!(
            "\n\n[已存在的实体名称（请勿重复抽取，可在关系中引用）]\n{}",
            existing_names.join(", ")
        )
    };
    let user_prompt = user_template.replace("{0}", &format!("{}{}", all_text, existing_hint));

    // 5. 构建 LLM Bridge
    let bridge = axagent_runtime::llm_bridge::build_llm_bridge_from_db(state.harness.master_key())
        .await
        .ok_or_else(|| {
            ErrorResponse::from_error(
                "未找到启用的 LLM Provider，无法执行实体抽取",
                ErrorCategory::Unrecoverable,
            )
        })?;

    // 6. 调用 LLM
    let llm_response = bridge.call_llm(system_prompt, &user_prompt).await.map_err(|e| {
        ErrorResponse::from_error(
            format!("LLM 实体抽取调用失败：{}", e),
            ErrorCategory::Unrecoverable,
        )
    })?;

    // 7. 解析 JSON（清理 markdown fences）
    let (entities, relations) = parse_entity_extraction_response(&llm_response)?;

    // 8. 写入 DB
    let result = axagent_dao::repo::knowledge_graph::batch_upsert_entities_and_relations(
        db,
        &knowledge_base_id,
        "knowledge_base",
        "",
        entities,
        relations,
    )
    .await
    .map_err(err_to_string)?;

    // 合并 skipped_chunks 到返回值
    Ok(ExtractEntitiesResult {
        new_entities: result.new_entities,
        updated_entities: result.updated_entities,
        new_relations: result.new_relations,
        skipped_chunks: result.skipped_chunks + skipped_chunks,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

/// 解析 LLM 实体抽取响应。
///
/// LLM 通常返回 JSON 对象 `{"entities": [...], "relations": [...]}`，
/// 但有时会包裹 markdown fences（```json ... ```）或前后多余文本。
/// 本函数：
/// 1. 去除 markdown fences
/// 2. 用 serde_json 解析为 `EntityExtractionPayload`
/// 3. 转换为 `ExtractedEntity` / `ExtractedRelation` 列表
///
/// 解析失败时返回空列表（不报错，因为 LLM 可能返回 "no entities found"）。
pub(crate) fn parse_entity_extraction_response(
    response: &str,
) -> Result<(Vec<ExtractedEntity>, Vec<ExtractedRelation>), String> {
    // 1. 清理 markdown fences
    let cleaned = strip_markdown_fences(response);

    // 2. 尝试解析 JSON 对象
    let payload: EntityExtractionPayload = match serde_json::from_str(&cleaned) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = %e,
                response_preview = %cleaned.chars().take(200).collect::<String>(),
                "LLM 实体抽取响应非合法 JSON，跳过本次抽取"
            );
            // 解析失败视为无实体抽取，返回空列表（不报错）
            return Ok((Vec::new(), Vec::new()));
        },
    };

    // 3. 转换为 harness 类型
    let entities = payload
        .entities
        .into_iter()
        .filter(|e| !e.name.is_empty())
        .map(|e| ExtractedEntity {
            name: e.name,
            entity_type: e.entity_type.unwrap_or_else(|| "concept".to_string()),
            aliases: e.aliases.unwrap_or_default(),
            description: e.description.unwrap_or_default(),
        })
        .collect();

    let relations = payload
        .relations
        .into_iter()
        .filter(|r| !r.source.is_empty() && !r.target.is_empty())
        .map(|r| ExtractedRelation {
            source: r.source,
            target: r.target,
            relation_type: r.relation.unwrap_or_else(|| "mentions".to_string()),
        })
        .collect();

    Ok((entities, relations))
}

/// 去除 markdown fences（```json ... ``` 或 ``` ... ```）。
fn strip_markdown_fences(s: &str) -> String {
    let trimmed = s.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    // 去掉开头的 ```（可能带语言标识如 ```json）
    let after_open = trimmed.trim_start_matches("```");
    let after_open = after_open.trim_start_matches("json").trim_start_matches("JSON");
    // 去掉结尾的 ```
    let after_close = after_open.trim_end_matches("```");
    after_close.trim().to_string()
}

/// LLM 实体抽取响应的 JSON payload 结构。
#[derive(Debug, serde::Deserialize)]
struct EntityExtractionPayload {
    #[serde(default)]
    entities: Vec<EntityPayload>,
    #[serde(default)]
    relations: Vec<RelationPayload>,
}

#[derive(Debug, serde::Deserialize)]
struct EntityPayload {
    name: String,
    #[serde(default, alias = "type")]
    entity_type: Option<String>,
    #[serde(default)]
    aliases: Option<Vec<String>>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct RelationPayload {
    source: String,
    target: String,
    #[serde(default)]
    relation: Option<String>,
}

/// P1-1: Wiki note 实体抽取 — 将指定 vault 的所有 notes 纳入知识图谱抽取
///
/// 流程：
/// 1. 从 vault 加载 notes（可指定 note_ids 增量抽取，否则全量）
/// 2. 逐笔记拼接内容，按 16KB 分批防止 context 爆炸（单笔记多字节安全截断）
/// 3. 加载已有实体列表（用于去重）
/// 4. 构建 LLM Bridge 并按批调用实体抽取
/// 5. 解析 JSON 响应
/// 6. 写入知识图谱，落 v113 来源字段 `("wiki", vault_id)`，与真实 KB 实体可区分
///
/// 与 `extract_entities_from_documents` 的区别：
/// - 输入是 vault_id 而非 knowledge_base_id
/// - 数据源是笔记表而非 vector_store chunks
/// - 输出写入到 vault 对应的知识图谱（来源标记 source_type="wiki"）
///
/// 核心逻辑与索引队列 worker 共用 `run_wiki_entity_extraction_core`（R6：
/// 笔记索引完成后自动入队抽取，本命令为手动触发入口）。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "从Wiki笔记抽取实体")]
#[tauri::command]
pub async fn extract_entities_from_wiki(
    state: State<'_, AppState>,
    vault_id: String,
    note_ids: Option<Vec<String>>,
) -> Result<ExtractEntitiesResult, String> {
    // 串行限流：与 extract_entities_for_kb 共用，并发实体抽取会占满共享连接池
    // （曾触发 Connection pool timed out）。拿不到 permit 时排队等待，不拒绝。
    let _permit = manual_extract_permit().await?;

    crate::index_queue::run_wiki_entity_extraction_core(
        state.harness.db(),
        state.harness.master_key(),
        &vault_id,
        note_ids,
    )
    .await
    .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::Unrecoverable).to_string())
}

/// 手动触发知识库实体抽取（同步执行，返回抽取计数供前端展示）
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "手动触发知识库实体抽取")]
#[tauri::command]
pub async fn extract_entities_for_kb(
    state: State<'_, AppState>,
    knowledge_base_id: String,
) -> Result<ExtractEntitiesResult, String> {
    use axagent_dao::repo::index_jobs as jobs;

    let _ =
        axagent_dao::repo::knowledge::get_knowledge_base(state.harness.db(), &knowledge_base_id)
            .await
            .map_err(|e| {
                ErrorResponse::from_error(
                    format!("知识库不存在: {}", e),
                    ErrorCategory::Unrecoverable,
                )
            })?;

    // 插入 job 记录用于历史追踪 / 失败标记（同步执行，无需经队列 worker 流转）
    let metadata = serde_json::json!({
        "manual_extract": true,
    });
    let input = jobs::CreateIndexJobInput {
        job_type: jobs::JOB_TYPE_EXTRACT_ENTITIES.to_string(),
        container_type: "KnowledgeBase".to_string(),
        container_id: knowledge_base_id.clone(),
        item_id: knowledge_base_id.clone(),
        max_retries: Some(1),
        priority: Some(0),
        metadata: Some(serde_json::to_string(&metadata).unwrap_or_default()),
    };
    let _ = jobs::enqueue_job(state.harness.db(), input).await.map_err(|e| {
        ErrorResponse::from_error(
            format!("记录实体抽取任务失败: {}", e),
            ErrorCategory::Unrecoverable,
        )
    })?;

    // 串行限流：并发实体抽取会占满共享连接池（曾触发 Connection pool timed out）。
    // 拿不到 permit 时排队等待，不拒绝。
    let _permit = manual_extract_permit().await?;

    // 同步执行核心抽取逻辑并返回计数
    crate::index_queue::run_entity_extraction_core(
        state.harness.db(),
        &state.vector_store,
        state.harness.master_key(),
        &knowledge_base_id,
    )
    .await
    .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::Unrecoverable).into())
}

/// P1-3: 跨源实体合并 — 按 name+entity_type 合并 Wiki/KB/Memory 中的重复实体
///
/// 解决三套实体系统各自为政的问题。
/// 合并策略：保留最早创建的实体作为"主实体"，合并其他实体的 aliases、description、关系引用。
#[agent_command(domain = knowledge, safety = Caution, call_mode = StateOnly, description = "合并重复实体")]
#[tauri::command]
pub async fn merge_duplicate_entities(
    state: State<'_, AppState>,
) -> Result<axagent_dao::repo::knowledge_graph::MergeEntitiesResult, String> {
    axagent_dao::repo::knowledge_graph::merge_duplicate_entities_across_all(state.harness.db())
        .await
        .map_err(|e| err_to_string(AxAgentError::internal(e.to_string())))
}

/// P2-1: Memory → Knowledge 实体回流命令
///
/// 将 Memory 中高重要性（>= importance_threshold）的条目
/// 批量转换为知识图谱实体，解决三套实体系统各自为政的问题。
///
/// 返回转换统计：读取条目数、成功转换数、失败数
#[derive(Debug, serde::Serialize)]
pub struct MemoryToKnowledgeResult {
    pub items_read: usize,
    pub entities_created: usize,
    pub failures: usize,
}

#[agent_command(domain = knowledge, safety = Caution, call_mode = StateInput, description = "同步记忆到知识图谱")]
#[tauri::command]
pub async fn sync_memory_to_knowledge_graph(
    state: State<'_, AppState>,
    importance_threshold: Option<f64>,
    max_items: Option<u32>,
) -> Result<MemoryToKnowledgeResult, String> {
    let threshold = importance_threshold.unwrap_or(0.7);
    let max = max_items.unwrap_or(100);

    let items = axagent_dao::repo::memory::list_high_importance_items(
        state.harness.db(),
        Some(threshold),
        Some(max),
    )
    .await
    .map_err(|e| err_to_string(AxAgentError::internal(e.to_string())))?;

    let items_read = items.len();
    let mut entities_created = 0usize;
    let mut failures = 0usize;

    for item in &items {
        let kb_id = if item.namespace_id.is_empty() {
            "memory_default".to_string()
        } else {
            item.namespace_id.clone()
        };
        let name: String = item.content.chars().take(100).collect();
        let confidence = (item.importance).min(1.0);

        match axagent_dao::repo::knowledge_graph::upsert_entity(
            state.harness.db(),
            &kb_id,
            &name,
            "memory_item",
            "[]",
            confidence,
            None,
            None,
        )
        .await
        {
            Ok(_) => entities_created += 1,
            Err(e) => {
                tracing::debug!("[sync_memory_to_knowledge] 转换失败 item={}: {}", item.id, e);
                failures += 1;
            },
        }
    }

    tracing::info!(
        "[sync_memory_to_knowledge] 完成：读取 {} 条，创建 {} 实体，{} 失败",
        items_read,
        entities_created,
        failures
    );

    Ok(MemoryToKnowledgeResult { items_read, entities_created, failures })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_fences_plain_json() {
        let input = r#"```json
{"entities": [], "relations": []}
```"#;
        let cleaned = strip_markdown_fences(input);
        assert_eq!(cleaned, r#"{"entities": [], "relations": []}"#);
    }

    #[test]
    fn test_strip_markdown_fences_no_fences() {
        let input = r#"{"entities": [], "relations": []}"#;
        let cleaned = strip_markdown_fences(input);
        assert_eq!(cleaned, input);
    }

    #[test]
    fn test_parse_entity_extraction_response_valid() {
        let response = r#"```json
        {
          "entities": [
            {"name": "Rust", "type": "technology", "description": "systems programming language"},
            {"name": "Tauri", "type": "tool", "aliases": ["TauriApp"]}
          ],
          "relations": [
            {"source": "Tauri", "target": "Rust", "relation": "uses"}
          ]
        }
        ```"#;
        let (entities, relations) = parse_entity_extraction_response(response).expect("测试应成功");
        assert_eq!(entities.len(), 2);
        assert_eq!(entities[0].name, "Rust");
        assert_eq!(entities[0].entity_type, "technology");
        assert_eq!(entities[1].aliases, vec!["TauriApp".to_string()]);
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source, "Tauri");
        assert_eq!(relations[0].target, "Rust");
        assert_eq!(relations[0].relation_type, "uses");
    }

    #[test]
    fn test_parse_entity_extraction_response_invalid_returns_empty() {
        let response = "Sorry, I cannot help with that.";
        let (entities, relations) = parse_entity_extraction_response(response).expect("测试应成功");
        assert!(entities.is_empty());
        assert!(relations.is_empty());
    }

    #[test]
    fn test_parse_entity_extraction_response_filters_empty_names() {
        let response = r#"{
            "entities": [
                {"name": "Valid"},
                {"name": ""}
            ],
            "relations": [
                {"source": "", "target": "B"},
                {"source": "A", "target": "B"}
            ]
        }"#;
        let (entities, relations) = parse_entity_extraction_response(response).expect("测试应成功");
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].name, "Valid");
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source, "A");
    }

    #[test]
    fn test_parse_entity_extraction_response_defaults_missing_fields() {
        let response = r#"{
            "entities": [{"name": "X"}],
            "relations": [{"source": "A", "target": "B"}]
        }"#;
        let (entities, relations) = parse_entity_extraction_response(response).expect("测试应成功");
        assert_eq!(entities[0].entity_type, "concept");
        assert!(entities[0].aliases.is_empty());
        assert!(entities[0].description.is_empty());
        assert_eq!(relations[0].relation_type, "mentions");
    }
}
