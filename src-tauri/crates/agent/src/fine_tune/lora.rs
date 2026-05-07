use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoRAConfig {
    pub rank: u32,
    pub alpha: u32,
    pub target_modules: Vec<String>,
    pub dropout: f32,
    pub bias: BiasType,
    pub learning_rate: f32,
    pub batch_size: u32,
    pub epochs: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BiasType {
    None,
    All,
    LoraOnly,
}

impl Default for LoRAConfig {
    fn default() -> Self {
        Self {
            rank: 8,
            alpha: 16,
            target_modules: vec![
                "q_proj".to_string(),
                "v_proj".to_string(),
                "k_proj".to_string(),
                "o_proj".to_string(),
            ],
            dropout: 0.05,
            bias: BiasType::None,
            learning_rate: 0.0002,
            batch_size: 4,
            epochs: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub status: JobStatus,
    pub config: LoRAConfig,
    pub dataset_id: String,
    pub base_model: String,
    pub output_lora: Option<String>,
    pub progress: TrainingProgress,
    pub metrics: TrainingMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Pending,
    Preparing,
    Training,
    Validating,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingProgress {
    pub current_epoch: u32,
    pub total_epochs: u32,
    pub current_step: u32,
    pub total_steps: u32,
    pub samples_per_second: f32,
    pub eta_seconds: u64,
    pub loss: f32,
}

impl TrainingProgress {
    pub fn new(total_epochs: u32, total_steps: u32) -> Self {
        Self {
            current_epoch: 0,
            total_epochs,
            current_step: 0,
            total_steps,
            samples_per_second: 0.0,
            eta_seconds: 0,
            loss: 0.0,
        }
    }

    pub fn percent_complete(&self) -> f32 {
        if self.total_steps == 0 {
            return 0.0;
        }
        (self.current_step as f32 / self.total_steps as f32) * 100.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainingMetrics {
    pub train_loss: Vec<f32>,
    pub val_loss: Vec<f32>,
    pub learning_rates: Vec<f32>,
    pub final_loss: Option<f32>,
    pub best_loss: Option<f32>,
}

impl TrainingJob {
    pub fn new(id: String, dataset_id: String, base_model: String, config: LoRAConfig) -> Self {
        let total_steps = config.batch_size * config.epochs;
        Self {
            id,
            status: JobStatus::Pending,
            config,
            dataset_id,
            base_model,
            output_lora: None,
            progress: TrainingProgress::new(3, total_steps),
            metrics: TrainingMetrics::default(),
        }
    }

    pub fn start(&mut self) {
        self.status = JobStatus::Training;
    }

    pub fn complete(&mut self, output_path: String) {
        self.status = JobStatus::Completed;
        self.output_lora = Some(output_path);
    }

    pub fn fail(&mut self) {
        self.status = JobStatus::Failed;
    }

    pub fn cancel(&mut self) {
        self.status = JobStatus::Cancelled;
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::Training | JobStatus::Preparing | JobStatus::Validating)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoRAAdapterInfo {
    pub adapter_id: String,
    pub name: String,
    pub base_model: String,
    pub lora_path: String,
    pub rank: u32,
    pub alpha: u32,
    pub training_date: DateTime<Utc>,
    pub performance_score: f32,
    pub description: String,
}

impl LoRAAdapterInfo {
    pub fn from_training_job(job: &TrainingJob, lora_path: String) -> Self {
        Self {
            adapter_id: uuid::Uuid::new_v4().to_string(),
            name: format!("{}-{}-lora", job.base_model, job.dataset_id),
            base_model: job.base_model.clone(),
            lora_path,
            rank: job.config.rank,
            alpha: job.config.alpha,
            training_date: Utc::now(),
            performance_score: job.metrics.final_loss.unwrap_or(0.0),
            description: format!("LoRA adapter trained on dataset {}", job.dataset_id),
        }
    }
}

pub struct LoRAConfigBuilder {
    rank: u32,
    alpha: u32,
    target_modules: Vec<String>,
    dropout: f32,
    bias: BiasType,
    learning_rate: f32,
    batch_size: u32,
    epochs: u32,
}

impl LoRAConfigBuilder {
    pub fn new() -> Self {
        Self {
            rank: 8,
            alpha: 16,
            target_modules: vec!["q_proj".to_string(), "v_proj".to_string()],
            dropout: 0.05,
            bias: BiasType::None,
            learning_rate: 0.0002,
            batch_size: 4,
            epochs: 3,
        }
    }

    pub fn rank(mut self, rank: u32) -> Self {
        self.rank = rank;
        self.alpha = rank * 2;
        self
    }

    pub fn alpha(mut self, alpha: u32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn target_modules(mut self, modules: Vec<String>) -> Self {
        self.target_modules = modules;
        self
    }

    pub fn dropout(mut self, dropout: f32) -> Self {
        self.dropout = dropout;
        self
    }

    pub fn bias(mut self, bias: BiasType) -> Self {
        self.bias = bias;
        self
    }

    pub fn learning_rate(mut self, lr: f32) -> Self {
        self.learning_rate = lr;
        self
    }

    pub fn batch_size(mut self, size: u32) -> Self {
        self.batch_size = size;
        self
    }

    pub fn epochs(mut self, epochs: u32) -> Self {
        self.epochs = epochs;
        self
    }

    pub fn build(self) -> LoRAConfig {
        LoRAConfig {
            rank: self.rank,
            alpha: self.alpha,
            target_modules: self.target_modules,
            dropout: self.dropout,
            bias: self.bias,
            learning_rate: self.learning_rate,
            batch_size: self.batch_size,
            epochs: self.epochs,
        }
    }
}

impl Default for LoRAConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lora_config_default() {
        let config = LoRAConfig::default();
        assert_eq!(config.rank, 8);
        assert_eq!(config.alpha, 16);
        assert_eq!(config.dropout, 0.05);
        assert_eq!(config.bias, BiasType::None);
        assert!((config.learning_rate - 0.0002).abs() < 0.0001);
        assert_eq!(config.batch_size, 4);
        assert_eq!(config.epochs, 3);
    }

    #[test]
    fn test_lora_config_builder_default() {
        let builder = LoRAConfigBuilder::default();
        let config = builder.build();
        assert_eq!(config.rank, 8);
        assert_eq!(config.alpha, 16);
    }

    #[test]
    fn test_lora_config_builder_custom_rank() {
        let config = LoRAConfigBuilder::new().rank(16).build();
        assert_eq!(config.rank, 16);
        assert_eq!(config.alpha, 32);
    }

    #[test]
    fn test_lora_config_builder_custom_alpha() {
        let config = LoRAConfigBuilder::new().alpha(64).build();
        assert_eq!(config.alpha, 64);
    }

    #[test]
    fn test_lora_config_builder_full() {
        let config = LoRAConfigBuilder::new()
            .rank(32)
            .alpha(64)
            .target_modules(vec!["q_proj".to_string()])
            .dropout(0.1)
            .bias(BiasType::All)
            .learning_rate(0.001)
            .batch_size(8)
            .epochs(5)
            .build();
        assert_eq!(config.rank, 32);
        assert_eq!(config.alpha, 64);
        assert_eq!(config.target_modules.len(), 1);
        assert!((config.dropout - 0.1).abs() < 0.001);
        assert_eq!(config.bias, BiasType::All);
        assert!((config.learning_rate - 0.001).abs() < 0.0001);
        assert_eq!(config.batch_size, 8);
        assert_eq!(config.epochs, 5);
    }

    #[test]
    fn test_training_progress_new() {
        let progress = TrainingProgress::new(3, 100);
        assert_eq!(progress.current_epoch, 0);
        assert_eq!(progress.total_epochs, 3);
        assert_eq!(progress.current_step, 0);
        assert_eq!(progress.total_steps, 100);
        assert!((progress.loss - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_training_progress_percent_complete_zero() {
        let progress = TrainingProgress::new(3, 0);
        assert!((progress.percent_complete() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_training_progress_percent_complete_half() {
        let mut progress = TrainingProgress::new(3, 100);
        progress.current_step = 50;
        assert!((progress.percent_complete() - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_training_progress_percent_complete_full() {
        let mut progress = TrainingProgress::new(3, 100);
        progress.current_step = 100;
        assert!((progress.percent_complete() - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_training_job_new() {
        let config = LoRAConfig::default();
        let job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        assert_eq!(job.id, "job1");
        assert_eq!(job.status, JobStatus::Pending);
        assert_eq!(job.dataset_id, "ds1");
        assert_eq!(job.base_model, "model1");
        assert!(job.output_lora.is_none());
    }

    #[test]
    fn test_training_job_start() {
        let config = LoRAConfig::default();
        let mut job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        job.start();
        assert_eq!(job.status, JobStatus::Training);
    }

    #[test]
    fn test_training_job_complete() {
        let config = LoRAConfig::default();
        let mut job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        job.complete("/output/lora".to_string());
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.output_lora, Some("/output/lora".to_string()));
    }

    #[test]
    fn test_training_job_fail() {
        let config = LoRAConfig::default();
        let mut job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        job.fail();
        assert_eq!(job.status, JobStatus::Failed);
    }

    #[test]
    fn test_training_job_cancel() {
        let config = LoRAConfig::default();
        let mut job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        job.cancel();
        assert_eq!(job.status, JobStatus::Cancelled);
    }

    #[test]
    fn test_training_job_is_running() {
        let config = LoRAConfig::default();
        let mut job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        assert!(!job.is_running());
        job.start();
        assert!(job.is_running());
        job.complete("/out".to_string());
        assert!(!job.is_running());
    }

    #[test]
    fn test_training_metrics_default() {
        let metrics = TrainingMetrics::default();
        assert!(metrics.train_loss.is_empty());
        assert!(metrics.val_loss.is_empty());
        assert!(metrics.final_loss.is_none());
        assert!(metrics.best_loss.is_none());
    }

    #[test]
    fn test_lora_adapter_info_from_training_job() {
        let config = LoRAConfig::default();
        let job =
            TrainingJob::new("job1".to_string(), "ds1".to_string(), "model1".to_string(), config);
        let info = LoRAAdapterInfo::from_training_job(&job, "/path/lora".to_string());
        assert_eq!(info.base_model, "model1");
        assert_eq!(info.lora_path, "/path/lora");
        assert_eq!(info.rank, 8);
        assert_eq!(info.alpha, 16);
    }

    #[test]
    fn test_bias_type_variants() {
        let biases = vec![BiasType::None, BiasType::All, BiasType::LoraOnly];
        for bias in biases {
            let json = serde_json::to_string(&bias).unwrap();
            let de: BiasType = serde_json::from_str(&json).unwrap();
            assert_eq!(de, bias);
        }
    }

    #[test]
    fn test_job_status_variants() {
        let statuses = vec![
            JobStatus::Pending,
            JobStatus::Preparing,
            JobStatus::Training,
            JobStatus::Validating,
            JobStatus::Completed,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ];
        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let de: JobStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(de, status);
        }
    }
}
