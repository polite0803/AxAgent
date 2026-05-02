use crate::reflector::Reflection;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: String,
    pub category: InsightCategory,
    pub title: String,
    pub content: String,
    pub source_task_id: String,
    pub confidence: f32,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub usage_count: u32,
    pub last_used: Option<DateTime<Utc>>,
}

impl Insight {
    pub fn new(
        category: InsightCategory,
        title: String,
        content: String,
        source_task_id: String,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            category,
            title,
            content,
            source_task_id,
            confidence: 0.5,
            tags: Vec::new(),
            created_at: Utc::now(),
            usage_count: 0,
            last_used: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn record_usage(&mut self) {
        self.usage_count += 1;
        self.last_used = Some(Utc::now());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InsightCategory {
    ErrorPattern,
    SuccessPattern,
    Optimization,
    Knowledge,
    Workflow,
    ToolUsage,
}

impl InsightCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightCategory::ErrorPattern => "error_pattern",
            InsightCategory::SuccessPattern => "success_pattern",
            InsightCategory::Optimization => "optimization",
            InsightCategory::Knowledge => "knowledge",
            InsightCategory::Workflow => "workflow",
            InsightCategory::ToolUsage => "tool_usage",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "error_pattern" => Some(InsightCategory::ErrorPattern),
            "success_pattern" => Some(InsightCategory::SuccessPattern),
            "optimization" => Some(InsightCategory::Optimization),
            "knowledge" => Some(InsightCategory::Knowledge),
            "workflow" => Some(InsightCategory::Workflow),
            "tool_usage" => Some(InsightCategory::ToolUsage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsightStats {
    pub total_insights: usize,
    pub by_category: HashMap<String, usize>,
    pub avg_confidence: f32,
    pub most_used: Option<Insight>,
}

pub struct InsightGenerator {
    insights: Arc<RwLock<Vec<Insight>>>,
    category_stats: Arc<RwLock<HashMap<InsightCategory, usize>>>,
}

impl InsightGenerator {
    pub fn new() -> Self {
        Self {
            insights: Arc::new(RwLock::new(Vec::new())),
            category_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn generate_from_reflection(&self, reflection: &Reflection) -> Option<Insight> {
        if reflection.reusable_patterns.is_empty() && reflection.error_patterns.is_empty() {
            return None;
        }

        let category = if !reflection.error_patterns.is_empty() {
            InsightCategory::ErrorPattern
        } else {
            InsightCategory::SuccessPattern
        };

        let title = if !reflection.error_patterns.is_empty() {
            format!("Error Pattern from Task {}", reflection.task_id)
        } else {
            format!("Success Pattern from Task {}", reflection.task_id)
        };

        let content = if !reflection.error_patterns.is_empty() {
            reflection.error_patterns.join("; ")
        } else {
            reflection.reusable_patterns.join("; ")
        };

        let confidence = if reflection.quality_score >= 8 {
            0.9
        } else if reflection.quality_score >= 5 {
            0.7
        } else {
            0.4
        };

        let mut tags = Vec::new();
        if !reflection.error_patterns.is_empty() {
            tags.push("error_handling".to_string());
        }
        if !reflection.reusable_patterns.is_empty() {
            tags.push("reusable".to_string());
        }
        tags.push(format!("quality_{}", reflection.quality_score));

        Some(
            Insight::new(category, title, content, reflection.task_id.clone())
                .with_confidence(confidence)
                .with_tags(tags),
        )
    }

    pub async fn store_insight(&self, insight: Insight) {
        let mut insights = self.insights.write().await;
        insights.push(insight.clone());

        let mut stats = self.category_stats.write().await;
        *stats.entry(insight.category).or_insert(0) += 1;
    }

    pub async fn get_insights(&self, category: Option<InsightCategory>) -> Vec<Insight> {
        let insights = self.insights.read().await;
        match category {
            Some(cat) => insights
                .iter()
                .filter(|i| i.category == cat)
                .cloned()
                .collect(),
            None => insights.clone(),
        }
    }

    pub async fn get_insight_by_id(&self, id: &str) -> Option<Insight> {
        let insights = self.insights.read().await;
        insights.iter().find(|i| i.id == id).cloned()
    }

    pub async fn search_insights(&self, query: &str) -> Vec<Insight> {
        let query_lower = query.to_lowercase();
        let insights = self.insights.read().await;

        insights
            .iter()
            .filter(|i| {
                i.title.to_lowercase().contains(&query_lower)
                    || i.content.to_lowercase().contains(&query_lower)
                    || i.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    pub async fn record_insight_usage(&self, id: &str) -> bool {
        let mut insights = self.insights.write().await;
        if let Some(insight) = insights.iter_mut().find(|i| i.id == id) {
            insight.record_usage();
            return true;
        }
        false
    }

    pub async fn get_stats(&self) -> InsightStats {
        let insights = self.insights.read().await;
        let stats = self.category_stats.read().await;

        let total = insights.len();
        let by_category: HashMap<String, usize> = stats
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), *v))
            .collect();

        let avg_confidence = if total > 0 {
            insights.iter().map(|i| i.confidence).sum::<f32>() / total as f32
        } else {
            0.0
        };

        let most_used = insights.iter().max_by_key(|i| i.usage_count).cloned();

        InsightStats {
            total_insights: total,
            by_category,
            avg_confidence,
            most_used,
        }
    }

    pub async fn delete_insight(&self, id: &str) -> bool {
        let mut insights = self.insights.write().await;
        let initial_len = insights.len();
        insights.retain(|i| i.id != id);
        insights.len() < initial_len
    }

    pub async fn clear_all(&self) {
        let mut insights = self.insights.write().await;
        insights.clear();

        let mut stats = self.category_stats.write().await;
        stats.clear();
    }

    pub async fn get_recent_insights(&self, limit: usize) -> Vec<Insight> {
        let insights = self.insights.read().await;
        let mut sorted = insights.clone();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        sorted.into_iter().take(limit).collect()
    }

    pub async fn get_top_insights(&self, limit: usize) -> Vec<Insight> {
        let insights = self.insights.read().await;
        let mut sorted = insights.clone();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.usage_count));
        sorted.into_iter().take(limit).collect()
    }

    pub async fn get_high_confidence_insights(&self, threshold: f32) -> Vec<Insight> {
        let insights = self.insights.read().await;
        insights
            .iter()
            .filter(|i| i.confidence >= threshold)
            .cloned()
            .collect()
    }

    pub fn generate_optimization_insight(
        &self,
        task_description: &str,
        duration_ms: u64,
    ) -> Insight {
        Insight::new(
            InsightCategory::Optimization,
            format!("Performance Optimization: {}", task_description),
            format!(
                "Task '{}' took {}ms. Consider caching, parallel execution, or algorithm optimization.",
                task_description, duration_ms
            ),
            String::new(),
        )
        .with_confidence(0.7)
        .with_tags(vec!["performance".to_string(), "optimization".to_string()])
    }

    pub fn generate_knowledge_insight(&self, topic: &str, content: &str) -> Insight {
        Insight::new(
            InsightCategory::Knowledge,
            format!("Knowledge: {}", topic),
            content.to_string(),
            String::new(),
        )
        .with_confidence(0.8)
        .with_tags(vec!["knowledge".to_string(), topic.to_lowercase()])
    }

    pub fn generate_workflow_insight(&self, tools: &[String], description: &str) -> Insight {
        Insight::new(
            InsightCategory::Workflow,
            format!("Workflow: {}", description),
            format!("Tool sequence: {}", tools.join(" -> ")),
            String::new(),
        )
        .with_confidence(0.6)
        .with_tags(vec!["workflow".to_string(), "tools".to_string()])
    }
}

impl Default for InsightGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_insight_storage() {
        let generator = InsightGenerator::new();

        let insight = Insight::new(
            InsightCategory::ErrorPattern,
            "Test Insight".to_string(),
            "Test content".to_string(),
            "task-1".to_string(),
        );

        generator.store_insight(insight.clone()).await;

        let insights = generator.get_insights(None).await;
        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].title, "Test Insight");
    }

    #[tokio::test]
    async fn test_insight_usage_tracking() {
        let generator = InsightGenerator::new();

        let insight = generator.generate_optimization_insight("Test Task", 5000);
        generator.store_insight(insight.clone()).await;

        let id = insight.id.clone();
        generator.record_insight_usage(&id).await;

        let updated = generator.get_insight_by_id(&id).await.unwrap();
        assert_eq!(updated.usage_count, 1);
    }

    #[tokio::test]
    async fn test_search_insights() {
        let generator = InsightGenerator::new();

        generator
            .store_insight(
                Insight::new(
                    InsightCategory::ErrorPattern,
                    "Timeout Error".to_string(),
                    "Network timeout occurred".to_string(),
                    "task-1".to_string(),
                )
                .with_tags(vec!["network".to_string()]),
            )
            .await;

        let results = generator.search_insights("timeout").await;
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Timeout"));
    }
}
