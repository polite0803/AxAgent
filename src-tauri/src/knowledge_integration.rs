use axagent_core::rag::KnowledgeContainer;
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationInsight {
    pub insight_type: InsightType,
    pub title: String,
    pub description: String,
    pub source_ids: Vec<SourceRef>,
    pub confidence: f64,
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    Duplicate,
    Stale,
    Related,
    Gap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRef {
    pub container_id: String,
    pub container_type: String,
    pub item_id: String,
    pub item_title: String,
}

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
                            let title_a = if content_a.len() >= 50 {
                                &content_a[..50]
                            } else {
                                content_a.as_str()
                            };
                            let title_b = if content_b.len() >= 50 {
                                &content_b[..50]
                            } else {
                                content_b.as_str()
                            };
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
                                        container_type: format!(
                                            "{:?}",
                                            containers[i].container_type
                                        ),
                                        item_id: id_a.clone(),
                                        item_title: title_a.to_string(),
                                    },
                                    SourceRef {
                                        container_id: containers[j].id.clone(),
                                        container_type: format!(
                                            "{:?}",
                                            containers[j].container_type
                                        ),
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
        let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 {
            return 0.0;
        }

        intersection as f64 / union as f64
    }
}

#[tauri::command]
pub async fn analyze_knowledge_integration(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<IntegrationInsight>, String> {
    let mut containers = Vec::new();

    let kbs = axagent_core::repo::knowledge::list_knowledge_bases(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for kb in &kbs {
        containers.push(KnowledgeContainer::from_knowledge_base(kb));
    }

    let namespaces = axagent_core::repo::memory::list_namespaces(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
    for ns in &namespaces {
        containers.push(KnowledgeContainer::from_memory_ns(ns));
    }

    let wikis = axagent_core::repo::wiki::list_wikis(state.harness.db())
        .await
        .map_err(|e| e.to_string())?;
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
                &ep,
                vec![query.clone()],
                dimensions,
            )
            .await;

            match embed_result {
                Ok(response) => {
                    if let Some(query_embedding) = response.embeddings.into_iter().next() {
                        match state
                            .vector_store
                            .search(&collection_name, query_embedding, 5)
                            .await
                        {
                            Ok(results) => results
                                .into_iter()
                                .map(|r| (r.id, r.content, r.score))
                                .collect(),
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
