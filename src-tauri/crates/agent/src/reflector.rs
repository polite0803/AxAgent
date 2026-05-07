use crate::insight_generator::InsightGenerator;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
pub struct QualityMetrics {
    pub task_success_score: f32,
    pub tool_efficiency_score: f32,
    pub iteration_efficiency_score: f32,
    pub time_efficiency_score: f32,
    pub error_recovery_score: f32,
    pub goal_completion_score: f32,
    pub overall_weighted_score: f32,
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
    pub quality_metrics: Option<QualityMetrics>,
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
            quality_metrics: None,
        }
    }

    pub fn with_quality(mut self, score: u8, analysis: String) -> Self {
        self.quality_score = score.clamp(1, 10);
        self.quality_analysis = analysis;
        self.quality_metrics = None;
        self
    }

    pub fn with_quality_metrics(mut self, metrics: QualityMetrics) -> Self {
        self.quality_score = (metrics.overall_weighted_score.round() as u8).clamp(1, 10);
        self.quality_metrics = Some(metrics);
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

        let metrics = self.calculate_quality_metrics(record);
        reflection.quality_score = (metrics.overall_weighted_score.round() as u8).clamp(1, 10);
        reflection.quality_analysis = self.analyze_quality(record, &metrics);
        reflection.quality_metrics = Some(metrics);

        reflection.efficiency_analysis = self.analyze_efficiency(record);

        let (errors, reusable) = self.analyze_patterns(record);
        reflection.error_patterns = errors;
        reflection.reusable_patterns = reusable;

        let metrics_ref = reflection.quality_metrics.as_ref().unwrap();
        reflection.knowledge_suggestions = self.generate_knowledge_suggestions(record, metrics_ref);
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

    fn calculate_quality_metrics(&self, record: &TaskExecutionRecord) -> QualityMetrics {
        let task_success_score = if record.success { 10.0 } else { 0.0 };

        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);
        let unique_ratio = unique_tools as f32 / total_tools as f32;
        let iteration_ratio = (unique_tools as f32 / record.iterations.max(1) as f32).min(1.0);
        let tool_efficiency_score = unique_ratio * 5.0 + iteration_ratio * 5.0;

        let expected_iterations = (unique_tools * 2).max(1);
        let iteration_efficiency_score =
            (expected_iterations as f32 / record.iterations.max(1) as f32).min(1.0) * 10.0;

        let expected_duration = record.iterations.max(1) as u64 * 2000;
        let time_efficiency_score =
            (expected_duration as f32 / record.duration_ms.max(1) as f32).min(1.0) * 10.0;

        let error_recovery_score = if record.success {
            if record.iterations > expected_iterations {
                7.0
            } else {
                10.0
            }
        } else if record.error.is_some() {
            0.0
        } else {
            2.0
        };

        let goal_completion_score = if record.success {
            8.0 + (unique_tools as f32 * 0.4).min(2.0)
        } else {
            2.0 + (unique_tools as f32 * 0.3).min(3.0)
        };

        let overall_weighted_score = task_success_score * 0.30
            + tool_efficiency_score * 0.20
            + iteration_efficiency_score * 0.15
            + time_efficiency_score * 0.15
            + error_recovery_score * 0.10
            + goal_completion_score * 0.10;

        QualityMetrics {
            task_success_score,
            tool_efficiency_score,
            iteration_efficiency_score,
            time_efficiency_score,
            error_recovery_score,
            goal_completion_score,
            overall_weighted_score,
        }
    }

    fn analyze_quality(&self, record: &TaskExecutionRecord, metrics: &QualityMetrics) -> String {
        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);
        let unique_ratio = (unique_tools as f32 / total_tools as f32) * 100.0;
        let expected_iterations = (unique_tools * 2).max(1);
        let expected_duration = record.iterations.max(1) as u64 * 2000;

        let task_status = if record.success {
            "completed successfully"
        } else {
            "task failed"
        };

        let error_status = if record.success && record.iterations > expected_iterations {
            "recovered from intermediate errors"
        } else if record.success {
            "no errors encountered"
        } else if record.error.is_some() {
            "unresolved error"
        } else {
            "no explicit error"
        };

        let goal_status = if record.success {
            "all sub-goals addressed"
        } else {
            "partial goal completion"
        };

        format!(
            "Task Success: {:.1}/10 ({})\nTool Efficiency: {:.1}/10 ({} unique tools, {} total calls, {:.0}% unique ratio)\nIteration Efficiency: {:.1}/10 ({} iterations for complexity level {})\nTime Efficiency: {:.1}/10 ({}ms vs {}ms expected)\nError Recovery: {:.1}/10 ({})\nGoal Completion: {:.1}/10 ({})\nOverall Weighted Score: {:.1}/10",
            metrics.task_success_score,
            task_status,
            metrics.tool_efficiency_score,
            unique_tools,
            total_tools,
            unique_ratio,
            metrics.iteration_efficiency_score,
            record.iterations,
            expected_iterations,
            metrics.time_efficiency_score,
            record.duration_ms,
            expected_duration,
            metrics.error_recovery_score,
            error_status,
            metrics.goal_completion_score,
            goal_status,
            metrics.overall_weighted_score,
        )
    }

    fn analyze_efficiency(&self, record: &TaskExecutionRecord) -> String {
        let mut analysis = String::new();

        let duration_per_iteration = if record.iterations > 0 {
            record.duration_ms / record.iterations as u64
        } else {
            record.duration_ms
        };

        analysis.push_str(&format!("Total duration: {}ms. ", record.duration_ms));
        analysis.push_str(&format!("Duration per iteration: {}ms. ", duration_per_iteration));

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

        let sequence_patterns = Self::detect_tool_sequence_patterns(&record.tools_used);
        reusable_patterns.extend(sequence_patterns);

        let retry_patterns = Self::detect_retry_patterns(&record.tools_used);
        error_patterns.extend(retry_patterns);

        let redundant = Self::detect_redundant_tool_calls(&record.tools_used);
        error_patterns.extend(redundant);

        let unique_tools = Self::count_unique_tools(&record.tools_used);
        if record.success && record.iterations > unique_tools * 2 {
            reusable_patterns.push("Error recovery pattern: task succeeded despite high iteration count suggesting intermediate failures".to_string());
        }
        if !record.success && record.iterations > 10 {
            error_patterns.push(format!(
                "Extended retry without success: {} iterations exhausted without recovery",
                record.iterations
            ));
        }

        if record.success {
            reusable_patterns.push(format!("Successfully completed: {}", record.task_description));
        }

        if !record.tools_used.is_empty() {
            reusable_patterns.push(format!("Tool combination: {}", record.tools_used.join(" -> ")));
        }

        (error_patterns, reusable_patterns)
    }

    fn detect_tool_sequence_patterns(tools: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();

        let has_read = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("read") || l.contains("get") || l.contains("fetch")
        });
        let has_edit = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("edit")
                || l.contains("write")
                || l.contains("update")
                || l.contains("modify")
                || l.contains("patch")
        });
        let has_verify = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("test")
                || l.contains("verify")
                || l.contains("check")
                || l.contains("validate")
        });
        let has_search = tools.iter().any(|t| {
            let l = t.to_lowercase();
            l.contains("search")
                || l.contains("find")
                || l.contains("query")
                || l.contains("lookup")
        });

        if has_read && has_edit && has_verify {
            patterns.push("read->edit->verify pattern detected".to_string());
        }
        if has_search && has_read {
            patterns.push("search->read pattern detected".to_string());
        }
        if has_edit && has_verify {
            patterns.push("edit->verify pattern detected".to_string());
        }

        patterns
    }

    fn detect_retry_patterns(tools: &[String]) -> Vec<String> {
        let mut patterns = Vec::new();
        let mut tool_counts: Vec<(String, usize)> = Vec::new();

        for tool in tools {
            if let Some(entry) = tool_counts.iter_mut().find(|(name, _)| name == tool) {
                entry.1 += 1;
            } else {
                tool_counts.push((tool.clone(), 1));
            }
        }

        for (tool, count) in &tool_counts {
            if *count > 1 {
                patterns.push(format!("Retry with same approach: {} used {} times", tool, count));
            }
        }

        for i in 0..tools.len().saturating_sub(2) {
            if tools[i] == tools[i + 2] && tools[i] != tools[i + 1] {
                patterns.push(format!(
                    "Approach variation: {} -> {} -> {}",
                    tools[i],
                    tools[i + 1],
                    tools[i + 2]
                ));
            }
        }

        patterns
    }

    fn detect_redundant_tool_calls(tools: &[String]) -> Vec<String> {
        let mut redundant = Vec::new();

        for i in 0..tools.len().saturating_sub(1) {
            if tools[i] == tools[i + 1] {
                redundant.push(format!("Consecutive redundant call: {}", tools[i]));
            }
        }

        redundant
    }

    fn count_unique_tools(tools: &[String]) -> usize {
        tools.iter().collect::<HashSet<_>>().len()
    }

    fn generate_knowledge_suggestions(
        &self,
        record: &TaskExecutionRecord,
        metrics: &QualityMetrics,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        let unique_tools = Self::count_unique_tools(&record.tools_used);
        let total_tools = record.tools_used.len().max(1);

        if metrics.tool_efficiency_score < 5.0 {
            let ratio = (unique_tools as f32 / total_tools as f32) * 100.0;
            suggestions.push(format!(
                "Tool efficiency ({:.1}/10) below threshold - reduce redundant calls (unique: {}/{}, ratio: {:.0}%)",
                metrics.tool_efficiency_score, unique_tools, total_tools, ratio
            ));
        }

        if metrics.iteration_efficiency_score < 5.0 {
            suggestions.push(format!(
                "Iteration efficiency ({:.1}/10) indicates excessive iterations ({}) for task complexity - consider more direct approaches",
                metrics.iteration_efficiency_score, record.iterations
            ));
        }

        if metrics.time_efficiency_score < 5.0 {
            suggestions.push(format!(
                "Time efficiency ({:.1}/10) suggests slow execution ({}ms) - consider caching or parallel execution",
                metrics.time_efficiency_score, record.duration_ms
            ));
        }

        if metrics.error_recovery_score > 0.0 && metrics.error_recovery_score < 8.0 {
            suggestions
                .push("Document error recovery patterns for similar future tasks".to_string());
        }

        if record.success && metrics.overall_weighted_score >= 7.0 {
            suggestions.push(format!(
                "High-quality execution pattern (score {:.1}) - consider templating this workflow for reuse",
                metrics.overall_weighted_score
            ));
        }

        suggestions
    }

    fn generate_improvement_suggestions(
        &self,
        record: &TaskExecutionRecord,
        reflection: &Reflection,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        if let Some(metrics) = &reflection.quality_metrics {
            if metrics.task_success_score < 5.0 {
                suggestions.push(format!(
                    "Task success score ({:.1}/10) indicates failure - review error: {}",
                    metrics.task_success_score,
                    record.error.as_deref().unwrap_or("unknown")
                ));
            }

            if metrics.tool_efficiency_score < 5.0 {
                let redundant = Self::count_redundant_calls(&record.tools_used);
                suggestions.push(format!(
                    "Tool efficiency ({:.1}/10) below 5.0 threshold - {} redundant tool call(s) detected",
                    metrics.tool_efficiency_score, redundant
                ));
            }

            if metrics.iteration_efficiency_score < 5.0 {
                suggestions.push(format!(
                    "Iteration efficiency ({:.1}/10) - reduce iterations from {} by planning tool usage upfront",
                    metrics.iteration_efficiency_score, record.iterations
                ));
            }

            if metrics.time_efficiency_score < 5.0 {
                let expected = record.iterations.max(1) as u64 * 2000;
                suggestions.push(format!(
                    "Time efficiency ({:.1}/10) - execution took {}ms vs {}ms expected, enable parallel execution",
                    metrics.time_efficiency_score, record.duration_ms, expected
                ));
            }
        }

        if reflection.quality_score < self.config.min_quality_threshold {
            suggestions.push(format!(
                "Quality score ({}) below threshold ({}) - review overall execution strategy",
                reflection.quality_score, self.config.min_quality_threshold
            ));
        }

        if !reflection.error_patterns.is_empty() {
            suggestions.push(format!(
                "Address {} identified error pattern(s) before next iteration",
                reflection.error_patterns.len()
            ));
        }

        suggestions
    }

    fn count_redundant_calls(tools: &[String]) -> usize {
        let mut count = 0;
        for i in 0..tools.len().saturating_sub(1) {
            if tools[i] == tools[i + 1] {
                count += 1;
            }
        }
        count
    }

    fn generate_summary(&self, record: &TaskExecutionRecord, reflection: &Reflection) -> String {
        let metrics_detail = match &reflection.quality_metrics {
            Some(m) => format!(
                " Breakdown: success={:.1}, tool_eff={:.1}, iter_eff={:.1}, time_eff={:.1}, err_recov={:.1}, goal_comp={:.1}.",
                m.task_success_score,
                m.tool_efficiency_score,
                m.iteration_efficiency_score,
                m.time_efficiency_score,
                m.error_recovery_score,
                m.goal_completion_score
            ),
            None => String::new(),
        };
        format!(
            "Task '{}' {} in {}ms with quality score {}/10.{}{} iterations, {} tools used. {} error patterns identified. {} reusable patterns found.",
            record.task_description,
            if record.success { "succeeded" } else { "failed" },
            record.duration_ms,
            reflection.quality_score,
            metrics_detail,
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
        assert!(reflection.quality_metrics.is_some());
        let metrics = reflection.quality_metrics.unwrap();
        assert!(metrics.overall_weighted_score >= 0.0 && metrics.overall_weighted_score <= 10.0);
        assert_eq!(metrics.task_success_score, 10.0);
    }

    #[tokio::test]
    async fn test_quality_metrics_failed_task() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(30);

        let mut record =
            TaskExecutionRecord::new("test-2".to_string(), "Failed task".to_string(), start, end);
        record.compute_duration();
        record = record
            .with_error("timeout: connection refused".to_string())
            .with_tools(vec![
                "search".to_string(),
                "search".to_string(),
                "read".to_string(),
            ])
            .with_iterations(15);

        let reflection = reflector.reflect(&record).await;

        assert!(reflection.quality_score < 5);
        let metrics = reflection.quality_metrics.unwrap();
        assert_eq!(metrics.task_success_score, 0.0);
        assert!(metrics.tool_efficiency_score < 7.0);
        assert!(metrics.error_recovery_score < 1.0);
    }

    #[tokio::test]
    async fn test_tool_sequence_detection() {
        let patterns = Reflector::detect_tool_sequence_patterns(&[
            "read_file".to_string(),
            "edit_file".to_string(),
            "test_runner".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("read->edit->verify")));

        let patterns = Reflector::detect_tool_sequence_patterns(&[
            "search_code".to_string(),
            "read_file".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("search->read")));
    }

    #[tokio::test]
    async fn test_retry_pattern_detection() {
        let patterns = Reflector::detect_retry_patterns(&[
            "search".to_string(),
            "search".to_string(),
            "read".to_string(),
        ]);
        assert!(patterns
            .iter()
            .any(|p| p.contains("Retry with same approach") && p.contains("search")));

        let patterns = Reflector::detect_retry_patterns(&[
            "search".to_string(),
            "read".to_string(),
            "search".to_string(),
        ]);
        assert!(patterns.iter().any(|p| p.contains("Approach variation")));
    }

    #[tokio::test]
    async fn test_redundant_call_detection() {
        let redundant = Reflector::detect_redundant_tool_calls(&[
            "read".to_string(),
            "read".to_string(),
            "edit".to_string(),
        ]);
        assert_eq!(redundant.len(), 1);
        assert!(redundant[0].contains("read"));
    }

    #[tokio::test]
    async fn test_error_recovery_scoring() {
        let reflector = Reflector::new();

        let start = Utc::now();
        let end = start + chrono::Duration::seconds(10);

        let mut record =
            TaskExecutionRecord::new("test-3".to_string(), "Recovery task".to_string(), start, end);
        record.compute_duration();
        record = record
            .with_success(true)
            .with_tools(vec!["read".to_string(), "edit".to_string()])
            .with_iterations(8);

        let reflection = reflector.reflect(&record).await;
        let metrics = reflection.quality_metrics.unwrap();
        assert_eq!(metrics.error_recovery_score, 7.0);
    }
}
