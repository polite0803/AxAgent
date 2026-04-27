use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::evaluator::benchmark::{BenchmarkTask, Difficulty, EvaluationMetric};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationScore {
    pub criteria_name: String,
    pub metric: EvaluationMetric,
    pub raw_score: f32,
    pub weighted_score: f32,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub task_id: String,
    pub success: bool,
    pub duration_ms: u64,
    pub scores: Vec<EvaluationScore>,
    pub overall_score: f32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub pass_rate: f32,
    pub avg_duration_ms: f32,
    pub avg_score: f32,
    pub score_breakdown: HashMap<String, f32>,
    pub difficulty_distribution: HashMap<String, usize>,
}

pub struct MetricsCalculator;

impl MetricsCalculator {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_task_score(
        &self,
        task: &BenchmarkTask,
        scores: &HashMap<String, f32>,
    ) -> TaskMetrics {
        let mut eval_scores = Vec::new();
        let mut total_weighted = 0.0f32;

        for criteria in &task.evaluation_criteria {
            let raw_score = scores.get(&criteria.name).copied().unwrap_or(0.0);
            let weighted_score = raw_score * criteria.weight;
            total_weighted += weighted_score;

            let passed = criteria
                .threshold
                .map(|threshold| raw_score >= threshold)
                .unwrap_or(true);

            eval_scores.push(EvaluationScore {
                criteria_name: criteria.name.clone(),
                metric: criteria.metric,
                raw_score,
                weighted_score,
                passed,
            });
        }

        let overall_score = total_weighted;
        let success = eval_scores.iter().all(|s| s.passed) && overall_score >= 0.5;

        TaskMetrics {
            task_id: task.id.clone(),
            success,
            duration_ms: 0,
            scores: eval_scores,
            overall_score,
            error_message: None,
        }
    }

    pub fn aggregate_task_metrics(&self, task_metrics: &[TaskMetrics]) -> AggregateMetrics {
        let total_tasks = task_metrics.len();
        let passed_tasks = task_metrics.iter().filter(|m| m.success).count();
        let failed_tasks = total_tasks - passed_tasks;
        let pass_rate = if total_tasks > 0 {
            passed_tasks as f32 / total_tasks as f32
        } else {
            0.0
        };

        let total_duration: u64 = task_metrics.iter().map(|m| m.duration_ms).sum();
        let avg_duration_ms = if total_tasks > 0 {
            total_duration as f32 / total_tasks as f32
        } else {
            0.0
        };

        let total_score: f32 = task_metrics.iter().map(|m| m.overall_score).sum();
        let avg_score = if total_tasks > 0 {
            total_score / total_tasks as f32
        } else {
            0.0
        };

        let mut score_breakdown: HashMap<String, f32> = HashMap::new();
        let difficulty_distribution: HashMap<String, usize> = HashMap::new();

        for metric in task_metrics {
            for score in &metric.scores {
                *score_breakdown
                    .entry(score.criteria_name.clone())
                    .or_insert(0.0) += score.raw_score;
            }
        }

        let names: Vec<String> = score_breakdown.keys().cloned().collect();
        for name in names {
            let count = task_metrics
                .iter()
                .filter(|m| m.scores.iter().any(|s| s.criteria_name == name))
                .count();
            if count > 0 {
                *score_breakdown.get_mut(&name).unwrap() /= count as f32;
            }
        }

        AggregateMetrics {
            total_tasks,
            passed_tasks,
            failed_tasks,
            pass_rate,
            avg_duration_ms,
            avg_score,
            score_breakdown,
            difficulty_distribution,
        }
    }

    pub fn compare_results(
        &self,
        baseline: &AggregateMetrics,
        current: &AggregateMetrics,
    ) -> ComparisonResult {
        let score_delta = current.avg_score - baseline.avg_score;
        let pass_rate_delta = current.pass_rate - baseline.pass_rate;
        let duration_delta = current.avg_duration_ms as f32 - baseline.avg_duration_ms as f32;

        ComparisonResult {
            score_delta,
            score_improved: score_delta > 0.0,
            pass_rate_delta,
            pass_rate_improved: pass_rate_delta > 0.0,
            duration_delta_ms: duration_delta,
            duration_improved: duration_delta < 0.0,
        }
    }
}

impl Default for MetricsCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub score_delta: f32,
    pub score_improved: bool,
    pub pass_rate_delta: f32,
    pub pass_rate_improved: bool,
    pub duration_delta_ms: f32,
    pub duration_improved: bool,
}

pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len1][len2]
}

pub fn levenshtein_similarity(s1: &str, s2: &str) -> f32 {
    let max_len = s1.chars().count().max(s2.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let distance = levenshtein_distance(s1, s2);
    1.0 - (distance as f32 / max_len as f32)
}

pub fn exact_match_score(expected: &str, actual: &str) -> f32 {
    if expected.trim() == actual.trim() {
        1.0
    } else {
        0.0
    }
}

pub fn contains_score(expected: &str, actual: &str) -> f32 {
    let actual_lower = actual.to_lowercase();
    let expected_lower = expected.to_lowercase();
    let expected_parts: Vec<&str> = expected_lower.split(',').map(|s| s.trim()).collect();

    if expected_parts.is_empty() {
        return 0.0;
    }

    let matches = expected_parts
        .iter()
        .filter(|part| actual_lower.contains(*part))
        .count();

    matches as f32 / expected_parts.len() as f32
}

pub fn format_score(score: f32) -> String {
    format!("{:.2}%", score * 100.0)
}

pub fn get_difficulty_label(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "简单",
        Difficulty::Medium => "中等",
        Difficulty::Hard => "困难",
        Difficulty::Expert => "专家",
    }
}
