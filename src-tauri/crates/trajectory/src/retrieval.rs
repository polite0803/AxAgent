//! Cross-session retrieval module with BM25 ranking
//!
//! Replaces TypeScript `CrossSessionRetriever.ts` with Rust implementation.

use crate::TrajectoryStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrievalResult {
    pub id: String,
    #[serde(rename = "type")]
    pub result_type: RetrievalType,
    pub content: String,
    pub relevance: f64,
    pub session_id: Option<String>,
    pub timestamp: i64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RetrievalType {
    Trajectory,
    Memory,
    Skill,
    Pattern,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSessionQuery {
    pub query: String,
    pub intent: Option<String>,
    pub entities: Vec<String>,
    pub time_range: Option<TimeRange>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone)]
pub struct BM25Config {
    pub k1: f64,
    pub b: f64,
}

impl Default for BM25Config {
    fn default() -> Self {
        Self { k1: 1.5, b: 0.75 }
    }
}

/// Cross-session memory retrieval using BM25 ranking.
///
/// Not currently integrated into the conversation flow.
/// RAG retrieval is handled by `axagent_core::rag` and `crate::indexing::collect_rag_context`.
/// Retained for potential future use in cross-session pattern learning.
#[allow(dead_code)]
pub struct CrossSessionRetriever {
    storage: Arc<TrajectoryStorage>,
    stop_words: Vec<String>,
    config: BM25Config,
}

impl CrossSessionRetriever {
    pub fn new(storage: Arc<TrajectoryStorage>) -> Self {
        let stop_words = vec![
            // English stop words
            "a".to_string(),
            "an".to_string(),
            "the".to_string(),
            "is".to_string(),
            "are".to_string(),
            "was".to_string(),
            "were".to_string(),
            "be".to_string(),
            "been".to_string(),
            "being".to_string(),
            "have".to_string(),
            "has".to_string(),
            "had".to_string(),
            "do".to_string(),
            "does".to_string(),
            "did".to_string(),
            "will".to_string(),
            "would".to_string(),
            "could".to_string(),
            "should".to_string(),
            "may".to_string(),
            "might".to_string(),
            "must".to_string(),
            "can".to_string(),
            "to".to_string(),
            "of".to_string(),
            "in".to_string(),
            "for".to_string(),
            "on".to_string(),
            "with".to_string(),
            "at".to_string(),
            "by".to_string(),
            "from".to_string(),
            "as".to_string(),
            "into".to_string(),
            "through".to_string(),
            "during".to_string(),
            "before".to_string(),
            "after".to_string(),
            "above".to_string(),
            "below".to_string(),
            "between".to_string(),
            "under".to_string(),
            "again".to_string(),
            "further".to_string(),
            "then".to_string(),
            "once".to_string(),
            "here".to_string(),
            "there".to_string(),
            "when".to_string(),
            "where".to_string(),
            "why".to_string(),
            "how".to_string(),
            "all".to_string(),
            "each".to_string(),
            "few".to_string(),
            "more".to_string(),
            "most".to_string(),
            "other".to_string(),
            "some".to_string(),
            "such".to_string(),
            "no".to_string(),
            "nor".to_string(),
            "not".to_string(),
            "only".to_string(),
            "own".to_string(),
            "same".to_string(),
            "so".to_string(),
            "than".to_string(),
            "too".to_string(),
            "very".to_string(),
            "just".to_string(),
            "but".to_string(),
            "and".to_string(),
            "or".to_string(),
            "if".to_string(),
            "because".to_string(),
            "until".to_string(),
            "while".to_string(),
            // Chinese stop words
            "的".to_string(),
            "了".to_string(),
            "是".to_string(),
            "在".to_string(),
            "我".to_string(),
            "有".to_string(),
            "和".to_string(),
            "就".to_string(),
            "不".to_string(),
            "人".to_string(),
            "都".to_string(),
            "一".to_string(),
            "个".to_string(),
            "上".to_string(),
            "也".to_string(),
            "很".to_string(),
            "到".to_string(),
            "说".to_string(),
            "要".to_string(),
            "去".to_string(),
            "你".to_string(),
            "会".to_string(),
            "着".to_string(),
            "没有".to_string(),
            "看".to_string(),
            "好".to_string(),
            "自己".to_string(),
            "这".to_string(),
            "他".to_string(),
            "她".to_string(),
            "它".to_string(),
            "们".to_string(),
            "那".to_string(),
            "些".to_string(),
            "什么".to_string(),
            "怎么".to_string(),
            "哪".to_string(),
            "为什么".to_string(),
            "可以".to_string(),
            "因为".to_string(),
            "所以".to_string(),
            "但是".to_string(),
            "如果".to_string(),
            "虽然".to_string(),
            "而且".to_string(),
            "或者".to_string(),
            "还是".to_string(),
            "已经".to_string(),
            "把".to_string(),
            "被".to_string(),
            "让".to_string(),
            "给".to_string(),
            "从".to_string(),
            "对".to_string(),
            "比".to_string(),
            "向".to_string(),
            "过".to_string(),
            "得".to_string(),
            "地".to_string(),
        ];

        Self {
            storage,
            stop_words,
            config: BM25Config::default(),
        }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !self.stop_words.iter().any(|sw| sw == w))
            .map(|w| self.stem(w))
            .collect()
    }

    fn stem(&self, word: &str) -> String {
        let suffixes = ["ing", "ed", "es", "s", "er", "est", "ly", "tion", "ness"];
        for suffix in &suffixes {
            if word.ends_with(suffix) && word.len() > suffix.len() + 2 {
                return word[..word.len() - suffix.len()].to_string();
            }
        }
        word.to_string()
    }

    fn calculate_idf(
        &self,
        term: &str,
        doc_count: usize,
        term_doc_counts: &HashMap<String, usize>,
    ) -> f64 {
        let doc_freq = *term_doc_counts.get(term).unwrap_or(&0) as f64;
        if doc_freq == 0.0 {
            return 0.0;
        }
        ((doc_count as f64 - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln()
    }

    pub fn retrieve(
        &self,
        query: &CrossSessionQuery,
    ) -> anyhow::Result<Vec<MemoryRetrievalResult>> {
        let query_terms = self.tokenize(&query.query);
        let limit = query.limit.unwrap_or(10);

        let trajectories = self.storage.get_trajectories(None)?;
        let doc_count = trajectories.len();

        let mut term_doc_counts: HashMap<String, usize> = HashMap::new();
        let mut doc_lengths: Vec<usize> = Vec::new();
        let mut doc_terms: Vec<HashMap<String, usize>> = Vec::new();

        for traj in &trajectories {
            let content = format!("{} {} {}", traj.topic, traj.summary, traj.outcome.as_str());
            let terms = self.tokenize(&content);
            let doc_length = terms.len();
            doc_lengths.push(doc_length);

            let mut term_freq: HashMap<String, usize> = HashMap::new();
            for term in &terms {
                *term_freq.entry(term.clone()).or_insert(0) += 1;
            }
            doc_terms.push(term_freq.clone());

            for term in term_freq.keys() {
                *term_doc_counts.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let avg_doc_length = if doc_lengths.is_empty() {
            0.0
        } else {
            doc_lengths.iter().sum::<usize>() as f64 / doc_lengths.len() as f64
        };

        let mut scores: Vec<(String, f64)> = Vec::new();

        for (doc_idx, traj) in trajectories.iter().enumerate() {
            let content = format!("{} {} {}", traj.topic, traj.summary, traj.outcome.as_str());
            self.tokenize(&content);
            let doc_length = doc_lengths[doc_idx];

            let mut score = 0.0;
            for query_term in &query_terms {
                if let Some(&tf) = doc_terms[doc_idx].get(query_term) {
                    let idf = self.calculate_idf(query_term, doc_count, &term_doc_counts);
                    // Standard BM25 TF normalization:
                    // tf_norm = tf * (k1 + 1) / (k1 * (1 - b + b * dl/avgdl) + tf)
                    let tf_norm = (tf as f64 * (self.config.k1 + 1.0))
                        / (self.config.k1
                            * (1.0 - self.config.b
                                + self.config.b * (doc_length as f64 / avg_doc_length))
                            + tf as f64);
                    score += idf * tf_norm;
                }
            }

            if score > 0.0 {
                scores.push((traj.id.clone(), score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut results = Vec::new();
        for (traj_id, score) in scores.into_iter().take(limit) {
            if let Some(traj) = trajectories.iter().find(|t| t.id == traj_id) {
                results.push(MemoryRetrievalResult {
                    id: traj.id.clone(),
                    result_type: RetrievalType::Trajectory,
                    content: format!("[{}] {}", traj.topic, traj.summary),
                    relevance: score,
                    session_id: Some(traj.session_id.clone()),
                    timestamp: traj.created_at.timestamp_millis(),
                    metadata: Some(serde_json::json!({
                        "outcome": traj.outcome.as_str(),
                        "quality": traj.quality.overall,
                    })),
                });
            }
        }

        Ok(results)
    }

    pub fn retrieve_by_entities(
        &self,
        entities: &[String],
    ) -> anyhow::Result<Vec<MemoryRetrievalResult>> {
        let query = CrossSessionQuery {
            query: entities.join(" "),
            intent: None,
            entities: entities.to_vec(),
            time_range: None,
            limit: Some(20),
        };
        self.retrieve(&query)
    }
}

impl Default for CrossSessionRetriever {
    fn default() -> Self {
        // 使用内存模式（无持久化存储）
        Self::new(Arc::new(TrajectoryStorage::new(std::sync::Arc::new(
            sea_orm::DatabaseConnection::default(),
        ))))
    }
}
