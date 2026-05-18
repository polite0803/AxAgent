use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evaluator::metrics::{format_score, get_difficulty_label};
use crate::evaluator::runner::{BenchmarkResult, TaskResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub benchmark_id: String,
    pub benchmark_name: String,
    pub generated_at: DateTime<Utc>,
    pub summary: ReportSummary,
    pub task_breakdown: Vec<TaskBreakdown>,
    pub category_scores: HashMap<String, f32>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSummary {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub pass_rate: f32,
    pub overall_score: f32,
    pub total_duration_ms: u64,
    pub avg_task_duration_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBreakdown {
    pub task_id: String,
    pub task_name: String,
    pub difficulty: String,
    pub success: bool,
    pub score: f32,
    pub duration_ms: u64,
    pub criteria_scores: Vec<CriteriaScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriteriaScore {
    pub name: String,
    pub score: f32,
    pub passed: bool,
}

pub struct ReportGenerator {
    include_recommendations: bool,
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            include_recommendations: true,
        }
    }

    pub fn with_recommendations(mut self, include: bool) -> Self {
        self.include_recommendations = include;
        self
    }

    pub fn generate(&self, result: &BenchmarkResult) -> BenchmarkReport {
        let summary = self.generate_summary(result);
        let task_breakdown = self.generate_task_breakdown(&result.task_results);
        let category_scores = self.calculate_category_scores(&result.task_results);
        let recommendations = if self.include_recommendations {
            self.generate_recommendations(&summary, &task_breakdown)
        } else {
            vec![]
        };

        BenchmarkReport {
            benchmark_id: result.benchmark_id.clone(),
            benchmark_name: result.benchmark_name.clone(),
            generated_at: Utc::now(),
            summary,
            task_breakdown,
            category_scores,
            recommendations,
        }
    }

    fn generate_summary(&self, result: &BenchmarkResult) -> ReportSummary {
        let total_tasks = result.task_results.len();
        let passed_tasks = result.task_results.iter().filter(|t| t.success).count();
        let failed_tasks = total_tasks - passed_tasks;

        ReportSummary {
            total_tasks,
            passed_tasks,
            failed_tasks,
            pass_rate: result.aggregate.pass_rate,
            overall_score: result.aggregate.avg_score,
            total_duration_ms: result.duration_ms,
            avg_task_duration_ms: result.aggregate.avg_duration_ms,
        }
    }

    fn generate_task_breakdown(&self, tasks: &[TaskResult]) -> Vec<TaskBreakdown> {
        tasks
            .iter()
            .map(|task| TaskBreakdown {
                task_id: task.task_id.clone(),
                task_name: task.task_name.clone(),
                difficulty: get_difficulty_label(task.difficulty).to_string(),
                success: task.success,
                score: task.overall_score,
                duration_ms: task.duration_ms,
                criteria_scores: task
                    .scores
                    .iter()
                    .map(|s| CriteriaScore {
                        name: s.criteria_name.clone(),
                        score: s.raw_score,
                        passed: s.passed,
                    })
                    .collect(),
            })
            .collect()
    }

    fn calculate_category_scores(&self, tasks: &[TaskResult]) -> HashMap<String, f32> {
        let mut scores: HashMap<String, Vec<f32>> = HashMap::new();

        for task in tasks {
            let category = match task.difficulty {
                crate::evaluator::benchmark::Difficulty::Easy => "easy",
                crate::evaluator::benchmark::Difficulty::Medium => "medium",
                crate::evaluator::benchmark::Difficulty::Hard => "hard",
                crate::evaluator::benchmark::Difficulty::Expert => "expert",
            };
            scores
                .entry(category.to_string())
                .or_default()
                .push(task.overall_score);
        }

        scores
            .into_iter()
            .map(|(k, v)| {
                let avg = v.iter().sum::<f32>() / v.len() as f32;
                (k, avg)
            })
            .collect()
    }

    // i18n-exempt: Generated benchmark report content (recommendations, markdown templates) — internal/report data, not UI
    fn generate_recommendations(
        &self,
        summary: &ReportSummary,
        tasks: &[TaskBreakdown],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if summary.pass_rate < 0.5 {
            recommendations.push("整体通过率较低，建议先进行基础能力训练".to_string());
        }

        let failed_tasks: Vec<_> = tasks.iter().filter(|t| !t.success).collect();
        if !failed_tasks.is_empty() {
            recommendations.push(format!(
                "{} 个任务未通过，建议针对性训练: {}",
                failed_tasks.len(),
                failed_tasks
                    .iter()
                    .map(|t| t.task_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        let low_score_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| t.score < 0.6 && t.success)
            .collect();
        if !low_score_tasks.is_empty() {
            recommendations
                .push(format!("{} 个任务分数偏低(60%以下)，有改进空间", low_score_tasks.len()));
        }

        if summary.avg_task_duration_ms > 30000.0 {
            recommendations.push("平均任务执行时间较长，考虑优化处理流程".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("表现优秀，继续保持！".to_string());
        }

        recommendations
    }

    // i18n-exempt: Markdown report template — generated report output, not UI
    pub fn to_markdown(&self, report: &BenchmarkReport) -> String {
        let mut md = String::new();

        md.push_str(&format!("# 基准测试报告: {}\n\n", report.benchmark_name));
        md.push_str(&format!(
            "**生成时间**: {}\n\n",
            report.generated_at.format("%Y-%m-%d %H:%M:%S")
        ));

        md.push_str("## 总体概览\n\n");
        md.push_str("| 指标 | 值 |\n|:---|---|\n");
        md.push_str(&format!("| 总任务数 | {} |\n", report.summary.total_tasks));
        md.push_str(&format!("| 通过数 | {} |\n", report.summary.passed_tasks));
        md.push_str(&format!("| 失败数 | {} |\n", report.summary.failed_tasks));
        md.push_str(&format!("| 通过率 | {} |\n", format_score(report.summary.pass_rate)));
        md.push_str(&format!("| 总体得分 | {} |\n", format_score(report.summary.overall_score)));
        md.push_str(&format!("| 总耗时 | {}ms |\n", report.summary.total_duration_ms));
        md.push_str(&format!("| 平均耗时 | {:.0}ms |\n", report.summary.avg_task_duration_ms));
        md.push_str("\n## 任务详情\n\n");

        for task in &report.task_breakdown {
            md.push_str(&format!("### {} ({})\n\n", task.task_name, task.difficulty));
            md.push_str(&format!(
                "- **状态**: {} |\n",
                if task.success {
                    "✅ 通过"
                } else {
                    "❌ 失败"
                }
            ));
            md.push_str(&format!("- **得分**: {} |\n", format_score(task.score)));
            md.push_str(&format!("- **耗时**: {}ms |\n\n", task.duration_ms));

            md.push_str("| 评估项 | 得分 | 通过 |\n|:---|:---|:---|\n");
            for criteria in &task.criteria_scores {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    criteria.name,
                    format_score(criteria.score),
                    if criteria.passed { "✅" } else { "❌" }
                ));
            }
            md.push('\n');
        }

        if !report.category_scores.is_empty() {
            md.push_str("## 分类得分\n\n");
            md.push_str("| 分类 | 平均得分 |\n|:---|:---|\n");
            for (category, score) in &report.category_scores {
                md.push_str(&format!("| {} | {} |\n", category, format_score(*score)));
            }
            md.push('\n');
        }

        if !report.recommendations.is_empty() {
            md.push_str("## 建议\n\n");
            for (i, rec) in report.recommendations.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, rec));
            }
        }

        md
    }

    pub fn to_json(&self, report: &BenchmarkReport) -> String {
        serde_json::to_string_pretty(report).unwrap_or_default()
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReportHistory {
    reports: Vec<BenchmarkReport>,
}

impl ReportHistory {
    pub fn new() -> Self {
        Self { reports: vec![] }
    }

    pub fn add(&mut self, report: BenchmarkReport) {
        self.reports.push(report);
    }

    pub fn get_all(&self) -> &[BenchmarkReport] {
        &self.reports
    }

    pub fn latest(&self) -> Option<&BenchmarkReport> {
        self.reports.last()
    }

    pub fn clear(&mut self) {
        self.reports.clear();
    }

    pub fn len(&self) -> usize {
        self.reports.len()
    }

    pub fn is_empty(&self) -> bool {
        self.reports.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::benchmark::Difficulty;
    use crate::evaluator::metrics::AggregateMetrics;
    use crate::evaluator::runner::{BenchmarkResult, RunnerConfig, ScoreResult, TaskResult};

    fn make_benchmark_result() -> BenchmarkResult {
        BenchmarkResult {
            benchmark_id: "test".to_string(),
            benchmark_name: "Test Benchmark".to_string(),
            run_at: Utc::now(),
            config: RunnerConfig::default(),
            task_results: vec![
                TaskResult {
                    task_id: "t1".to_string(),
                    task_name: "Task 1".to_string(),
                    difficulty: Difficulty::Easy,
                    success: true,
                    duration_ms: 100,
                    scores: vec![ScoreResult {
                        criteria_name: "accuracy".to_string(),
                        metric: crate::evaluator::benchmark::EvaluationMetric::ExactMatch,
                        raw_score: 1.0,
                        weighted_score: 1.0,
                        passed: true,
                    }],
                    overall_score: 1.0,
                    response: Some("response".to_string()),
                    error: None,
                    trace_id: None,
                },
                TaskResult {
                    task_id: "t2".to_string(),
                    task_name: "Task 2".to_string(),
                    difficulty: Difficulty::Hard,
                    success: false,
                    duration_ms: 200,
                    scores: vec![ScoreResult {
                        criteria_name: "accuracy".to_string(),
                        metric: crate::evaluator::benchmark::EvaluationMetric::ExactMatch,
                        raw_score: 0.3,
                        weighted_score: 0.3,
                        passed: false,
                    }],
                    overall_score: 0.3,
                    response: None,
                    error: Some("error".to_string()),
                    trace_id: None,
                },
            ],
            aggregate: AggregateMetrics {
                total_tasks: 2,
                passed_tasks: 1,
                failed_tasks: 1,
                pass_rate: 0.5,
                avg_duration_ms: 150.0,
                avg_score: 0.65,
                score_breakdown: std::collections::HashMap::new(),
                difficulty_distribution: std::collections::HashMap::new(),
            },
            duration_ms: 300,
        }
    }

    #[test]
    fn test_report_generator_new() {
        let generator = ReportGenerator::new();
        assert!(generator.include_recommendations);
    }

    #[test]
    fn test_report_generator_with_recommendations_false() {
        let generator = ReportGenerator::new().with_recommendations(false);
        assert!(!generator.include_recommendations);
    }

    #[test]
    fn test_report_generator_generate() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert_eq!(report.benchmark_id, "test");
        assert_eq!(report.summary.total_tasks, 2);
        assert_eq!(report.summary.passed_tasks, 1);
        assert_eq!(report.summary.failed_tasks, 1);
        assert!(!report.task_breakdown.is_empty());
    }

    #[test]
    fn test_report_generator_generate_without_recommendations() {
        let generator = ReportGenerator::new().with_recommendations(false);
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert!(report.recommendations.is_empty());
    }

    #[test]
    fn test_report_generator_generate_with_recommendations() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_report_generator_to_markdown() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        let md = generator.to_markdown(&report);
        assert!(md.contains("基准测试报告"));
        assert!(md.contains("总体概览"));
        assert!(md.contains("任务详情"));
    }

    #[test]
    fn test_report_generator_to_json() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        let json = generator.to_json(&report);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["benchmark_id"], "test");
    }

    #[test]
    fn test_report_history_new() {
        let history = ReportHistory::new();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_report_history_add_and_get() {
        let mut history = ReportHistory::new();
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        history.add(report.clone());
        assert_eq!(history.len(), 1);
        assert!(!history.is_empty());
        let latest = history.latest();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().benchmark_id, "test");
    }

    #[test]
    fn test_report_history_clear() {
        let mut history = ReportHistory::new();
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        history.add(report);
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn test_report_history_latest_empty() {
        let history = ReportHistory::new();
        assert!(history.latest().is_none());
    }

    #[test]
    fn test_report_summary_fields() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert!((report.summary.pass_rate - 0.5).abs() < 0.001);
        assert_eq!(report.summary.total_duration_ms, 300);
    }

    #[test]
    fn test_task_breakdown_difficulty_labels() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert_eq!(report.task_breakdown[0].difficulty, "简单");
        assert_eq!(report.task_breakdown[1].difficulty, "困难");
    }

    #[test]
    fn test_report_category_scores() {
        let generator = ReportGenerator::new();
        let result = make_benchmark_result();
        let report = generator.generate(&result);
        assert!(!report.category_scores.is_empty());
    }

    #[test]
    fn test_criteria_score() {
        let cs = CriteriaScore {
            name: "test".to_string(),
            score: 0.9,
            passed: true,
        };
        assert!((cs.score - 0.9).abs() < 0.001);
        assert!(cs.passed);
    }
}
