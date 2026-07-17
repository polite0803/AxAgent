// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_credential::{Credential, CredentialType};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{State, command};

const CRED_ID_FLUX: &str = "image_gen_flux_token";
const CRED_ID_OPENAI: &str = "image_gen_openai_key";
const CRED_NAME_FLUX: &str = "Flux (Replicate) API Token";
const CRED_NAME_OPENAI: &str = "OpenAI (DALL-E) API Key";

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
    let app_dir = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".")).join("AxAgent");
    fs::create_dir_all(&app_dir).ok();
    app_dir.join("image_gen_config.json")
}

/// 从 credential 存储加载 API key，若不存在则回退到 JSON 配置（旧版迁移）
async fn load_api_key(
    cred_id: &str,
    cred_mgr: &axagent_credential::CredentialManager,
    json_key: &str,
) -> Option<String> {
    // 优先从加密存储加载
    if let Ok(cred) = cred_mgr.get_credential(cred_id).await {
        match &cred.credential_type {
            CredentialType::BearerToken { token } => return Some(token.clone()),
            CredentialType::ApiKey { key, .. } => return Some(key.clone()),
            _ => {},
        }
    }
    // 回退：从 JSON 配置读取（旧版迁移路径）
    if !json_key.is_empty() {
        Some(json_key.to_string())
    } else {
        None
    }
}

#[command]
pub async fn get_image_gen_config(state: State<'_, AppState>) -> Result<ImageGenConfig, String> {
    let path = get_image_gen_config_path();
    let mut config = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
        serde_json::from_str::<ImageGenConfig>(&content)
            .map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
            .unwrap_or_default()
    } else {
        ImageGenConfig::default()
    };

    // 从 credential 加密存储（或旧 JSON 回退）加载 API key
    if let Some(key) =
        load_api_key(CRED_ID_FLUX, &state.credential_manager, &config.flux_api_token).await
    {
        config.flux_api_token = key;
    }
    if let Some(key) =
        load_api_key(CRED_ID_OPENAI, &state.credential_manager, &config.openai_api_key).await
    {
        config.openai_api_key = key;
    }

    Ok(config)
}

#[command]
pub async fn save_image_gen_config(
    state: State<'_, AppState>,
    config: ImageGenConfig,
) -> Result<(), String> {
    // 1. 保存 API key 到加密 credential 存储
    let flux_token = config.flux_api_token.clone();
    let openai_key = config.openai_api_key.clone();

    if !flux_token.is_empty() {
        let cred = Credential::new(
            CRED_ID_FLUX.to_string(),
            CRED_NAME_FLUX.to_string(),
            CredentialType::BearerToken { token: flux_token },
        );
        state.credential_manager.save_credential(&cred).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    if !openai_key.is_empty() {
        let cred = Credential::new(
            CRED_ID_OPENAI.to_string(),
            CRED_NAME_OPENAI.to_string(),
            CredentialType::BearerToken { token: openai_key },
        );
        state.credential_manager.save_credential(&cred).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;
    }

    // 2. 保存非敏感字段到 JSON 配置（清空 key 字段，已转存到 credential 系统）
    let json_config =
        ImageGenConfig { flux_api_token: String::new(), openai_api_key: String::new(), ..config };

    let path = get_image_gen_config_path();
    let content = serde_json::to_string_pretty(&json_config).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;
    fs::write(&path, content).map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
