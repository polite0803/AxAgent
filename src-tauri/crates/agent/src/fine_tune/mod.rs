pub mod dataset;
pub mod lora;
pub mod trainer;

pub use dataset::{DataFormat, DatasetMetadata, FineTuneDataset, FineTuneSample};
pub use lora::{JobStatus, LoRAAdapterInfo, LoRAConfig, TrainingJob};
pub use trainer::FineTuneTrainer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManager {
    pub base_models: HashMap<String, BaseModelInfo>,
    pub lora_adapters: HashMap<String, LoRAAdapterInfo>,
    pub active_config: ActiveModelConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModelInfo {
    pub model_id: String,
    pub name: String,
    pub path: String,
    pub size_gb: f32,
    pub context_length: u32,
    pub supports_lora: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveModelConfig {
    pub base_model: String,
    pub lora_adapters: Vec<String>,
    pub system_prompt: Option<String>,
    pub generation_params: GenerationParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationParams {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
    pub repeat_penalty: f32,
}

impl Default for GenerationParams {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            max_tokens: 4096,
            repeat_penalty: 1.1,
        }
    }
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            base_models: HashMap::new(),
            lora_adapters: HashMap::new(),
            active_config: ActiveModelConfig {
                base_model: String::new(),
                lora_adapters: Vec::new(),
                system_prompt: None,
                generation_params: GenerationParams::default(),
            },
        }
    }

    pub fn register_base_model(&mut self, model: BaseModelInfo) {
        self.base_models.insert(model.model_id.clone(), model);
    }

    pub fn register_lora_adapter(&mut self, adapter: LoRAAdapterInfo) {
        self.lora_adapters
            .insert(adapter.adapter_id.clone(), adapter);
    }

    pub fn set_active_config(&mut self, config: ActiveModelConfig) {
        self.active_config = config;
    }

    pub fn get_base_models(&self) -> Vec<&BaseModelInfo> {
        self.base_models.values().collect()
    }

    pub fn get_lora_adapters(&self) -> Vec<&LoRAAdapterInfo> {
        self.lora_adapters.values().collect()
    }

    pub fn get_adapters_for_base_model(&self, base_model: &str) -> Vec<&LoRAAdapterInfo> {
        self.lora_adapters
            .values()
            .filter(|a| a.base_model == base_model)
            .collect()
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}
