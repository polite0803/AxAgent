use axagent_agent::fine_tune::lora::LoRAAdapterInfo;
use axagent_agent::fine_tune::trainer::TrainingStats;
use axagent_agent::fine_tune::{ActiveModelConfig, BaseModelInfo, TrainingJob};
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub num_samples: usize,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrainingJobInfo {
    pub id: String,
    pub status: String,
    pub dataset_id: String,
    pub base_model: String,
    pub progress_percent: f32,
    pub current_loss: f32,
    pub output_lora: Option<String>,
}

impl From<&TrainingJob> for TrainingJobInfo {
    fn from(job: &TrainingJob) -> Self {
        Self {
            id: job.id.clone(),
            status: format!("{:?}", job.status),
            dataset_id: job.dataset_id.clone(),
            base_model: job.base_model.clone(),
            progress_percent: job.progress.percent_complete(),
            current_loss: job.progress.loss,
            output_lora: job.output_lora.clone(),
        }
    }
}

#[command]
pub fn list_datasets() -> Result<Vec<DatasetInfo>, String> {
    Ok(vec![])
}

#[command]
pub fn get_dataset(dataset_id: String) -> Result<DatasetInfo, String> {
    let _ = dataset_id;
    Err("Dataset not found".to_string())
}

#[command]
pub fn create_dataset(name: String, description: String) -> Result<DatasetInfo, String> {
    Ok(DatasetInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        num_samples: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[command]
pub fn add_sample(
    dataset_id: String,
    input: String,
    output: String,
    system_prompt: Option<String>,
) -> Result<(), String> {
    let _ = (dataset_id, input, output, system_prompt);
    Ok(())
}

#[command]
pub fn delete_dataset(dataset_id: String) -> Result<(), String> {
    let _ = dataset_id;
    Ok(())
}

#[command]
pub fn list_training_jobs() -> Result<Vec<TrainingJobInfo>, String> {
    Ok(vec![])
}

#[command]
pub fn get_training_job(job_id: String) -> Result<TrainingJobInfo, String> {
    let _ = job_id;
    Err("Training job not found".to_string())
}

#[command]
pub fn create_training_job(
    dataset_id: String,
    base_model: String,
    _rank: u32,
    _alpha: u32,
    _learning_rate: f32,
    _batch_size: u32,
    _epochs: u32,
) -> Result<TrainingJobInfo, String> {
    Ok(TrainingJobInfo {
        id: uuid::Uuid::new_v4().to_string(),
        status: "Pending".to_string(),
        dataset_id,
        base_model,
        progress_percent: 0.0,
        current_loss: 0.0,
        output_lora: None,
    })
}

#[command]
pub fn start_training_job(job_id: String) -> Result<(), String> {
    let _ = job_id;
    Ok(())
}

#[command]
pub fn cancel_training_job(job_id: String) -> Result<(), String> {
    let _ = job_id;
    Ok(())
}

#[command]
pub fn delete_training_job(job_id: String) -> Result<(), String> {
    let _ = job_id;
    Ok(())
}

#[command]
pub fn get_training_stats() -> Result<TrainingStats, String> {
    Ok(TrainingStats {
        total_jobs: 0,
        completed_jobs: 0,
        running_jobs: 0,
        failed_jobs: 0,
    })
}

#[command]
pub fn list_base_models() -> Result<Vec<BaseModelInfo>, String> {
    Ok(vec![])
}

#[command]
pub fn list_lora_adapters() -> Result<Vec<LoRAAdapterInfo>, String> {
    Ok(vec![])
}

#[command]
pub fn set_active_model(base_model: String, adapter_ids: Vec<String>) -> Result<(), String> {
    let _ = (base_model, adapter_ids);
    Ok(())
}

#[command]
pub fn get_active_model() -> Result<Option<ActiveModelConfig>, String> {
    Ok(None)
}
