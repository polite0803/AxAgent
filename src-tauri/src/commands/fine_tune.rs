// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;
use axagent_agent::fine_tune::candle_trainer::train_with_embeddings;
use axagent_agent::fine_tune::dataset::{
    DatasetMetadata, FineTuneDataset, FineTuneSample, SampleMetadata,
};
use axagent_agent::fine_tune::lora::{LoRAAdapterInfo, LoRAConfigBuilder};
use axagent_agent::fine_tune::trainer::TrainingStats;
use axagent_agent::fine_tune::{
    ActiveModelConfig, BaseModelInfo, FineTuneTrainer, ModelManager, TrainingJob,
};
use axagent_harness::types::{EmbedRequest, ModelType};
use axagent_harness::{ProviderRequestContext, resolve_base_url_for_type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
#[serde(rename_all = "camelCase")]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub num_samples: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    /// 取消训练标志：job_id → cancel flag
    job_cancel_flags: HashMap<String, Arc<AtomicBool>>,
}

impl Default for FineTuneState {
    fn default() -> Self {
        Self {
            datasets: HashMap::new(),
            samples: HashMap::new(),
            trainer: FineTuneTrainer::new(),
            model_manager: ModelManager::new(),
            job_cancel_flags: HashMap::new(),
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

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "列出所有微调数据集")]
#[command]
pub fn list_datasets() -> Result<Vec<DatasetInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.datasets.values().cloned().collect())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateInput, description = "获取指定数据集")]
#[command]
pub fn get_dataset(dataset_id: String) -> Result<DatasetInfo, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.datasets.get(&dataset_id).cloned().ok_or_else(|| "Dataset not found".to_string())
}

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "创建微调数据集")]
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

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "向数据集添加样本")]
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

#[agent_command(domain = "fine_tune", safety = Dangerous, call_mode = StateInput, description = "删除微调数据集")]
#[command]
pub fn delete_dataset(dataset_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.datasets.remove(&dataset_id);
    s.samples.remove(&dataset_id);
    let _ = persist_datasets(&s); // Best-effort persist
    Ok(())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "列出所有训练任务")]
#[command]
pub fn list_training_jobs() -> Result<Vec<TrainingJobInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.trainer.list_jobs().iter().map(|j| TrainingJobInfo::from(*j)).collect())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateInput, description = "获取指定训练任务")]
#[command]
pub fn get_training_job(job_id: String) -> Result<TrainingJobInfo, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.trainer
        .get_job(&job_id)
        .map(TrainingJobInfo::from)
        .ok_or_else(|| "Training job not found".to_string())
}

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "创建微调训练任务")]
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

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "启动微调训练任务")]
#[command]
pub async fn start_training_job(
    app_state: tauri::State<'_, crate::AppState>,
    job_id: String,
) -> Result<(), String> {
    // 检查 lora_finetune_enabled 门控
    let db = app_state.harness.db();
    let settings = axagent_dao::repo::settings::get_settings(db)
        .await
        .map_err(|e| format!("Failed to read settings: {e}"))?;
    if !settings.lora_finetune_enabled {
        return Err(
            "LoRA fine-tuning is disabled. Enable 'lora_finetune_enabled' in settings to use this feature."
                .to_string(),
        );
    }

    // 提取训练所需数据后立即释放锁
    let (config, ds_id, samples, num_samples, cancel_flag) = {
        let mut guard = state().lock().map_err(|e| format!("Lock error: {e}"))?;
        let job =
            guard.trainer.get_job(&job_id).ok_or_else(|| format!("Job '{job_id}' not found"))?;
        let config = job.config.clone();
        let ds_id = job.dataset_id.clone();
        let dataset =
            guard.datasets.get(&ds_id).ok_or_else(|| format!("Dataset '{ds_id}' not found"))?;
        let samples = guard.samples.get(&ds_id).cloned().unwrap_or_default();
        let num_samples = samples.len();
        let _ds_name = dataset.name.clone();

        // 创建取消标志
        let cancel_flag = Arc::new(AtomicBool::new(false));
        guard.job_cancel_flags.insert(job_id.clone(), cancel_flag.clone());

        // 标记为 Preparing
        if let Some(j) = guard.trainer.get_job_mut(&job_id) {
            j.status = axagent_agent::fine_tune::lora::JobStatus::Preparing;
        }

        (config, ds_id.clone(), samples, num_samples, cancel_flag)
    };

    if samples.is_empty() {
        let mut guard = state().lock().map_err(|e| format!("Lock error: {e}"))?;
        let _ = guard.trainer.fail_job(&job_id);
        guard.job_cancel_flags.remove(&job_id);
        return Err("Dataset has no samples".to_string());
    }

    // 构建 FineTuneDataset
    let ft_dataset = FineTuneDataset {
        id: ds_id.clone(),
        name: String::new(),
        description: String::new(),
        samples: samples
            .iter()
            .map(|smp| FineTuneSample {
                id: uuid::Uuid::new_v4().to_string(),
                input: smp.input.clone(),
                output: smp.output.clone(),
                system_prompt: smp.system_prompt.clone(),
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

    let output_dir = FINE_TUNE_DIR.join("adapters");
    let _dataset_id = ds_id.clone();

    // 尝试获取真实 embedding
    let embed_result = try_compute_embeddings_internal(db, &app_state, &samples).await;

    // 准备后台训练
    let jid = job_id.clone();
    let jid2 = job_id.clone();

    tokio::task::spawn(async move {
        // 进度回调：更新 trainer 中的 job progress
        let progress_cb = move |progress: f32, loss: f64| {
            if cancel_flag.load(Ordering::SeqCst) {
                return;
            }
            if let Ok(mut guard) = state().lock() {
                let _ =
                    guard.trainer.update_progress(&jid, (progress * 10.0) as u32, 0, loss as f32);
            }
        };

        let result = match embed_result {
            Ok(Some((input_emb, target_emb, dim))) => {
                tracing::info!(
                    "[fine_tune] Background training with real embeddings (dim={})",
                    dim
                );
                train_with_embeddings(
                    input_emb,
                    target_emb,
                    &config,
                    &output_dir,
                    dim,
                    Some(progress_cb),
                )
            },
            _ => {
                tracing::info!("[fine_tune] Background char-level training");
                axagent_agent::fine_tune::candle_trainer::train_lora(
                    &ft_dataset,
                    &config,
                    &output_dir,
                    None::<fn(f32, f64)>,
                )
            },
        };

        match result {
            Ok(path) => {
                tracing::info!("[fine_tune] Background training done: {}", path.display());
                if let Ok(mut guard) = state().lock() {
                    let _ = guard.trainer.complete_job(&jid2, path.to_string_lossy().to_string());
                    guard.job_cancel_flags.remove(&jid2);
                }
            },
            Err(e) => {
                tracing::warn!("[fine_tune] Background training failed: {e}");
                if let Ok(mut guard) = state().lock() {
                    let _ = guard.trainer.fail_job(&jid2);
                    guard.job_cancel_flags.remove(&jid2);
                }
            },
        }
    });

    Ok(())
}

/// 尝试从 provider 计算真实 embedding。
async fn try_compute_embeddings_internal(
    db: &sea_orm::DatabaseConnection,
    app_state: &crate::AppState,
    samples: &[Sample],
) -> Result<Option<(Vec<Vec<f32>>, Vec<Vec<f32>>, usize)>, String> {
    let all_providers = axagent_dao::repo::provider::list_providers_merged(db)
        .await
        .map_err(|e| format!("list_providers: {e}"))?;

    let embedding_provider = all_providers
        .iter()
        .find(|p| p.models.iter().any(|m| m.model_type == ModelType::Embedding));

    let Some(provider) = embedding_provider else {
        return Ok(None);
    };

    let embed_model = provider
        .models
        .iter()
        .find(|m| m.model_type == ModelType::Embedding)
        .ok_or_else(|| "no embedding model".to_string())?;

    let master_key = app_state.harness.master_key();
    let key = axagent_dao::repo::provider::get_active_key(db, &provider.id)
        .await
        .map_err(|e| format!("get_key: {e}"))?;
    let api_key = axagent_crypto::decrypt_key(&key.key_encrypted, master_key)
        .map_err(|e| format!("decrypt_key: {e}"))?;
    let ctx = ProviderRequestContext {
        api_key,
        key_id: key.id.clone(),
        provider_id: provider.id.clone(),
        base_url: Some(resolve_base_url_for_type(&provider.api_host, &provider.provider_type)),
        api_path: provider.api_path.clone(),
        proxy_config: axagent_harness::types::provider_model::resolve_provider_proxy(
            &provider.proxy_config,
            &axagent_dao::repo::settings::get_settings(db).await.unwrap_or_default(),
        ),
        custom_headers: None,
        api_mode: None,
        conversation: None,
        previous_response_id: None,
        store_response: None,
    };

    let registry = app_state.harness.provider_registry();
    let registry_key =
        axagent_harness::types::provider_model::provider_registry_key(&provider.provider_type);
    let adapter = registry
        .get(registry_key)
        .ok_or_else(|| format!("No adapter for provider type '{registry_key}'"))?;

    let input_texts: Vec<String> = samples.iter().map(|s| s.input.clone()).collect();
    let input_request =
        EmbedRequest { model: embed_model.model_id.clone(), input: input_texts, dimensions: None };
    let input_response =
        adapter.embed(&ctx, input_request).await.map_err(|e| format!("embed inputs: {e}"))?;
    let input_dim = input_response.dimensions;

    let target_texts: Vec<String> = samples.iter().map(|s| s.output.clone()).collect();
    let target_request = EmbedRequest {
        model: embed_model.model_id.clone(),
        input: target_texts,
        dimensions: Some(input_dim),
    };
    let target_response =
        adapter.embed(&ctx, target_request).await.map_err(|e| format!("embed targets: {e}"))?;

    Ok(Some((input_response.embeddings, target_response.embeddings, input_dim)))
}

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "取消微调训练任务")]
#[command]
pub fn cancel_training_job(job_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    // 设置取消标志（后台任务会检测并提前退出）
    if let Some(flag) = s.job_cancel_flags.get(&job_id) {
        flag.store(true, Ordering::SeqCst);
        let _ = s.trainer.cancel_training(&job_id);
    }
    Ok(())
}

#[agent_command(domain = "fine_tune", safety = Dangerous, call_mode = StateInput, description = "删除微调训练任务")]
#[command]
pub fn delete_training_job(job_id: String) -> Result<(), String> {
    let mut s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    s.trainer.delete_job(&job_id).map_err(|e| {
        crate::commands::error::ErrorResponse::err_with_detail(
            crate::commands::error_code::fine_tune::DELETE_FAILED,
            format!("{e:?}"),
        )
    })?;
    Ok(())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "获取训练统计信息")]
#[command]
pub fn get_training_stats() -> Result<TrainingStats, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.trainer.get_training_stats())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "列出所有基础模型")]
#[command]
pub fn list_base_models() -> Result<Vec<BaseModelInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.model_manager.get_base_models().iter().cloned().cloned().collect())
}

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "列出所有 LoRA 适配器")]
#[command]
pub fn list_lora_adapters() -> Result<Vec<LoRAAdapterInfo>, String> {
    let s = state().lock().map_err(|e| format!("Lock error: {}", e))?;
    Ok(s.model_manager.get_lora_adapters().iter().cloned().cloned().collect())
}

#[agent_command(domain = "fine_tune", safety = Caution, call_mode = StateInput, description = "设置活跃模型配置")]
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

#[agent_command(domain = "fine_tune", safety = Safe, call_mode = StateOnly, description = "获取当前活跃模型")]
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
