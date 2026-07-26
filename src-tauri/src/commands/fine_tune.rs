// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent::fine_tune::dataset::{
    DatasetMetadata, FineTuneDataset, FineTuneSample, SampleMetadata,
};
use axagent_agent::fine_tune::lora::{LoRAAdapterInfo, LoRAConfigBuilder};
use axagent_agent::fine_tune::trainer::TrainingStats;
use axagent_agent::fine_tune::{
    ActiveModelConfig, BaseModelInfo, FineTuneTrainer, ModelManager, TrainingJob,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::command;
use tracing::warn;

static FINE_TUNE_DIR: std::sync::LazyLock<PathBuf> = std::sync::LazyLock::new(|| {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".axagent").join("fine_tune")
});

fn datasets_file() -> PathBuf {
    FINE_TUNE_DIR.join("datasets.json")
}

fn samples_file() -> PathBuf {
    FINE_TUNE_DIR.join("samples.json")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub num_samples: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJobInfo {
    pub id: String,
    pub status: String,
    pub dataset_id: String,
    pub base_model: String,
    pub progress_percent: f32,
    pub current_loss: f32,
    pub output_lora: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sample {
    input: String,
    output: String,
    system_prompt: Option<String>,
}

struct FineTuneState {
    datasets: HashMap<String, DatasetInfo>,
    samples: HashMap<String, Vec<Sample>>,
    trainer: FineTuneTrainer,
    model_manager: ModelManager,
}

impl Default for FineTuneState {
    fn default() -> Self {
        Self {
            datasets: HashMap::new(),
            samples: HashMap::new(),
            trainer: FineTuneTrainer::new(),
            model_manager: ModelManager::new(),
        }
    }
}

static FINE_TUNE_STATE: std::sync::OnceLock<Mutex<FineTuneState>> = std::sync::OnceLock::new();

fn ensure_dir() -> Result<(), String> {
    fs::create_dir_all(&*FINE_TUNE_DIR)
        .map_err(|e| format!("Failed to create fine_tune directory: {}", e))
}

fn state() -> &'static Mutex<FineTuneState> {
    FINE_TUNE_STATE.get_or_init(|| {
        let mut s = FineTuneState::default();
        // Load persisted datasets and samples from disk
        if let Err(e) = load_datasets(&mut s) {
            warn!("[fine_tune] Failed to load datasets from disk: {}", e);
        }
        Mutex::new(s)
    })
}

fn persist_datasets(state: &FineTuneState) -> Result<(), String> {
    ensure_dir()?;
    let json = serde_json::to_string_pretty(&state.datasets)
        .map_err(|e| format!("Serialize datasets: {}", e))?;
    fs::write(datasets_file(), json).map_err(|e| format!("Write datasets: {}", e))?;
    let samples_json = serde_json::to_string_pretty(&state.samples)
        .map_err(|e| format!("Serialize samples: {}", e))?;
    fs::write(samples_file(), samples_json).map_err(|e| format!("Write samples: {}", e))?;
    Ok(())
}

fn load_datasets(state: &mut FineTuneState) -> Result<(), String> {
    let path = datasets_file();
    if path.exists() {
        let json = fs::read_to_string(&path).map_err(|e| format!("Read datasets: {}", e))?;
        state.datasets = serde_json::from_str(&json).unwrap_or_default();
    }
    let samples_path = samples_file();
    if samples_path.exists() {
        let json = fs::read_to_string(&samples_path).map_err(|e| format!("Read samples: {}", e))?;
        state.samples = serde_json::from_str(&json).unwrap_or_default();
    }
    Ok(())
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
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.datasets.values().cloned().collect())
}

#[command]
pub fn get_dataset(dataset_id: String) -> Result<DatasetInfo, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.datasets.get(&dataset_id).cloned().ok_or_else(|| "Dataset not found".to_string())
}

#[command]
pub fn create_dataset(name: String, description: String) -> Result<DatasetInfo, String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    let dataset = DatasetInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        num_samples: 0,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    s.datasets.insert(dataset.id.clone(), dataset.clone());
    s.samples.insert(dataset.id.clone(), Vec::new());
    let _ = persist_datasets(&s); // Best-effort persist
    Ok(dataset)
}

#[command]
pub fn add_sample(
    dataset_id: String,
    input: String,
    output: String,
    system_prompt: Option<String>,
) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    let samples = s.samples.get_mut(&dataset_id).ok_or_else(|| "Dataset not found".to_string())?;
    samples.push(Sample { input, output, system_prompt });
    let new_count = samples.len();
    if let Some(ds) = s.datasets.get_mut(&dataset_id) {
        ds.num_samples = new_count;
    }
    let _ = persist_datasets(&s); // Best-effort persist
    Ok(())
}

#[command]
pub fn delete_dataset(dataset_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.datasets.remove(&dataset_id);
    s.samples.remove(&dataset_id);
    let _ = persist_datasets(&s); // Best-effort persist
    Ok(())
}

#[command]
pub fn list_training_jobs() -> Result<Vec<TrainingJobInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.trainer.list_jobs().iter().map(|j| TrainingJobInfo::from(*j)).collect())
}

#[command]
pub fn get_training_job(job_id: String) -> Result<TrainingJobInfo, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.trainer
        .get_job(&job_id)
        .map(TrainingJobInfo::from)
        .ok_or_else(|| "Training job not found".to_string())
}

#[command]
pub fn create_training_job(
    dataset_id: String,
    base_model: String,
    rank: u32,
    alpha: u32,
    learning_rate: f32,
    batch_size: u32,
    epochs: u32,
) -> Result<TrainingJobInfo, String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;

    let config = LoRAConfigBuilder::new()
        .rank(rank)
        .alpha(alpha)
        .learning_rate(learning_rate)
        .batch_size(batch_size)
        .epochs(epochs)
        .build();

    let job = s.trainer.create_job(dataset_id, base_model, config);
    Ok(TrainingJobInfo::from(&job))
}

#[command]
pub async fn start_training_job(
    app_state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<(), String> {
    // 检查 lora_finetune_enabled 门控
    let settings = axagent_dao::repo::settings::get_settings(app_state.harness.db())
        .await
        .map_err(|e| format!("Failed to read settings: {e}"))?;
    if !settings.lora_finetune_enabled {
        return Err("LoRA fine-tuning is disabled. Enable 'lora_finetune_enabled' in settings to use this feature.".to_string());
    }

    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;

    // 获取训练任务配置
    let config;
    let ds_id;
    {
        let job =
            s.trainer.get_job(&job_id).ok_or_else(|| format!("Job '{}' not found", job_id))?;
        config = job.config.clone();
        ds_id = job.dataset_id.clone();
    }

    // 获取数据集
    let dataset = s.datasets.get(&ds_id).ok_or_else(|| format!("Dataset '{}' not found", ds_id))?;
    let samples = s.samples.get(&ds_id).cloned().unwrap_or_default();
    let num_samples = samples.len();
    let dataset_id = dataset.id.clone();
    let dataset_name = dataset.name.clone();
    drop(s); // 释放锁（训练可能耗时）

    // 构建 FineTuneDataset（使用显式字段，SampleMetadata/DatasetMetadata 无 Default）
    let ft_dataset = FineTuneDataset {
        id: dataset_id.clone(),
        name: dataset_name,
        description: String::new(),
        samples: samples
            .into_iter()
            .map(|s| FineTuneSample {
                id: uuid::Uuid::new_v4().to_string(),
                input: s.input,
                output: s.output,
                system_prompt: s.system_prompt,
                metadata: SampleMetadata {
                    source: "manual".to_string(),
                    category: None,
                    difficulty: None,
                    tags: vec![],
                },
            })
            .collect(),
        format: axagent_agent::fine_tune::dataset::DataFormat::Jsonl,
        metadata: DatasetMetadata {
            source: "manual".to_string(),
            license: "custom".to_string(),
            tags: vec![],
            num_samples,
            created_at: chrono::Utc::now(),
        },
    };

    // 使用真实的 candle-based LoRA 训练
    let output_dir = FINE_TUNE_DIR.join("adapters");
    match axagent_agent::fine_tune::candle_trainer::train_lora(
        &ft_dataset,
        &config,
        &output_dir,
        None::<fn(f32, f64)>,
    ) {
        Ok(safetensors_path) => {
            tracing::info!("[fine_tune] Training completed: {}", safetensors_path.display());
            let mut s = state().lock().map_err(|e| format!("Lock error: {e}"))?;
            s.trainer
                .complete_job(&job_id, safetensors_path.to_string_lossy().to_string())
                .map_err(|e| format!("complete_job: {e:?}"))?;
            Ok(())
        },
        Err(e) => {
            let mut s = state().lock().map_err(|e| format!("Lock error: {e}"))?;
            let _ = s.trainer.fail_job(&job_id);
            Err(format!("Training failed: {e}"))
        },
    }
}

#[command]
pub fn cancel_training_job(job_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.trainer.cancel_training(&job_id).map_err(|e| format!("Cancel failed: {:?}", e))?;
    Ok(())
}

#[command]
pub fn delete_training_job(job_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.trainer.delete_job(&job_id).map_err(|e| format!("Delete failed: {:?}", e))?;
    Ok(())
}

#[command]
pub fn get_training_stats() -> Result<TrainingStats, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.trainer.get_training_stats())
}

#[command]
pub fn list_base_models() -> Result<Vec<BaseModelInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.model_manager.get_base_models().iter().cloned().cloned().collect())
}

#[command]
pub fn list_lora_adapters() -> Result<Vec<LoRAAdapterInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.model_manager.get_lora_adapters().iter().cloned().cloned().collect())
}

#[command]
pub fn set_active_model(base_model: String, adapter_ids: Vec<String>) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.model_manager.set_active_config(ActiveModelConfig {
        base_model,
        lora_adapters: adapter_ids,
        system_prompt: None,
        generation_params: Default::default(),
    });
    Ok(())
}

#[command]
pub fn get_active_model() -> Result<Option<ActiveModelConfig>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    let config = s.model_manager.active_config.clone();
    if config.base_model.is_empty() {
        Ok(None)
    } else {
        Ok(Some(config))
    }
}
