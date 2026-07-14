// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_harness::repo_dtos::{InsightType, IntegrationInsight, SourceRef};
use axagent_search::rag::KnowledgeContainer;
use tauri::State;

pub struct KnowledgeIntegrationEngine;

impl KnowledgeIntegrationEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_duplicates(
        &self,
        containers: &[KnowledgeContainer],
        search_results: &[Vec<(String, String, f32)>],
    ) -> Vec<IntegrationInsight> {
        let mut insights = Vec::new();

        for i in 0..containers.len() {
            for j in (i + 1)..containers.len() {
                for (id_a, content_a, _) in &search_results[i] {
                    for (id_b, content_b, _) in &search_results[j] {
                        let similarity = self.calculate_text_similarity(content_a, content_b);
                        if similarity > 0.8 {
                            // 使用按字符截断，避免对多字节 UTF-8（中文/emoji）按字节切片导致 panic
                            let title_a = truncate_to_chars(content_a, 50);
                            let title_b = truncate_to_chars(content_b, 50);
                            insights.push(IntegrationInsight {
                                insight_type: InsightType::Duplicate,
                                title: "Potential duplicate knowledge".to_string(),
                                description: format!(
                                    "Similar content found in {} and {}",
                                    containers[i].name, containers[j].name
                                ),
                                source_ids: vec![
                                    SourceRef {
                                        container_id: containers[i].id.clone(),
                                        // 用稳定的 container_type_str() 而非 Debug 格式化，
                                        // 避免重命名 enum 变体时破坏前端契约
                                        container_type: containers[i]
                                            .container_type_str()
                                            .to_string(),
                                        item_id: id_a.clone(),
                                        item_title: title_a.to_string(),
                                    },
                                    SourceRef {
                                        container_id: containers[j].id.clone(),
                                        container_type: containers[j]
                                            .container_type_str()
                                            .to_string(),
                                        item_id: id_b.clone(),
                                        item_title: title_b.to_string(),
                                    },
                                ],
                                confidence: similarity,
                                suggested_action: Some(
                                    "Consider merging or deduplicating".to_string(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        insights
    }

    fn calculate_text_similarity(&self, a: &str, b: &str) -> f64 {
        let tokens_a = tokenize(a);
        let tokens_b = tokenize(b);

        if tokens_a.is_empty() || tokens_b.is_empty() {
            return 0.0;
        }

        let set_a: std::collections::HashSet<String> = tokens_a.into_iter().collect();
        let set_b: std::collections::HashSet<String> = tokens_b.into_iter().collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }
}

/// 按字符数截断字符串，避免对多字节 UTF-8 按字节切片导致 `byte index not a char boundary` panic。
fn truncate_to_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars).collect()
    }
}

/// 分词：有空格的文本按空白切分；无空格（如中文/日文）按字符切分，
/// 使 Jaccard 相似度对 CJK 也有效（原本整段被视为一个词，相似度恒为 0）。
fn tokenize(s: &str) -> Vec<String> {
    let ws: Vec<&str> = s.split_whitespace().collect();
    if !ws.is_empty() {
        return ws.into_iter().map(|w| w.to_string()).collect();
    }
    s.chars().filter(|c| !c.is_whitespace()).map(|c| c.to_string()).collect()
}

#[tauri::command]
pub async fn analyze_knowledge_integration(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<IntegrationInsight>, String> {
    let mut containers = Vec::new();

    let kbs = axagent_dao::repo::knowledge::list_knowledge_bases(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for kb in &kbs {
        containers.push(KnowledgeContainer::from_knowledge_base(kb));
    }

    let namespaces = axagent_dao::repo::memory::list_namespaces(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for ns in &namespaces {
        containers.push(KnowledgeContainer::from_memory_ns(ns));
    }

    let wikis =
        axagent_dao::repo::wiki::list_wikis(state.harness.db()).await.map_err(|e| e.to_string())?;
    for wiki in &wikis {
        containers.push(KnowledgeContainer::from_wiki(wiki));
    }

    if containers.is_empty() {
        return Ok(vec![]);
    }

    let mut all_search_results: Vec<Vec<(String, String, f32)>> = Vec::new();

    for container in &containers {
        // collection_name returns "kb_{id}", "mem_{id}", or "wiki_{id}" — this matches
        // the format used by rag::collection_id() and is valid for
        // VectorStore::validated_collection_name() which prepends "vec_".
        let collection_name = container.collection_name();
        let embedding_provider = container.embedding_provider.clone();
        let dimensions = container.embedding_dimensions.map(|d| d as usize);

        let entries = if let Some(ep) = embedding_provider {
            let embed_result = crate::indexing::generate_embeddings(
                state.harness.db(),
                state.harness.master_key(),
                state.harness.provider_registry(),
                &ep,
                vec![query.clone()],
                dimensions,
            )
            .await;

            match embed_result {
                Ok(response) => {
                    if let Some(query_embedding) = response.embeddings.into_iter().next() {
                        match state.vector_store.search(&collection_name, query_embedding, 5).await
                        {
                            Ok(results) => {
                                results.into_iter().map(|r| (r.id, r.content, r.score)).collect()
                            },
                            Err(_) => vec![],
                        }
                    } else {
                        vec![]
                    }
                },
                Err(_) => vec![],
            }
        } else {
            vec![]
        };

        all_search_results.push(entries);
    }

    let engine = KnowledgeIntegrationEngine::new();
    let insights = engine.detect_duplicates(&containers, &all_search_results);

    Ok(insights)
}
