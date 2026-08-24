// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use axagent_agent_macro::agent_command;
use axagent_credential::CredentialType;
use axagent_providers::image_gen::{
    DallEProvider, FluxProvider, ImageGenProvider, ImageGenRequest, ImageGenResponse,
};
use tauri::{State, command};

const CRED_ID_FLUX: &str = "image_gen_flux_token";
const CRED_ID_OPENAI: &str = "image_gen_openai_key";

/// 从 credential 系统读取 API key 回退
async fn resolve_api_key(
    provider: &str,
    api_key: Option<String>,
    cred_mgr: &axagent_credential::CredentialManager,
) -> Result<String, String> {
    if let Some(key) = api_key {
        if !key.is_empty() {
            return Ok(key);
        }
    }
    // 从加密存储读取
    let cred_id = match provider {
        "flux" | "Flux" => CRED_ID_FLUX,
        "dall-e" | "dalle" | "DALL-E" => CRED_ID_OPENAI,
        _ => return Err(format!("Unknown provider: {provider}")),
    };
    let cred = cred_mgr.get_credential(cred_id).await.map_err(|_| {
        format!(
            "API key not configured for {provider}. Please set it in Settings → Image Generation."
        )
    })?;
    match &cred.credential_type {
        CredentialType::BearerToken { token } => Ok(token.clone()),
        CredentialType::ApiKey { key, .. } => Ok(key.clone()),
        _ => Err(format!("Invalid credential type for {provider}")),
    }
}

#[allow(clippy::too_many_arguments)]
#[agent_command(domain = image_gen, safety = Safe, call_mode = StateInput, description = "生成图像")]
#[command]
pub async fn generate_image(
    state: State<'_, AppState>,
    prompt: String,
    negative_prompt: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    steps: Option<u32>,
    seed: Option<u64>,
    model: Option<String>,
    provider: Option<String>,
    api_key: Option<String>,
    quality: Option<String>,
) -> Result<ImageGenResponse, String> {
    let provider_name = provider.as_deref().unwrap_or("flux");

    let request = ImageGenRequest {
        prompt,
        negative_prompt,
        width,
        height,
        steps,
        seed,
        model,
        n: Some(1),
        quality,
    };

    match provider_name {
        "flux" | "Flux" => {
            let api_token =
                resolve_api_key(provider_name, api_key, &state.credential_manager).await?;
            let flux_provider = FluxProvider::new(api_token);
            flux_provider.generate(request).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        "dall-e" | "dalle" | "DALL-E" => {
            let api_key =
                resolve_api_key(provider_name, api_key, &state.credential_manager).await?;
            let dalle_provider = DallEProvider::new(api_key, None);
            dalle_provider.generate(request).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })
        },
        _ => Err(format!("Unknown provider: {}. Use 'flux' or 'dall-e'", provider_name)),
    }
}
