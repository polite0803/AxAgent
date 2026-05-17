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
        let duration_delta = current.avg_duration_ms - baseline.avg_duration_ms;

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

    for (i, row) in matrix.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in matrix[0].iter_mut().enumerate() {
        *val = j;
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

// i18n-note: Difficulty labels used in benchmark reports. Future: accept language parameter.
pub fn get_difficulty_label(difficulty: Difficulty) -> &'static str {
    match difficulty {
        Difficulty::Easy => "简单",
        Difficulty::Medium => "中等",
        Difficulty::Hard => "困难",
        Difficulty::Expert => "专家",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_distance_empty() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", ""), 0);
    }

    #[test]
    fn test_levenshtein_distance_substitution() {
        assert_eq!(levenshtein_distance("cat", "bat"), 1);
    }

    #[test]
    fn test_levenshtein_distance_insertion_deletion() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_levenshtein_similarity_identical() {
        let sim = levenshtein_similarity("hello", "hello");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_levenshtein_similarity_empty() {
        let sim = levenshtein_similarity("", "");
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_levenshtein_similarity_partial() {
        let sim = levenshtein_similarity("abc", "axc");
        assert!(sim > 0.5 && sim < 1.0);
    }

    #[test]
    fn test_exact_match_score_same() {
        assert!((exact_match_score("hello", "hello") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_exact_match_score_different() {
        assert!((exact_match_score("hello", "world") - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_exact_match_score_trim() {
        assert!((exact_match_score("  hello  ", "hello") - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_contains_score_full_match() {
        let score = contains_score("hello,world", "hello world test");
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_contains_score_partial_match() {
        let score = contains_score("hello,missing", "hello test");
        assert!((score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_contains_score_no_match() {
        let score = contains_score("xyz", "hello world");
        assert!((score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_format_score() {
        assert_eq!(format_score(0.856), "85.60%");
        assert_eq!(format_score(1.0), "100.00%");
        assert_eq!(format_score(0.0), "0.00%");
    }

    #[test]
    fn test_get_difficulty_label() {
        assert_eq!(get_difficulty_label(Difficulty::Easy), "简单");
        assert_eq!(get_difficulty_label(Difficulty::Medium), "中等");
        assert_eq!(get_difficulty_label(Difficulty::Hard), "困难");
        assert_eq!(get_difficulty_label(Difficulty::Expert), "专家");
    }

    #[test]
    fn test_metrics_calculator_calculate_task_score() {
        let calc = MetricsCalculator::new();
        let task = BenchmarkTask {
            id: "t1".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            input: crate::evaluator::benchmark::TaskInput {
                query: "q".to_string(),
                context: None,
                constraints: vec![],
            },
            expected_output: None,
            evaluation_criteria: vec![
                crate::evaluator::benchmark::EvaluationCriteria {
                    name: "c1".to_string(),
                    metric: EvaluationMetric::ExactMatch,
                    weight: 0.6,
                    threshold: Some(0.5),
                },
                crate::evaluator::benchmark::EvaluationCriteria {
                    name: "c2".to_string(),
                    metric: EvaluationMetric::Contains,
                    weight: 0.4,
                    threshold: Some(0.5),
                },
            ],
            difficulty: Difficulty::Easy,
            tags: vec![],
        };
        let mut scores = std::collections::HashMap::new();
        scores.insert("c1".to_string(), 0.8);
        scores.insert("c2".to_string(), 0.6);
        let result = calc.calculate_task_score(&task, &scores);
        assert!((result.overall_score - (0.8 * 0.6 + 0.6 * 0.4)).abs() < 0.001);
        assert!(result.success);
    }

    #[test]
    fn test_metrics_calculator_aggregate_empty() {
        let calc = MetricsCalculator::new();
        let agg = calc.aggregate_task_metrics(&[]);
        assert_eq!(agg.total_tasks, 0);
        assert_eq!(agg.passed_tasks, 0);
        assert!((agg.pass_rate - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_metrics_calculator_compare_results() {
        let calc = MetricsCalculator::new();
        let baseline = AggregateMetrics {
            total_tasks: 2,
            passed_tasks: 1,
            failed_tasks: 1,
            pass_rate: 0.5,
            avg_duration_ms: 100.0,
            avg_score: 0.6,
            score_breakdown: std::collections::HashMap::new(),
            difficulty_distribution: std::collections::HashMap::new(),
        };
        let current = AggregateMetrics {
            total_tasks: 2,
            passed_tasks: 2,
            failed_tasks: 0,
            pass_rate: 1.0,
            avg_duration_ms: 80.0,
            avg_score: 0.8,
            score_breakdown: std::collections::HashMap::new(),
            difficulty_distribution: std::collections::HashMap::new(),
        };
        let comparison = calc.compare_results(&baseline, &current);
        assert!(comparison.score_improved);
        assert!(comparison.pass_rate_improved);
        assert!(comparison.duration_improved);
        assert!((comparison.score_delta - 0.2).abs() < 0.001);
    }

    #[test]
    fn test_evaluation_score_serialization() {
        let score = EvaluationScore {
            criteria_name: "test".to_string(),
            metric: EvaluationMetric::ExactMatch,
            raw_score: 0.9,
            weighted_score: 0.54,
            passed: true,
        };
        let json = serde_json::to_string(&score).unwrap();
        let de: EvaluationScore = serde_json::from_str(&json).unwrap();
        assert!((de.raw_score - 0.9).abs() < 0.001);
        assert!(de.passed);
    }
}
