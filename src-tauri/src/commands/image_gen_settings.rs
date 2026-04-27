use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenConfig {
    pub default_provider: String,
    pub flux_api_token: String,
    pub openai_api_key: String,
    pub openai_base_url: String,
    pub default_width: u32,
    pub default_height: u32,
    pub default_steps: u32,
    pub save_to_artifact: bool,
}

impl Default for ImageGenConfig {
    fn default() -> Self {
        Self {
            default_provider: "flux".to_string(),
            flux_api_token: String::new(),
            openai_api_key: String::new(),
            openai_base_url: "https://api.openai.com/v1".to_string(),
            default_width: 1024,
            default_height: 1024,
            default_steps: 4,
            save_to_artifact: true,
        }
    }
}

fn get_image_gen_config_path() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("AxAgent");
    fs::create_dir_all(&app_dir).ok();
    app_dir.join("image_gen_config.json")
}

#[command]
pub fn get_image_gen_config() -> Result<ImageGenConfig, String> {
    let path = get_image_gen_config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(ImageGenConfig::default())
    }
}

#[command]
pub fn save_image_gen_config(config: ImageGenConfig) -> Result<(), String> {
    let path = get_image_gen_config_path();
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}
