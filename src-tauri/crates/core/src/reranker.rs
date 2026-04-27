use serde::{Deserialize, Serialize};

use crate::hybrid_search::HybridSearchResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankConfig {
    pub enabled: bool,
    pub model: Option<String>,
    pub top_n: usize,
    pub score_threshold: Option<f32>,
}

impl Default for RerankConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: None,
            top_n: 5,
            score_threshold: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RerankedResult {
    pub id: String,
    pub document_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub original_score: f32,
    pub rerank_score: f32,
    pub rerank_reason: Option<String>,
}

pub struct Reranker;

impl Reranker {
    pub fn new() -> Self {
        Self
    }

    pub fn rerank(
        &self,
        query: &str,
        results: Vec<HybridSearchResult>,
        config: &RerankConfig,
    ) -> Vec<RerankedResult> {
        if !config.enabled || results.is_empty() {
            return results
                .into_iter()
                .map(|r| RerankedResult {
                    id: r.id,
                    document_id: r.document_id,
                    chunk_index: r.chunk_index,
                    content: r.content,
                    original_score: r.combined_score,
                    rerank_score: r.combined_score,
                    rerank_reason: None,
                })
                .collect();
        }

        let mut scored: Vec<RerankedResult> = results
            .into_iter()
            .map(|r| {
                let rerank_score = self.calculate_rerank_score(query, &r);
                let rerank_reason = self.explain_score(query, &r, rerank_score);
                RerankedResult {
                    id: r.id,
                    document_id: r.document_id,
                    chunk_index: r.chunk_index,
                    content: r.content,
                    original_score: r.combined_score,
                    rerank_score,
                    rerank_reason: Some(rerank_reason),
                }
            })
            .collect();

        scored.sort_by(|a, b| b.rerank_score.partial_cmp(&a.rerank_score).unwrap());

        let mut result: Vec<RerankedResult> = scored
            .into_iter()
            .filter(|r| {
                if let Some(threshold) = config.score_threshold {
                    r.rerank_score >= threshold
                } else {
                    true
                }
            })
            .take(config.top_n)
            .collect();

        for (i, r) in result.iter_mut().enumerate() {
            if r.rerank_reason.is_none() {
                r.rerank_reason = Some(format!("Ranked #{} by semantic similarity", i + 1));
            }
        }

        result
    }

    fn calculate_rerank_score(&self, query: &str, result: &HybridSearchResult) -> f32 {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = result.content.to_lowercase();

        let exact_matches = query_terms
            .iter()
            .filter(|term| content_lower.contains(*term))
            .count() as f32;
        let exact_match_score = exact_matches / query_terms.len().max(1) as f32;

        let query_word_count = query_terms.len();
        let content_word_count = result.content.split_whitespace().count().max(1);
        let coverage = query_terms
            .iter()
            .filter(|term| content_lower.split_whitespace().any(|w| w.contains(*term)))
            .count() as f32
            / query_word_count.max(1) as f32;

        let position_score = self.calculate_position_score(query, &result.content);

        let length_penalty =
            self.calculate_length_penalty(result.content.len(), content_word_count);

        let original_contribution = result.combined_score * 0.3;
        let exact_contribution = exact_match_score * 0.25;
        let coverage_contribution = coverage * 0.2;
        let position_contribution = position_score * 0.15;
        let length_contribution = length_penalty * 0.1;

        original_contribution
            + exact_contribution
            + coverage_contribution
            + position_contribution
            + length_contribution
    }

    fn calculate_position_score(&self, query: &str, content: &str) -> f32 {
        let content_lower = content.to_lowercase();
        let query_lower = query.to_lowercase();

        let first_match_pos = content_lower
            .find(&query_lower)
            .map(|pos| pos as f32 / content_lower.len() as f32)
            .unwrap_or(1.0);

        1.0 - first_match_pos
    }

    fn calculate_length_penalty(&self, _char_count: usize, word_count: usize) -> f32 {
        let ideal_words = 100;
        let ratio = word_count as f32 / ideal_words as f32;

        if ratio < 1.0 {
            ratio
        } else {
            1.0 / ratio.sqrt()
        }
    }

    fn explain_score(&self, query: &str, result: &HybridSearchResult, rerank_score: f32) -> String {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = result.content.to_lowercase();

        let matches: Vec<&str> = query_terms
            .iter()
            .filter(|term| content_lower.contains(*term))
            .copied()
            .collect();

        if matches.is_empty() {
            format!(
                "No exact term matches, semantic score {:.3}",
                result.combined_score
            )
        } else if matches.len() == query_terms.len() {
            format!("All query terms matched, score {:.3}", rerank_score)
        } else {
            format!(
                "{} of {} query terms matched, score {:.3}",
                matches.len(),
                query_terms.len(),
                rerank_score
            )
        }
    }
}

impl Default for Reranker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reranker_basic() {
        let reranker = Reranker::new();
        let config = RerankConfig::default();

        let results = vec![
            HybridSearchResult {
                id: "1".to_string(),
                document_id: "doc1".to_string(),
                chunk_index: 0,
                content: "The quick brown fox jumps over the lazy dog".to_string(),
                vector_score: Some(0.9),
                bm25_score: Some(0.5),
                combined_score: 0.8,
            },
            HybridSearchResult {
                id: "2".to_string(),
                document_id: "doc2".to_string(),
                chunk_index: 0,
                content: "A lazy cat sleeps on the windowsill".to_string(),
                vector_score: Some(0.7),
                bm25_score: Some(0.3),
                combined_score: 0.6,
            },
        ];

        let reranked = reranker.rerank("lazy dog", results, &config);

        assert_eq!(reranked.len(), 2);
        assert_eq!(reranked[0].id, "1");
    }

    #[test]
    fn test_reranker_empty_results() {
        let reranker = Reranker::new();
        let config = RerankConfig::default();

        let reranked = reranker.rerank("test query", vec![], &config);
        assert!(reranked.is_empty());
    }

    #[test]
    fn test_reranker_disabled() {
        let reranker = Reranker::new();
        let mut config = RerankConfig::default();
        config.enabled = false;

        let results = vec![HybridSearchResult {
            id: "1".to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: "Test content".to_string(),
            vector_score: Some(0.9),
            bm25_score: Some(0.5),
            combined_score: 0.8,
        }];

        let reranked = reranker.rerank("test", results, &config);
        assert_eq!(reranked.len(), 1);
        assert_eq!(reranked[0].rerank_score, 0.8);
    }

    #[test]
    fn test_reranker_top_n_limit() {
        let reranker = Reranker::new();
        let mut config = RerankConfig::default();
        config.top_n = 2;

        let results = vec![
            HybridSearchResult {
                id: "1".to_string(),
                document_id: "doc1".to_string(),
                chunk_index: 0,
                content: "Result 1".to_string(),
                vector_score: Some(0.5),
                bm25_score: Some(0.5),
                combined_score: 0.5,
            },
            HybridSearchResult {
                id: "2".to_string(),
                document_id: "doc2".to_string(),
                chunk_index: 0,
                content: "Result 2".to_string(),
                vector_score: Some(0.7),
                bm25_score: Some(0.7),
                combined_score: 0.7,
            },
            HybridSearchResult {
                id: "3".to_string(),
                document_id: "doc3".to_string(),
                chunk_index: 0,
                content: "Result 3".to_string(),
                vector_score: Some(0.9),
                bm25_score: Some(0.9),
                combined_score: 0.9,
            },
        ];

        let reranked = reranker.rerank("test", results, &config);
        assert_eq!(reranked.len(), 2);
    }

    #[test]
    fn test_reranker_all_terms_matched() {
        let reranker = Reranker::new();
        let config = RerankConfig::default();

        let results = vec![HybridSearchResult {
            id: "1".to_string(),
            document_id: "doc1".to_string(),
            chunk_index: 0,
            content: "The quick brown fox".to_string(),
            vector_score: Some(0.9),
            bm25_score: Some(0.5),
            combined_score: 0.8,
        }];

        let reranked = reranker.rerank("quick brown fox", results, &config);
        assert!(reranked[0]
            .rerank_reason
            .as_ref()
            .unwrap()
            .contains("All query terms matched"));
    }
}
