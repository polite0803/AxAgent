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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingMetrics {
    pub train_loss: Vec<f32>,
    pub val_loss: Vec<f32>,
    pub learning_rates: Vec<f32>,
    pub final_loss: Option<f32>,
    pub best_loss: Option<f32>,
}

impl Default for TrainingMetrics {
    fn default() -> Self {
        Self {
            train_loss: Vec::new(),
            val_loss: Vec::new(),
            learning_rates: Vec::new(),
            final_loss: None,
            best_loss: None,
        }
    }
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
        matches!(
            self.status,
            JobStatus::Training | JobStatus::Preparing | JobStatus::Validating
        )
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
