// SPDX-License-Identifier: AGPL-3.0-only

//! 统一搜索相关的共享类型定义。
//! 消除 agent::{research_state, deep_research} 中 SearchResult 的重复定义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SearchSourceType {
    Web,
    Academic,
    Wikipedia,
    GitHub,
    Documentation,
    News,
    Blog,
    Forum,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub source_type: SearchSourceType,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub published_date: Option<String>,
    pub credibility_score: Option<f32>,
    pub relevance_score: f32,
    pub extracted_at: DateTime<Utc>,
    pub query: Option<String>,
}

impl SearchResult {
    pub fn new(source_type: SearchSourceType, url: String, title: String, snippet: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_type,
            url,
            title,
            snippet,
            published_date: None,
            credibility_score: None,
            relevance_score: 0.0,
            extracted_at: Utc::now(),
            query: None,
        }
    }

    pub fn with_published_date(mut self, date: String) -> Self {
        self.published_date = Some(date);
        self
    }

    pub fn with_credibility(mut self, score: f32) -> Self {
        self.credibility_score = Some(score);
        self
    }

    pub fn with_relevance(mut self, score: f32) -> Self {
        self.relevance_score = score;
        self
    }
}
