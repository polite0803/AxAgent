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

    #[test]
    fn test_insight_new() {
        let insight = Insight::new(
            InsightCategory::Knowledge,
            "Test Title".to_string(),
            "Test Content".to_string(),
            "task-123".to_string(),
        );
        assert!(!insight.id.is_empty());
        assert_eq!(insight.category, InsightCategory::Knowledge);
        assert_eq!(insight.title, "Test Title");
        assert_eq!(insight.content, "Test Content");
        assert_eq!(insight.source_task_id, "task-123");
        assert!((insight.confidence - 0.5).abs() < f32::EPSILON);
        assert!(insight.tags.is_empty());
        assert!(insight.usage_count == 0);
        assert!(insight.last_used.is_none());
    }

    #[test]
    fn test_insight_with_confidence() {
        let insight = Insight::new(
            InsightCategory::Optimization,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )
        .with_confidence(0.85);
        assert!((insight.confidence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_with_confidence_clamp_high() {
        let insight = Insight::new(
            InsightCategory::Optimization,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )
        .with_confidence(1.5);
        assert!((insight.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_with_confidence_clamp_low() {
        let insight = Insight::new(
            InsightCategory::Optimization,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )
        .with_confidence(-0.5);
        assert!((insight.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_with_tags() {
        let insight = Insight::new(
            InsightCategory::ErrorPattern,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )
        .with_tags(vec!["error".to_string(), "network".to_string()]);
        assert_eq!(insight.tags.len(), 2);
        assert!(insight.tags.contains(&"error".to_string()));
        assert!(insight.tags.contains(&"network".to_string()));
    }

    #[test]
    fn test_insight_record_usage() {
        let mut insight = Insight::new(
            InsightCategory::Knowledge,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        );
        assert_eq!(insight.usage_count, 0);
        assert!(insight.last_used.is_none());

        insight.record_usage();
        assert_eq!(insight.usage_count, 1);
        assert!(insight.last_used.is_some());

        insight.record_usage();
        assert_eq!(insight.usage_count, 2);
    }

    #[test]
    fn test_insight_category_as_str() {
        assert_eq!(InsightCategory::ErrorPattern.as_str(), "error_pattern");
        assert_eq!(InsightCategory::SuccessPattern.as_str(), "success_pattern");
        assert_eq!(InsightCategory::Optimization.as_str(), "optimization");
        assert_eq!(InsightCategory::Knowledge.as_str(), "knowledge");
        assert_eq!(InsightCategory::Workflow.as_str(), "workflow");
        assert_eq!(InsightCategory::ToolUsage.as_str(), "tool_usage");
    }

    #[test]
    fn test_insight_category_from_str() {
        assert_eq!(InsightCategory::from_str("error_pattern"), Some(InsightCategory::ErrorPattern));
        assert_eq!(InsightCategory::from_str("success_pattern"), Some(InsightCategory::SuccessPattern));
        assert_eq!(InsightCategory::from_str("optimization"), Some(InsightCategory::Optimization));
        assert_eq!(InsightCategory::from_str("knowledge"), Some(InsightCategory::Knowledge));
        assert_eq!(InsightCategory::from_str("workflow"), Some(InsightCategory::Workflow));
        assert_eq!(InsightCategory::from_str("tool_usage"), Some(InsightCategory::ToolUsage));
        assert_eq!(InsightCategory::from_str("invalid"), None);
    }

    #[test]
    fn test_generate_from_reflection_empty() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-1".to_string());
        let result = generator.generate_from_reflection(&reflection);
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_from_reflection_error_pattern() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-1".to_string())
            .with_patterns(
                vec!["Timeout error".to_string()],
                vec![],
            );
        let result = generator.generate_from_reflection(&reflection);
        assert!(result.is_some());
        let insight = result.unwrap();
        assert_eq!(insight.category, InsightCategory::ErrorPattern);
        assert!(insight.title.contains("Error Pattern"));
        assert!(insight.content.contains("Timeout error"));
        assert!(insight.tags.contains(&"error_handling".to_string()));
    }

    #[test]
    fn test_generate_from_reflection_success_pattern() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-2".to_string())
            .with_patterns(
                vec![],
                vec!["Efficient caching".to_string()],
            );
        let result = generator.generate_from_reflection(&reflection);
        assert!(result.is_some());
        let insight = result.unwrap();
        assert_eq!(insight.category, InsightCategory::SuccessPattern);
        assert!(insight.title.contains("Success Pattern"));
        assert!(insight.content.contains("Efficient caching"));
        assert!(insight.tags.contains(&"reusable".to_string()));
    }

    #[test]
    fn test_generate_from_reflection_high_quality() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-3".to_string())
            .with_quality(9, "Excellent".to_string())
            .with_patterns(vec![], vec!["Pattern".to_string()]);
        let result = generator.generate_from_reflection(&reflection);
        let insight = result.unwrap();
        assert!((insight.confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn test_generate_from_reflection_medium_quality() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-4".to_string())
            .with_quality(6, "Good".to_string())
            .with_patterns(vec![], vec!["Pattern".to_string()]);
        let result = generator.generate_from_reflection(&reflection);
        let insight = result.unwrap();
        assert!((insight.confidence - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_generate_from_reflection_low_quality() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-5".to_string())
            .with_quality(3, "Poor".to_string())
            .with_patterns(vec!["Error".to_string()], vec![]);
        let result = generator.generate_from_reflection(&reflection);
        let insight = result.unwrap();
        assert!((insight.confidence - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn test_generate_from_reflection_both_patterns() {
        let generator = InsightGenerator::new();
        let reflection = Reflection::new("task-6".to_string())
            .with_patterns(
                vec!["Error pattern".to_string()],
                vec!["Reusable pattern".to_string()],
            );
        let result = generator.generate_from_reflection(&reflection);
        let insight = result.unwrap();
        assert_eq!(insight.category, InsightCategory::ErrorPattern);
        assert!(insight.tags.contains(&"error_handling".to_string()));
        assert!(insight.tags.contains(&"reusable".to_string()));
    }

    #[tokio::test]
    async fn test_get_insights_by_category() {
        let generator = InsightGenerator::new();

        generator.store_insight(Insight::new(
            InsightCategory::ErrorPattern,
            "Error".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )).await;

        generator.store_insight(Insight::new(
            InsightCategory::Knowledge,
            "Knowledge".to_string(),
            "Content".to_string(),
            "task-2".to_string(),
        )).await;

        let errors = generator.get_insights(Some(InsightCategory::ErrorPattern)).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].category, InsightCategory::ErrorPattern);

        let knowledge = generator.get_insights(Some(InsightCategory::Knowledge)).await;
        assert_eq!(knowledge.len(), 1);
        assert_eq!(knowledge[0].category, InsightCategory::Knowledge);

        let all = generator.get_insights(None).await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_insight_by_id() {
        let generator = InsightGenerator::new();

        let insight = Insight::new(
            InsightCategory::Knowledge,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        );
        let id = insight.id.clone();
        generator.store_insight(insight).await;

        let found = generator.get_insight_by_id(&id).await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Title");

        let not_found = generator.get_insight_by_id("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_record_insight_usage_not_found() {
        let generator = InsightGenerator::new();
        let result = generator.record_insight_usage("nonexistent").await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_record_insight_usage_success() {
        let generator = InsightGenerator::new();

        let insight = Insight::new(
            InsightCategory::Knowledge,
            "Title".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        );
        let id = insight.id.clone();
        generator.store_insight(insight).await;

        let result = generator.record_insight_usage(&id).await;
        assert!(result);

        let updated = generator.get_insight_by_id(&id).await.unwrap();
        assert_eq!(updated.usage_count, 1);
        assert!(updated.last_used.is_some());
    }

    #[tokio::test]
    async fn test_get_stats_empty() {
        let generator = InsightGenerator::new();
        let stats = generator.get_stats().await;
        assert_eq!(stats.total_insights, 0);
        assert!(stats.by_category.is_empty());
        assert!((stats.avg_confidence - 0.0).abs() < f32::EPSILON);
        assert!(stats.most_used.is_none());
    }

    #[tokio::test]
    async fn test_get_stats_with_data() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::ErrorPattern, "E1".to_string(), "C1".to_string(), "t1".to_string())
                .with_confidence(0.8)
        ).await;

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "K1".to_string(), "C2".to_string(), "t2".to_string())
                .with_confidence(0.6)
        ).await;

        let stats = generator.get_stats().await;
        assert_eq!(stats.total_insights, 2);
        assert!((stats.avg_confidence - 0.7).abs() < 0.01);
        assert_eq!(*stats.by_category.get("error_pattern").unwrap_or(&0), 1);
        assert_eq!(*stats.by_category.get("knowledge").unwrap_or(&0), 1);
    }

    #[tokio::test]
    async fn test_get_stats_most_used() {
        let generator = InsightGenerator::new();

        let insight1 = Insight::new(InsightCategory::Knowledge, "K1".to_string(), "C1".to_string(), "t1".to_string());
        let id1 = insight1.id.clone();
        generator.store_insight(insight1).await;

        let insight2 = Insight::new(InsightCategory::Knowledge, "K2".to_string(), "C2".to_string(), "t2".to_string());
        generator.store_insight(insight2).await;

        generator.record_insight_usage(&id1).await;
        generator.record_insight_usage(&id1).await;

        let stats = generator.get_stats().await;
        assert!(stats.most_used.is_some());
        assert_eq!(stats.most_used.unwrap().id, id1);
    }

    #[tokio::test]
    async fn test_delete_insight() {
        let generator = InsightGenerator::new();

        let insight = Insight::new(InsightCategory::Knowledge, "Title".to_string(), "Content".to_string(), "t1".to_string());
        let id = insight.id.clone();
        generator.store_insight(insight).await;

        let deleted = generator.delete_insight(&id).await;
        assert!(deleted);

        let found = generator.get_insight_by_id(&id).await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_delete_insight_not_found() {
        let generator = InsightGenerator::new();
        let deleted = generator.delete_insight("nonexistent").await;
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_clear_all() {
        let generator = InsightGenerator::new();

        generator.store_insight(Insight::new(InsightCategory::Knowledge, "K1".to_string(), "C1".to_string(), "t1".to_string())).await;
        generator.store_insight(Insight::new(InsightCategory::ErrorPattern, "E1".to_string(), "C2".to_string(), "t2".to_string())).await;

        assert_eq!(generator.get_insights(None).await.len(), 2);

        generator.clear_all().await;

        assert_eq!(generator.get_insights(None).await.len(), 0);
        let stats = generator.get_stats().await;
        assert_eq!(stats.total_insights, 0);
        assert!(stats.by_category.is_empty());
    }

    #[tokio::test]
    async fn test_get_recent_insights() {
        let generator = InsightGenerator::new();

        generator.store_insight(Insight::new(InsightCategory::Knowledge, "K1".to_string(), "C1".to_string(), "t1".to_string())).await;
        generator.store_insight(Insight::new(InsightCategory::Knowledge, "K2".to_string(), "C2".to_string(), "t2".to_string())).await;
        generator.store_insight(Insight::new(InsightCategory::Knowledge, "K3".to_string(), "C3".to_string(), "t3".to_string())).await;

        let recent = generator.get_recent_insights(2).await;
        assert_eq!(recent.len(), 2);
    }

    #[tokio::test]
    async fn test_get_recent_insights_empty() {
        let generator = InsightGenerator::new();
        let recent = generator.get_recent_insights(5).await;
        assert!(recent.is_empty());
    }

    #[tokio::test]
    async fn test_get_top_insights() {
        let generator = InsightGenerator::new();

        let insight1 = Insight::new(InsightCategory::Knowledge, "K1".to_string(), "C1".to_string(), "t1".to_string());
        let id1 = insight1.id.clone();
        generator.store_insight(insight1).await;

        let insight2 = Insight::new(InsightCategory::Knowledge, "K2".to_string(), "C2".to_string(), "t2".to_string());
        generator.store_insight(insight2).await;

        generator.record_insight_usage(&id1).await;
        generator.record_insight_usage(&id1).await;

        let top = generator.get_top_insights(1).await;
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].id, id1);
    }

    #[tokio::test]
    async fn test_get_high_confidence_insights() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "High".to_string(), "C1".to_string(), "t1".to_string())
                .with_confidence(0.9)
        ).await;

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "Low".to_string(), "C2".to_string(), "t2".to_string())
                .with_confidence(0.3)
        ).await;

        let high = generator.get_high_confidence_insights(0.8).await;
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].title, "High");
    }

    #[test]
    fn test_generate_optimization_insight() {
        let generator = InsightGenerator::new();
        let insight = generator.generate_optimization_insight("Build Project", 30000);

        assert_eq!(insight.category, InsightCategory::Optimization);
        assert!(insight.title.contains("Performance Optimization"));
        assert!(insight.content.contains("Build Project"));
        assert!(insight.content.contains("30000ms"));
        assert!((insight.confidence - 0.7).abs() < f32::EPSILON);
        assert!(insight.tags.contains(&"performance".to_string()));
        assert!(insight.tags.contains(&"optimization".to_string()));
    }

    #[test]
    fn test_generate_knowledge_insight() {
        let generator = InsightGenerator::new();
        let insight = generator.generate_knowledge_insight("Rust", "Rust is a systems programming language");

        assert_eq!(insight.category, InsightCategory::Knowledge);
        assert!(insight.title.contains("Knowledge"));
        assert!(insight.title.contains("Rust"));
        assert!(insight.content.contains("systems programming"));
        assert!((insight.confidence - 0.8).abs() < f32::EPSILON);
        assert!(insight.tags.contains(&"knowledge".to_string()));
        assert!(insight.tags.contains(&"rust".to_string()));
    }

    #[test]
    fn test_generate_workflow_insight() {
        let generator = InsightGenerator::new();
        let tools = vec!["search".to_string(), "read".to_string(), "edit".to_string()];
        let insight = generator.generate_workflow_insight(&tools, "Code modification workflow");

        assert_eq!(insight.category, InsightCategory::Workflow);
        assert!(insight.title.contains("Workflow"));
        assert!(insight.content.contains("search -> read -> edit"));
        assert!((insight.confidence - 0.6).abs() < f32::EPSILON);
        assert!(insight.tags.contains(&"workflow".to_string()));
        assert!(insight.tags.contains(&"tools".to_string()));
    }

    #[tokio::test]
    async fn test_insight_generator_default() {
        let generator = InsightGenerator::default();
        let insights = generator.get_insights(None).await;
        assert!(insights.is_empty());
    }

    #[tokio::test]
    async fn test_search_insights_by_tag() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::ErrorPattern, "Error".to_string(), "Content".to_string(), "t1".to_string())
                .with_tags(vec!["network".to_string()])
        ).await;

        let results = generator.search_insights("network").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_insights_by_content() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "Title".to_string(), "Database optimization techniques".to_string(), "t1".to_string())
        ).await;

        let results = generator.search_insights("database").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_insights_case_insensitive() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "UPPERCASE Title".to_string(), "Content".to_string(), "t1".to_string())
        ).await;

        let results = generator.search_insights("uppercase").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_search_insights_no_match() {
        let generator = InsightGenerator::new();

        generator.store_insight(
            Insight::new(InsightCategory::Knowledge, "Title".to_string(), "Content".to_string(), "t1".to_string())
        ).await;

        let results = generator.search_insights("nonexistent").await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_insight_serialization() {
        let insight = Insight::new(
            InsightCategory::Knowledge,
            "Test".to_string(),
            "Content".to_string(),
            "task-1".to_string(),
        )
        .with_confidence(0.8)
        .with_tags(vec!["tag1".to_string()]);

        let json = serde_json::to_string(&insight).unwrap();
        let deserialized: Insight = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, insight.id);
        assert_eq!(deserialized.title, "Test");
        assert!((deserialized.confidence - 0.8).abs() < f32::EPSILON);
        assert_eq!(deserialized.tags.len(), 1);
    }

    #[test]
    fn test_insight_stats_serialization() {
        let stats = InsightStats {
            total_insights: 5,
            by_category: vec![("knowledge".to_string(), 3)].into_iter().collect(),
            avg_confidence: 0.75,
            most_used: None,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: InsightStats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_insights, 5);
        assert!((deserialized.avg_confidence - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_insight_category_serialization() {
        let cat = InsightCategory::Optimization;
        let json = serde_json::to_string(&cat).unwrap();
        let deserialized: InsightCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, InsightCategory::Optimization);
    }
}
