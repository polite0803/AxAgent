use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent_config::AgentConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentGroup {
    Control,
    Treatment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentMetric {
    TaskCompletionRate,
    AverageIterations,
    AverageDuration,
    ToolEfficiency,
    QualityScore,
    ErrorRate,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperimentStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    pub id: String,
    pub name: String,
    pub description: String,
    pub control_config: AgentConfig,
    pub treatment_config: AgentConfig,
    pub sample_size: usize,
    pub metrics: Vec<ExperimentMetric>,
    pub created_at: DateTime<Utc>,
    pub status: ExperimentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupStats {
    pub group: ExperimentGroup,
    pub sample_count: usize,
    pub task_completion_rate: f32,
    pub avg_iterations: f32,
    pub avg_duration_ms: f64,
    pub tool_efficiency: f32,
    pub avg_quality_score: f32,
    pub error_rate: f32,
    pub custom_metrics: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricComparison {
    pub metric: ExperimentMetric,
    pub control_value: f32,
    pub treatment_value: f32,
    pub improvement: f32,
    pub is_significant: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub experiment_id: String,
    pub control_stats: GroupStats,
    pub treatment_stats: GroupStats,
    pub metric_comparisons: Vec<MetricComparison>,
    pub winner: Option<ExperimentGroup>,
    pub confidence_level: f32,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialResult {
    pub experiment_id: String,
    pub group: ExperimentGroup,
    pub task_completed: bool,
    pub iterations: usize,
    pub duration_ms: u64,
    pub tools_used: usize,
    pub unique_tools: usize,
    pub quality_score: f32,
    pub had_errors: bool,
    pub custom_metrics: HashMap<String, f32>,
}

pub struct ExperimentRunner {
    experiments: Arc<RwLock<HashMap<String, ExperimentConfig>>>,
    trial_results: Arc<RwLock<HashMap<String, Vec<TrialResult>>>>,
}

impl ExperimentRunner {
    pub fn new() -> Self {
        Self {
            experiments: Arc::new(RwLock::new(HashMap::new())),
            trial_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_experiment(&self, config: ExperimentConfig) -> Result<(), String> {
        let id = config.id.clone();
        let mut experiments = self.experiments.write().await;
        if experiments.contains_key(&id) {
            return Err(format!("Experiment with id '{}' already exists", id));
        }
        experiments.insert(id, config);
        Ok(())
    }

    pub async fn start_experiment(&self, id: &str) -> Result<(), String> {
        let mut experiments = self.experiments.write().await;
        let config = experiments
            .get_mut(id)
            .ok_or_else(|| format!("Experiment '{}' not found", id))?;
        if config.status != ExperimentStatus::Draft && config.status != ExperimentStatus::Paused {
            return Err(format!("Cannot start experiment in {:?} status", config.status));
        }
        config.status = ExperimentStatus::Running;
        Ok(())
    }

    pub async fn pause_experiment(&self, id: &str) -> Result<(), String> {
        let mut experiments = self.experiments.write().await;
        let config = experiments
            .get_mut(id)
            .ok_or_else(|| format!("Experiment '{}' not found", id))?;
        if config.status != ExperimentStatus::Running {
            return Err(format!("Cannot pause experiment in {:?} status", config.status));
        }
        config.status = ExperimentStatus::Paused;
        Ok(())
    }

    pub async fn cancel_experiment(&self, id: &str) -> Result<(), String> {
        let mut experiments = self.experiments.write().await;
        let config = experiments
            .get_mut(id)
            .ok_or_else(|| format!("Experiment '{}' not found", id))?;
        if config.status == ExperimentStatus::Completed
            || config.status == ExperimentStatus::Cancelled
        {
            return Err(format!("Cannot cancel experiment in {:?} status", config.status));
        }
        config.status = ExperimentStatus::Cancelled;
        Ok(())
    }

    pub async fn record_trial(&self, result: TrialResult) -> Result<(), String> {
        let experiments = self.experiments.read().await;
        let config = experiments
            .get(&result.experiment_id)
            .ok_or_else(|| format!("Experiment '{}' not found", result.experiment_id))?;
        if config.status != ExperimentStatus::Running {
            return Err(format!(
                "Cannot record trial for experiment in {:?} status",
                config.status
            ));
        }
        drop(experiments);

        let mut trial_results = self.trial_results.write().await;
        trial_results
            .entry(result.experiment_id.clone())
            .or_insert_with(Vec::new)
            .push(result);
        Ok(())
    }

    pub async fn get_results(
        &self,
        experiment_id: &str,
    ) -> Result<Option<ExperimentResult>, String> {
        let experiments = self.experiments.read().await;
        let config = experiments
            .get(experiment_id)
            .ok_or_else(|| format!("Experiment '{}' not found", experiment_id))?
            .clone();
        drop(experiments);

        let trial_results = self.trial_results.read().await;
        let results = trial_results.get(experiment_id);

        let control_trials: Vec<&TrialResult> = results
            .map(|r| {
                r.iter()
                    .filter(|t| t.group == ExperimentGroup::Control)
                    .collect()
            })
            .unwrap_or_default();
        let treatment_trials: Vec<&TrialResult> = results
            .map(|r| {
                r.iter()
                    .filter(|t| t.group == ExperimentGroup::Treatment)
                    .collect()
            })
            .unwrap_or_default();

        let control_stats = Self::compute_group_stats(ExperimentGroup::Control, &control_trials);
        let treatment_stats =
            Self::compute_group_stats(ExperimentGroup::Treatment, &treatment_trials);

        let metric_comparisons =
            Self::compute_metric_comparisons(&config.metrics, &control_stats, &treatment_stats);

        let winner = Self::determine_winner(&metric_comparisons);
        let confidence_level = Self::compute_confidence_level(&metric_comparisons);

        Ok(Some(ExperimentResult {
            experiment_id: experiment_id.to_string(),
            control_stats,
            treatment_stats,
            metric_comparisons,
            winner,
            confidence_level,
            completed_at: Utc::now(),
        }))
    }

    pub async fn list_experiments(&self) -> Vec<ExperimentConfig> {
        let experiments = self.experiments.read().await;
        experiments.values().cloned().collect()
    }

    pub async fn get_experiment(&self, id: &str) -> Option<ExperimentConfig> {
        let experiments = self.experiments.read().await;
        experiments.get(id).cloned()
    }

    fn compute_group_stats(group: ExperimentGroup, trials: &[&TrialResult]) -> GroupStats {
        if trials.is_empty() {
            return GroupStats {
                group,
                sample_count: 0,
                task_completion_rate: 0.0,
                avg_iterations: 0.0,
                avg_duration_ms: 0.0,
                tool_efficiency: 0.0,
                avg_quality_score: 0.0,
                error_rate: 0.0,
                custom_metrics: HashMap::new(),
            };
        }

        let n = trials.len();
        let completed = trials.iter().filter(|t| t.task_completed).count();
        let task_completion_rate = completed as f32 / n as f32;

        let avg_iterations = trials.iter().map(|t| t.iterations as f32).sum::<f32>() / n as f32;

        let avg_duration_ms = trials.iter().map(|t| t.duration_ms as f64).sum::<f64>() / n as f64;

        let total_tools: f32 = trials.iter().map(|t| t.tools_used as f32).sum();
        let unique_tools: f32 = trials.iter().map(|t| t.unique_tools as f32).sum();
        let tool_efficiency = if unique_tools > 0.0 {
            unique_tools / total_tools
        } else {
            0.0
        };

        let avg_quality_score = trials.iter().map(|t| t.quality_score).sum::<f32>() / n as f32;

        let errors = trials.iter().filter(|t| t.had_errors).count();
        let error_rate = errors as f32 / n as f32;

        let custom_metrics = Self::aggregate_custom_metrics(trials);

        GroupStats {
            group,
            sample_count: n,
            task_completion_rate,
            avg_iterations,
            avg_duration_ms,
            tool_efficiency,
            avg_quality_score,
            error_rate,
            custom_metrics,
        }
    }

    fn aggregate_custom_metrics(trials: &[&TrialResult]) -> HashMap<String, f32> {
        let mut sums: HashMap<String, f32> = HashMap::new();
        let mut counts: HashMap<String, usize> = HashMap::new();

        for trial in trials {
            for (key, value) in &trial.custom_metrics {
                *sums.entry(key.clone()).or_insert(0.0) += value;
                *counts.entry(key.clone()).or_insert(0) += 1;
            }
        }

        let mut averages = HashMap::new();
        for (key, sum) in &sums {
            let count = counts[key];
            averages.insert(key.clone(), sum / count as f32);
        }
        averages
    }

    fn compute_metric_comparisons(
        metrics: &[ExperimentMetric],
        control: &GroupStats,
        treatment: &GroupStats,
    ) -> Vec<MetricComparison> {
        metrics
            .iter()
            .map(|metric| {
                let (control_value, treatment_value) =
                    Self::get_metric_values(metric, control, treatment);
                let improvement = if control_value.abs() > f32::EPSILON {
                    (treatment_value - control_value) / control_value.abs()
                } else if treatment_value.abs() > f32::EPSILON {
                    1.0
                } else {
                    0.0
                };
                let is_significant = Self::is_statistically_significant(
                    metric,
                    control_value,
                    treatment_value,
                    control.sample_count,
                    treatment.sample_count,
                );
                MetricComparison {
                    metric: metric.clone(),
                    control_value,
                    treatment_value,
                    improvement,
                    is_significant,
                }
            })
            .collect()
    }

    fn get_metric_values(
        metric: &ExperimentMetric,
        control: &GroupStats,
        treatment: &GroupStats,
    ) -> (f32, f32) {
        match metric {
            ExperimentMetric::TaskCompletionRate => {
                (control.task_completion_rate, treatment.task_completion_rate)
            },
            ExperimentMetric::AverageIterations => {
                (control.avg_iterations, treatment.avg_iterations)
            },
            ExperimentMetric::AverageDuration => {
                (control.avg_duration_ms as f32, treatment.avg_duration_ms as f32)
            },
            ExperimentMetric::ToolEfficiency => {
                (control.tool_efficiency, treatment.tool_efficiency)
            },
            ExperimentMetric::QualityScore => {
                (control.avg_quality_score, treatment.avg_quality_score)
            },
            ExperimentMetric::ErrorRate => (control.error_rate, treatment.error_rate),
            ExperimentMetric::Custom(name) => (
                control.custom_metrics.get(name).copied().unwrap_or(0.0),
                treatment.custom_metrics.get(name).copied().unwrap_or(0.0),
            ),
        }
    }

    fn is_statistically_significant(
        metric: &ExperimentMetric,
        control_value: f32,
        treatment_value: f32,
        control_n: usize,
        treatment_n: usize,
    ) -> bool {
        if control_n < 10 || treatment_n < 10 {
            return false;
        }

        let improvement = if control_value.abs() > f32::EPSILON {
            (treatment_value - control_value) / control_value.abs()
        } else {
            return false;
        };

        let improvement_pct = improvement.abs();
        if improvement_pct <= 0.05 {
            return false;
        }

        match metric {
            ExperimentMetric::TaskCompletionRate => {
                let p1 = control_value;
                let p2 = treatment_value;
                let p_pool = (p1 * control_n as f32 + p2 * treatment_n as f32)
                    / (control_n + treatment_n) as f32;
                if p_pool <= 0.0 || p_pool >= 1.0 {
                    return false;
                }
                let se =
                    (p_pool * (1.0 - p_pool) * (1.0 / control_n as f32 + 1.0 / treatment_n as f32))
                        .sqrt();
                if se < f32::EPSILON {
                    return false;
                }
                let z = (p2 - p1) / se;
                z.abs() > 1.96
            },
            _ => {
                let pooled_se = ((control_value * (1.0 - control_value)).max(0.001)
                    / control_n as f32
                    + (treatment_value * (1.0 - treatment_value)).max(0.001) / treatment_n as f32)
                    .sqrt();
                if pooled_se < f32::EPSILON {
                    return false;
                }
                let t_stat = (treatment_value - control_value) / pooled_se;
                t_stat.abs() > 2.0
            },
        }
    }

    fn determine_winner(comparisons: &[MetricComparison]) -> Option<ExperimentGroup> {
        if comparisons.is_empty() {
            return None;
        }

        let significant: Vec<&MetricComparison> =
            comparisons.iter().filter(|c| c.is_significant).collect();

        if significant.is_empty() {
            return None;
        }

        let control_wins = significant.iter().filter(|c| c.improvement < 0.0).count();
        let treatment_wins = significant.iter().filter(|c| c.improvement > 0.0).count();

        if treatment_wins > control_wins {
            Some(ExperimentGroup::Treatment)
        } else if control_wins > treatment_wins {
            Some(ExperimentGroup::Control)
        } else {
            None
        }
    }

    fn compute_confidence_level(comparisons: &[MetricComparison]) -> f32 {
        if comparisons.is_empty() {
            return 0.0;
        }

        let significant_count = comparisons.iter().filter(|c| c.is_significant).count();
        significant_count as f32 / comparisons.len() as f32
    }
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(id: &str) -> ExperimentConfig {
        ExperimentConfig {
            id: id.to_string(),
            name: format!("Test {}", id),
            description: "A test experiment".to_string(),
            control_config: AgentConfig::default(),
            treatment_config: AgentConfig::default(),
            sample_size: 100,
            metrics: vec![ExperimentMetric::TaskCompletionRate],
            created_at: Utc::now(),
            status: ExperimentStatus::Draft,
        }
    }

    fn trial_result(
        experiment_id: &str,
        group: ExperimentGroup,
        completed: bool,
        iterations: usize,
        duration_ms: u64,
        quality: f32,
        had_errors: bool,
    ) -> TrialResult {
        TrialResult {
            experiment_id: experiment_id.to_string(),
            group,
            task_completed: completed,
            iterations,
            duration_ms,
            tools_used: 5,
            unique_tools: 3,
            quality_score: quality,
            had_errors,
            custom_metrics: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_create_experiment() {
        let runner = ExperimentRunner::new();
        let config = test_config("exp-1");
        assert!(runner.create_experiment(config).await.is_ok());
        let exp = runner.get_experiment("exp-1").await;
        assert!(exp.is_some());
        assert_eq!(exp.unwrap().name, "Test exp-1");
    }

    #[tokio::test]
    async fn test_create_duplicate_experiment() {
        let runner = ExperimentRunner::new();
        let config = test_config("exp-dup");
        assert!(runner.create_experiment(config.clone()).await.is_ok());
        let result = runner.create_experiment(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[tokio::test]
    async fn test_start_experiment_from_draft() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-start"))
            .await
            .unwrap();
        assert!(runner.start_experiment("exp-start").await.is_ok());
        let exp = runner.get_experiment("exp-start").await.unwrap();
        assert_eq!(exp.status, ExperimentStatus::Running);
    }

    #[tokio::test]
    async fn test_start_experiment_from_paused() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-resume"))
            .await
            .unwrap();
        runner.start_experiment("exp-resume").await.unwrap();
        runner.pause_experiment("exp-resume").await.unwrap();
        assert!(runner.start_experiment("exp-resume").await.is_ok());
        let exp = runner.get_experiment("exp-resume").await.unwrap();
        assert_eq!(exp.status, ExperimentStatus::Running);
    }

    #[tokio::test]
    async fn test_start_experiment_invalid_state() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-bad-start"))
            .await
            .unwrap();
        runner.start_experiment("exp-bad-start").await.unwrap();
        let result = runner.start_experiment("exp-bad-start").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot start"));
    }

    #[tokio::test]
    async fn test_pause_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-pause"))
            .await
            .unwrap();
        runner.start_experiment("exp-pause").await.unwrap();
        assert!(runner.pause_experiment("exp-pause").await.is_ok());
        let exp = runner.get_experiment("exp-pause").await.unwrap();
        assert_eq!(exp.status, ExperimentStatus::Paused);
    }

    #[tokio::test]
    async fn test_pause_non_running_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-pause-bad"))
            .await
            .unwrap();
        let result = runner.pause_experiment("exp-pause-bad").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot pause"));
    }

    #[tokio::test]
    async fn test_cancel_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-cancel"))
            .await
            .unwrap();
        runner.start_experiment("exp-cancel").await.unwrap();
        assert!(runner.cancel_experiment("exp-cancel").await.is_ok());
        let exp = runner.get_experiment("exp-cancel").await.unwrap();
        assert_eq!(exp.status, ExperimentStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_cancel_completed_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-cancel-done"))
            .await
            .unwrap();
        runner.start_experiment("exp-cancel-done").await.unwrap();
        let mut exp = runner.get_experiment("exp-cancel-done").await.unwrap();
        exp.status = ExperimentStatus::Completed;
        {
            let mut experiments = runner.experiments.write().await;
            experiments.insert("exp-cancel-done".to_string(), exp);
        }
        let result = runner.cancel_experiment("exp-cancel-done").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot cancel"));
    }

    #[tokio::test]
    async fn test_record_trial_running_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-trial"))
            .await
            .unwrap();
        runner.start_experiment("exp-trial").await.unwrap();
        let trial = trial_result("exp-trial", ExperimentGroup::Control, true, 5, 1000, 0.9, false);
        assert!(runner.record_trial(trial).await.is_ok());
    }

    #[tokio::test]
    async fn test_record_trial_non_running_experiment() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-trial-bad"))
            .await
            .unwrap();
        let trial =
            trial_result("exp-trial-bad", ExperimentGroup::Control, true, 5, 1000, 0.9, false);
        let result = runner.record_trial(trial).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot record trial"));
    }

    #[tokio::test]
    async fn test_record_trial_nonexistent_experiment() {
        let runner = ExperimentRunner::new();
        let trial =
            trial_result("no-such-exp", ExperimentGroup::Control, true, 5, 1000, 0.9, false);
        let result = runner.record_trial(trial).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_results_no_trials() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-empty"))
            .await
            .unwrap();
        let result = runner.get_results("exp-empty").await;
        assert!(result.is_ok());
        let res = result.unwrap().unwrap();
        assert_eq!(res.control_stats.sample_count, 0);
        assert_eq!(res.treatment_stats.sample_count, 0);
        assert!(res.winner.is_none());
    }

    #[tokio::test]
    async fn test_get_results_with_trials() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-results"))
            .await
            .unwrap();
        runner.start_experiment("exp-results").await.unwrap();
        for _ in 0..15 {
            runner
                .record_trial(trial_result(
                    "exp-results",
                    ExperimentGroup::Control,
                    true,
                    5,
                    1000,
                    0.8,
                    false,
                ))
                .await
                .unwrap();
        }
        for _ in 0..15 {
            runner
                .record_trial(trial_result(
                    "exp-results",
                    ExperimentGroup::Treatment,
                    true,
                    3,
                    800,
                    0.95,
                    false,
                ))
                .await
                .unwrap();
        }
        let result = runner.get_results("exp-results").await.unwrap().unwrap();
        assert_eq!(result.control_stats.sample_count, 15);
        assert_eq!(result.treatment_stats.sample_count, 15);
        assert!(!result.metric_comparisons.is_empty());
    }

    #[tokio::test]
    async fn test_get_results_nonexistent_experiment() {
        let runner = ExperimentRunner::new();
        let result = runner.get_results("no-such").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_experiments() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-a"))
            .await
            .unwrap();
        runner
            .create_experiment(test_config("exp-b"))
            .await
            .unwrap();
        let list = runner.list_experiments().await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_compute_group_stats_empty() {
        let stats = ExperimentRunner::compute_group_stats(ExperimentGroup::Control, &[]);
        assert_eq!(stats.sample_count, 0);
        assert_eq!(stats.task_completion_rate, 0.0);
    }

    #[tokio::test]
    async fn test_compute_group_stats_with_data() {
        let trials: Vec<TrialResult> = vec![
            trial_result("x", ExperimentGroup::Control, true, 4, 1000, 0.8, false),
            trial_result("x", ExperimentGroup::Control, false, 6, 2000, 0.5, true),
        ];
        let refs: Vec<&TrialResult> = trials.iter().collect();
        let stats = ExperimentRunner::compute_group_stats(ExperimentGroup::Control, &refs);
        assert_eq!(stats.sample_count, 2);
        assert!((stats.task_completion_rate - 0.5).abs() < f32::EPSILON);
        assert!((stats.error_rate - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_is_statistically_significant_insufficient_samples() {
        assert!(!ExperimentRunner::is_statistically_significant(
            &ExperimentMetric::TaskCompletionRate,
            0.5,
            0.7,
            5,
            5,
        ));
    }

    #[tokio::test]
    async fn test_determine_winner_no_comparisons() {
        assert_eq!(ExperimentRunner::determine_winner(&[]), None);
    }

    #[tokio::test]
    async fn test_determine_winner_treatment() {
        let comparisons = vec![MetricComparison {
            metric: ExperimentMetric::TaskCompletionRate,
            control_value: 0.5,
            treatment_value: 0.8,
            improvement: 0.6,
            is_significant: true,
        }];
        assert_eq!(
            ExperimentRunner::determine_winner(&comparisons),
            Some(ExperimentGroup::Treatment)
        );
    }

    #[tokio::test]
    async fn test_determine_winner_control() {
        let comparisons = vec![MetricComparison {
            metric: ExperimentMetric::TaskCompletionRate,
            control_value: 0.8,
            treatment_value: 0.5,
            improvement: -0.375,
            is_significant: true,
        }];
        assert_eq!(
            ExperimentRunner::determine_winner(&comparisons),
            Some(ExperimentGroup::Control)
        );
    }

    #[tokio::test]
    async fn test_compute_confidence_level_empty() {
        assert!((ExperimentRunner::compute_confidence_level(&[]) - 0.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_compute_confidence_level_mixed() {
        let comparisons = vec![
            MetricComparison {
                metric: ExperimentMetric::TaskCompletionRate,
                control_value: 0.5,
                treatment_value: 0.8,
                improvement: 0.6,
                is_significant: true,
            },
            MetricComparison {
                metric: ExperimentMetric::ErrorRate,
                control_value: 0.1,
                treatment_value: 0.1,
                improvement: 0.0,
                is_significant: false,
            },
        ];
        let confidence = ExperimentRunner::compute_confidence_level(&comparisons);
        assert!((confidence - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_nonexistent_experiment_operations() {
        let runner = ExperimentRunner::new();
        assert!(runner.start_experiment("nope").await.is_err());
        assert!(runner.pause_experiment("nope").await.is_err());
        assert!(runner.cancel_experiment("nope").await.is_err());
        assert!(runner.get_experiment("nope").await.is_none());
    }

    #[tokio::test]
    async fn test_experiment_status_transitions() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-flow"))
            .await
            .unwrap();
        assert_eq!(
            runner.get_experiment("exp-flow").await.unwrap().status,
            ExperimentStatus::Draft
        );
        runner.start_experiment("exp-flow").await.unwrap();
        assert_eq!(
            runner.get_experiment("exp-flow").await.unwrap().status,
            ExperimentStatus::Running
        );
        runner.pause_experiment("exp-flow").await.unwrap();
        assert_eq!(
            runner.get_experiment("exp-flow").await.unwrap().status,
            ExperimentStatus::Paused
        );
        runner.start_experiment("exp-flow").await.unwrap();
        assert_eq!(
            runner.get_experiment("exp-flow").await.unwrap().status,
            ExperimentStatus::Running
        );
        runner.cancel_experiment("exp-flow").await.unwrap();
        assert_eq!(
            runner.get_experiment("exp-flow").await.unwrap().status,
            ExperimentStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn test_custom_metrics_in_trial() {
        let runner = ExperimentRunner::new();
        runner
            .create_experiment(test_config("exp-custom"))
            .await
            .unwrap();
        runner.start_experiment("exp-custom").await.unwrap();
        let mut custom = HashMap::new();
        custom.insert("latency_p99".to_string(), 250.0_f32);
        let trial = TrialResult {
            experiment_id: "exp-custom".to_string(),
            group: ExperimentGroup::Control,
            task_completed: true,
            iterations: 3,
            duration_ms: 500,
            tools_used: 4,
            unique_tools: 2,
            quality_score: 0.85,
            had_errors: false,
            custom_metrics: custom,
        };
        assert!(runner.record_trial(trial).await.is_ok());
    }
}
