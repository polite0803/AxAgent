use crate::insight_generator::InsightGenerator;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    pub task_id: String,
    pub task_description: String,
    pub result: Option<serde_json::Value>,
    pub success: bool,
    pub error: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub tools_used: Vec<String>,
    pub iterations: usize,
}

impl TaskExecutionRecord {
    pub fn new(
        task_id: String,
        task_description: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            task_id,
            task_description,
            result: None,
            success: false,
            error: None,
            start_time,
            end_time,
            duration_ms: 0,
            tools_used: Vec::new(),
            iterations: 0,
        }
    }

    pub fn with_result(mut self, result: serde_json::Value) -> Self {
        self.result = Some(result);
        self
    }

    pub fn with_success(mut self, success: bool) -> Self {
        self.success = success;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self.success = false;
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools_used = tools;
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn compute_duration(&mut self) {
        self.duration_ms = self
            .end_time
            .signed_duration_since(self.start_time)
            .num_milliseconds() as u64;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub task_id: String,
    pub timestamp: DateTime<Utc>,
    pub quality_score: u8,
    pub quality_analysis: String,
    pub efficiency_analysis: String,
    pub error_patterns: Vec<String>,
    pub reusable_patterns: Vec<String>,
    pub knowledge_suggestions: Vec<String>,
    pub improvement_suggestions: Vec<String>,
    pub overall_summary: String,
}

impl Reflection {
    pub fn new(task_id: String) -> Self {
        Self {
            task_id,
            timestamp: Utc::now(),
            quality_score: 0,
            quality_analysis: String::new(),
            efficiency_analysis: String::new(),
            error_patterns: Vec::new(),
            reusable_patterns: Vec::new(),
            knowledge_suggestions: Vec::new(),
            improvement_suggestions: Vec::new(),
            overall_summary: String::new(),
        }
    }

    pub fn with_quality(mut self, score: u8, analysis: String) -> Self {
        self.quality_score = score.clamp(1, 10);
        self.quality_analysis = analysis;
        self
    }

    pub fn with_efficiency(mut self, analysis: String) -> Self {
        self.efficiency_analysis = analysis;
        self
    }

    pub fn with_patterns(mut self, errors: Vec<String>, reusable: Vec<String>) -> Self {
        self.error_patterns = errors;
        self.reusable_patterns = reusable;
        self
    }

    pub fn with_knowledge(mut self, suggestions: Vec<String>) -> Self {
        self.knowledge_suggestions = suggestions;
        self
    }

    pub fn with_improvements(mut self, suggestions: Vec<String>) -> Self {
        self.improvement_suggestions = suggestions;
        self
    }

    pub fn with_summary(mut self, summary: String) -> Self {
        self.overall_summary = summary;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionConfig {
    pub enabled: bool,
    pub min_quality_threshold: u8,
    pub store_insights: bool,
    pub max_history: usize,
}

impl Default for ReflectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_quality_threshold: 5,
            store_insights: true,
            max_history: 100,
        }
    }
}

pub struct Reflector {
    config: ReflectionConfig,
    insight_generator: Arc<InsightGenerator>,
    history: Arc<RwLock<Vec<Reflection>>>,
}

impl Reflector {
    pub fn new() -> Self {
        Self {
            config: ReflectionConfig::default(),
            insight_generator: Arc::new(InsightGenerator::new()),
            history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_config(mut self, config: ReflectionConfig) -> Self {
        self.config = config;
        self
    }

    pub async fn reflect(&self, record: &TaskExecutionRecord) -> Reflection {
        let mut reflection = Reflection::new(record.task_id.clone());

        let quality_analysis = self.analyze_quality(record);
        reflection.quality_score = self.calculate_quality_score(&quality_analysis);
        reflection.quality_analysis = quality_analysis;

        reflection.efficiency_analysis = self.analyze_efficiency(record);

        let (errors, reusable) = self.analyze_patterns(record);
        reflection.error_patterns = errors;
        reflection.reusable_patterns = reusable;

        reflection.knowledge_suggestions = self.generate_knowledge_suggestions(record);
        reflection.improvement_suggestions =
            self.generate_improvement_suggestions(record, &reflection);

        reflection.overall_summary = self.generate_summary(record, &reflection);

        if self.config.store_insights {
            let mut history = self.history.write().await;
            if history.len() >= self.config.max_history {
                history.remove(0);
            }
            history.push(reflection.clone());

            if let Some(insights) = self.insight_generator.generate_from_reflection(&reflection) {
                self.insight_generator.store_insight(insights).await;
            }
        }

        reflection
    }

    fn analyze_quality(&self, record: &TaskExecutionRecord) -> String {
        let mut analysis = String::new();

        if record.success {
            analysis.push_str("Task completed successfully. ");
        } else {
            analysis.push_str(&format!(
                "Task failed with error: {}. ",
                record.error.as_deref().unwrap_or("Unknown")
            ));
        }

        if !record.tools_used.is_empty() {
            analysis.push_str(&format!("Used {} tools. ", record.tools_used.len()));
        }

        if record.iterations > 10 {
            analysis.push_str("High iteration count suggests complex reasoning. ");
        } else if record.iterations <= 3 {
            analysis.push_str("Efficient resolution with minimal iterations. ");
        }

        analysis
    }

    fn calculate_quality_score(&self, analysis: &str) -> u8 {
        let mut score: i32 = 5;

        if analysis.contains("successfully") {
            score += 2;
        }
        if analysis.contains("failed") {
            score -= 3;
        }
        if analysis.contains("Efficient") {
            score += 1;
        }
        if analysis.contains("High iteration") {
            score -= 1;
        }

        score.clamp(1, 10) as u8
    }

    fn analyze_efficiency(&self, record: &TaskExecutionRecord) -> String {
        let mut analysis = String::new();

        let duration_per_iteration = if record.iterations > 0 {
            record.duration_ms / record.iterations as u64
        } else {
            record.duration_ms
        };

        analysis.push_str(&format!("Total duration: {}ms. ", record.duration_ms));
        analysis.push_str(&format!(
            "Duration per iteration: {}ms. ",
            duration_per_iteration
        ));

        if record.duration_ms > 60000 {
            analysis.push_str("Execution time exceeds 1 minute. Consider optimization. ");
        } else if record.duration_ms < 5000 {
            analysis.push_str("Quick execution. ");
        }

        if record.iterations > 20 {
            analysis.push_str("High iteration count may indicate inefficient reasoning. ");
        }

        analysis
    }

    fn analyze_patterns(&self, record: &TaskExecutionRecord) -> (Vec<String>, Vec<String>) {
        let mut error_patterns = Vec::new();
        let mut reusable_patterns = Vec::new();

        if let Some(ref error) = record.error {
            let error_lower = error.to_lowercase();

            if error_lower.contains("timeout") {
                error_patterns.push(
                    "Timeout issues - consider increasing timeout or optimizing query".to_string(),
                );
            }
            if error_lower.contains("permission") || error_lower.contains("denied") {
                error_patterns.push("Permission issues - verify access rights".to_string());
            }
            if error_lower.contains("not found") || error_lower.contains("404") {
                error_patterns.push("Resource not found - verify target existence".to_string());
            }
            if error_lower.contains("network") || error_lower.contains("connection") {
                error_patterns.push("Network instability - add retry logic".to_string());
            }
        }

        if record.success {
            reusable_patterns.push(format!(
                "Successfully completed: {}",
                record.task_description
            ));
        }

        if !record.tools_used.is_empty() {
            reusable_patterns.push(format!(
                "Tool combination: {}",
                record.tools_used.join(" -> ")
            ));
        }

        (error_patterns, reusable_patterns)
    }

    fn generate_knowledge_suggestions(&self, record: &TaskExecutionRecord) -> Vec<String> {
        let mut suggestions = Vec::new();

        if record.iterations > 5 {
            suggestions.push(
                "Consider caching intermediate reasoning results for similar tasks".to_string(),
            );
        }

        if record.error.is_some() {
            suggestions.push("Document error handling patterns for future reference".to_string());
        }

        if record.tools_used.len() > 3 {
            suggestions.push(
                "Multi-tool workflows can be optimized by reordering tool sequence".to_string(),
            );
        }

        suggestions
    }

    fn generate_improvement_suggestions(
        &self,
        record: &TaskExecutionRecord,
        reflection: &Reflection,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if reflection.quality_score < self.config.min_quality_threshold {
            suggestions.push(format!(
                "Quality score ({}) below threshold - review execution strategy",
                reflection.quality_score
            ));
        }

        if record.duration_ms > 30000 {
            suggestions.push("Consider enabling parallel execution for subtasks".to_string());
        }

        if !reflection.error_patterns.is_empty() {
            suggestions.push("Address identified error patterns in next iteration".to_string());
        }

        suggestions
    }

    fn generate_summary(&self, record: &TaskExecutionRecord, reflection: &Reflection) -> String {
        format!(
            "Task '{}' {} in {}ms with quality score {}/10. {} iterations, {} tools used. {} error patterns identified. {} reusable patterns found.",
            record.task_description,
            if record.success { "succeeded" } else { "failed" },
            record.duration_ms,
            reflection.quality_score,
            record.iterations,
            record.tools_used.len(),
            reflection.error_patterns.len(),
            reflection.reusable_patterns.len()
        )
    }

    pub async fn get_history(&self) -> Vec<Reflection> {
        self.history.read().await.clone()
    }

    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    pub fn get_insight_generator(&self) -> Arc<InsightGenerator> {
        Arc::clone(&self.insight_generator)
    }
}

impl Default for Reflector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reflection_creation() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(5);

        let mut record =
            TaskExecutionRecord::new("test-1".to_string(), "Test task".to_string(), start, end);
        record.compute_duration();
        record = record
            .with_success(true)
            .with_tools(vec!["tool1".to_string(), "tool2".to_string()]);

        let reflection = reflector.reflect(&record).await;

        assert_eq!(reflection.task_id, "test-1");
        assert!(reflection.quality_score >= 1 && reflection.quality_score <= 10);
        assert!(!reflection.overall_summary.is_empty());
    }
}
