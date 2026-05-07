// 新用户引导 — 智能环境检测与快速预设

use crate::AppState;
use axagent_core::repo::provider::{
    add_provider_key, create_provider, list_providers, toggle_provider,
};
use axagent_core::types::{CreateProviderInput, ProviderType};
use serde::Serialize;
use tauri::State;

// ── 检测结果类型 ──

#[derive(Debug, Serialize)]
pub struct OllamaDetection {
    available: bool,
    models: Vec<OllamaModelInfo>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OllamaModelInfo {
    name: String,
    size_mb: Option<f64>,
    family: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DetectedApiKey {
    provider_type: String,
    prefix: String,
    env_var: String,
}

#[derive(Debug, Serialize)]
pub struct PresetResult {
    success: bool,
    provider_enabled: Option<String>,
    default_model_set: Option<String>,
    message: String,
}

// ── 环境变量名 → provider_type 映射 ──

const KEY_ENV_VARS: &[(&str, &str)] = &[
    ("OPENAI_API_KEY", "openai"),
    ("ANTHROPIC_API_KEY", "anthropic"),
    ("GEMINI_API_KEY", "gemini"),
    ("DEEPSEEK_API_KEY", "deepseek"),
    ("GROK_API_KEY", "xai"),
];

fn detect_single_key(env_var: &str, provider_type: &str) -> Option<DetectedApiKey> {
    let val = std::env::var(env_var).ok()?;
    if val.trim().is_empty() {
        return None;
    }
    let prefix: String = val.chars().take(8).collect();
    Some(DetectedApiKey {
        provider_type: provider_type.to_string(),
        prefix: format!("{}…", prefix),
        env_var: env_var.to_string(),
    })
}

// ── Tauri 命令 ──

/// 检测本地 Ollama 是否可用
#[tauri::command]
pub async fn detect_ollama_availability(ollama_host: Option<String>) -> Result<OllamaDetection, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let base = ollama_host.unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/tags", base.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            let models: Vec<OllamaModelInfo> = body["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| {
                            let info = &m["details"];
                            Some(OllamaModelInfo {
                                name: m["name"].as_str()?.to_string(),
                                size_mb: info["parameter_size"]
                                    .as_str()
                                    .and_then(|s| s.replace("B", "").parse::<f64>().ok()),
                                family: info["family"].as_str().map(|s| s.to_string()),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            Ok(OllamaDetection {
                available: !models.is_empty(),
                models,
                error: None,
            })
        },
        Ok(resp) => Ok(OllamaDetection {
            available: false,
            models: vec![],
            error: Some(format!("Ollama 响应异常: HTTP {}", resp.status())),
        }),
        Err(e) => {
            if e.is_connect() || e.is_timeout() {
                Ok(OllamaDetection {
                    available: false,
                    models: vec![],
                    error: None, // 静默：服务未运行
                })
            } else {
                Ok(OllamaDetection {
                    available: false,
                    models: vec![],
                    error: Some(format!("检测失败: {}", e)),
                })
            }
        },
    }
}

/// 检测环境变量中的 API Key
#[tauri::command]
pub async fn detect_api_keys() -> Result<Vec<DetectedApiKey>, String> {
    let keys: Vec<DetectedApiKey> = KEY_ENV_VARS
        .iter()
        .filter_map(|(env_var, provider_type)| detect_single_key(env_var, provider_type))
        .collect();
    Ok(keys)
}

/// 应用快速预设
#[tauri::command]
pub async fn apply_quick_start_preset(
    app_state: State<'_, AppState>,
    preset: String,
) -> Result<PresetResult, String> {
    let db = &app_state.sea_db;

    match preset.as_str() {
        "ollama" => {
            let existing = list_providers(db)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.provider_type == ProviderType::Ollama);

            let ollama_host = existing
                .as_ref()
                .map(|p| p.api_host.clone())
                .unwrap_or_else(|| "http://localhost:11434".to_string());

            let provider_id = if let Some(ref p) = existing {
                if !p.enabled {
                    toggle_provider(db, &p.id, true)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                p.id.clone()
            } else {
                let created = create_provider(
                    db,
                    CreateProviderInput {
                        name: "Ollama (本地)".into(),
                        provider_type: ProviderType::Ollama,
                        api_host: ollama_host.clone(),
                        api_path: Some("/v1/chat/completions".into()),
                        enabled: true,
                        builtin_id: None,
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
                created.id
            };

            Ok(PresetResult {
                success: true,
                provider_enabled: Some("ollama".into()),
                default_model_set: Some(provider_id),
                message: "Ollama 本地提供者已启用，请到设置中拉取模型列表".into(),
            })
        },

        "openai" => {
            let key_val = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            let key_prefix: String = key_val.chars().take(8).collect();

            let existing = list_providers(db)
                .await
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|p| p.provider_type == ProviderType::OpenAI);

            let pid = if let Some(ref p) = existing {
                if !p.enabled {
                    toggle_provider(db, &p.id, true)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                p.id.clone()
            } else {
                let created = create_provider(
                    db,
                    CreateProviderInput {
                        name: "OpenAI".into(),
                        provider_type: ProviderType::OpenAI,
                        api_host: "https://api.openai.com".into(),
                        api_path: Some("/v1/chat/completions".into()),
                        enabled: true,
                        builtin_id: None,
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
                created.id
            };

            if !key_val.is_empty() {
                add_provider_key(db, &pid, &key_val, &key_prefix)
                    .await
                    .map_err(|e| format!("添加 Key 失败: {}", e))?;
            }

            Ok(PresetResult {
                success: true,
                provider_enabled: Some("openai".into()),
                default_model_set: Some("gpt-4o".into()),
                message: if key_val.is_empty() {
                    "OpenAI 提供者已启用，请添加 API Key".into()
                } else {
                    "OpenAI 提供者已启用，API Key 已配置".into()
                },
            })
        },

        "minimal" => {
            let providers = list_providers(db).await.map_err(|e| e.to_string())?;
            let has_enabled = providers.iter().any(|p| p.enabled);

            Ok(PresetResult {
                success: true,
                provider_enabled: if has_enabled {
                    Some("已存在".into())
                } else {
                    None
                },
                default_model_set: None,
                message: if has_enabled {
                    "已检测到启用的提供者，你可以直接开始".into()
                } else {
                    "请在设置中添加模型供应商".into()
                },
            })
        },

        _ => Err(format!("未知预设: {}", preset)),
    }
}
