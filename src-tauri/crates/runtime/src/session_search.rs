use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub session_id: String,
    pub message_index: usize,
    pub content: String,
    pub highlight_ranges: Vec<(usize, usize)>,
    pub timestamp: String,
    pub agent_name: Option<String>,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub session_filter: Option<Vec<String>>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

pub struct SessionSearchEngine {
    cache: Arc<RwLock<HashMap<String, Vec<SearchIndexEntry>>>>,
}

#[derive(Debug, Clone)]
struct SearchIndexEntry {
    message_index: usize,
    content: String,
    timestamp: String,
    agent_name: Option<String>,
}

impl Default for SessionSearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionSearchEngine {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn index_session(&self, session_id: &str, messages: Vec<IndexedMessage>) {
        let entries: Vec<SearchIndexEntry> = messages
            .into_iter()
            .enumerate()
            .map(|(idx, msg)| SearchIndexEntry {
                message_index: idx,
                content: msg.content,
                timestamp: msg.timestamp,
                agent_name: msg.agent_name,
            })
            .collect();

        self.cache.write().await.insert(session_id.to_string(), entries);
    }

    pub async fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let cache = self.cache.read().await;
        let mut results = Vec::new();

        let pattern = if query.regex {
            if query.case_sensitive {
                query.query.clone()
            } else {
                format!("(?i){}", query.query)
            }
        } else {
            let escaped = regex::escape(&query.query);
            if query.case_sensitive {
                escaped
            } else {
                format!("(?i){}", escaped)
            }
        };

        let Ok(re) = regex::Regex::new(&pattern) else {
            return results;
        };

        for (session_id, entries) in cache.iter() {
            if let Some(ref filters) = query.session_filter {
                if !filters.contains(session_id) {
                    continue;
                }
            }

            for entry in entries {
                for mat in re.find_iter(&entry.content) {
                    let highlight_ranges = vec![(mat.start(), mat.end())];

                    let snippet = Self::create_snippet(&entry.content, mat.start(), 80);

                    results.push(SearchResult {
                        session_id: session_id.clone(),
                        message_index: entry.message_index,
                        content: snippet,
                        highlight_ranges,
                        timestamp: entry.timestamp.clone(),
                        agent_name: entry.agent_name.clone(),
                        score: Self::calculate_score(&entry.content, mat.start(), &query.query),
                    });
                }
            }
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(query.limit);
        if query.offset > 0 {
            results = results.into_iter().skip(query.offset).collect();
        }

        results
    }

    fn create_snippet(content: &str, match_pos: usize, context: usize) -> String {
        let start = match_pos.saturating_sub(context);
        let end = (match_pos + context).min(content.len());

        let mut snippet = String::new();
        if start > 0 {
            snippet.push_str("...");
        }
        snippet.push_str(&content[start..end]);
        if end < content.len() {
            snippet.push_str("...");
        }

        snippet
    }

    fn calculate_score(content: &str, match_pos: usize, query: &str) -> f32 {
        let mut score = 1.0;

        if content.starts_with(query) {
            score += 2.0;
        }

        let relative_pos = match_pos as f32 / content.len() as f32;
        if relative_pos < 0.2 {
            score += 1.0;
        }

        let query_words: Vec<&str> = query.split_whitespace().collect();
        for word in query_words {
            if content.to_lowercase().contains(&word.to_lowercase()) {
                score += 0.5;
            }
        }

        score
    }

    pub async fn remove_session(&self, session_id: &str) {
        self.cache.write().await.remove(session_id);
    }

    pub async fn clear_index(&self) {
        self.cache.write().await.clear();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedMessage {
    pub content: String,
    pub timestamp: String,
    pub agent_name: Option<String>,
}
