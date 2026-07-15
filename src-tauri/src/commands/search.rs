// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::search as search_err;
use axagent_harness::types::{CreateSearchProviderInput, SearchProvider};
use axagent_search::search::{SearchServiceConfig, execute_search_with_config};
use tauri::command;

/// 列出所有搜索提供商
#[command]
pub async fn list_search_providers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchProvider>, String> {
    axagent_dao::repo::search_provider::list_search_providers(state.harness.db())
        .await
        .map_err(|e| e.to_string())
}

/// 获取单个搜索提供商
#[command]
pub async fn get_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<SearchProvider, String> {
    axagent_dao::repo::search_provider::get_search_provider(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

/// 创建搜索提供商
#[command]
pub async fn create_search_provider(
    state: tauri::State<'_, AppState>,
    input: CreateSearchProviderInput,
) -> Result<SearchProvider, String> {
    // Encrypt API key before storing
    let mut input = input;
    if let Some(ref key) = input.api_key {
        if !key.is_empty() {
            input.api_key = Some(
                axagent_crypto::encrypt_key(key, state.harness.master_key())
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    axagent_dao::repo::search_provider::create_search_provider(state.harness.db(), input)
        .await
        .map_err(|e| e.to_string())
}

/// 更新搜索提供商
#[command]
pub async fn update_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
    mut input: CreateSearchProviderInput,
) -> Result<SearchProvider, String> {
    if let Some(ref key) = input.api_key {
        if !key.is_empty() {
            input.api_key = Some(
                axagent_crypto::encrypt_key(key, state.harness.master_key())
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    axagent_dao::repo::search_provider::update_search_provider(state.harness.db(), &id, input)
        .await
        .map_err(|e| e.to_string())
}

/// 删除搜索提供商
#[command]
pub async fn delete_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_dao::repo::search_provider::delete_search_provider(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取搜索提供商的 API key
async fn get_search_api_key(
    db: &sea_orm::DatabaseConnection,
    id: &str,
    master_key: &[u8; 32],
) -> Result<Option<String>, String> {
    use axagent_entities::search_providers;
    use sea_orm::EntityTrait;

    let model = search_providers::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("SearchProvider {} not found", id))?;

    match model.api_key_ref {
        Some(ref encrypted) if !encrypted.is_empty() => {
            axagent_crypto::decrypt_key(encrypted, master_key).map(Some).map_err(|e| e.to_string())
        },
        _ => Ok(None),
    }
}

/// 测试搜索提供商网络连通性（仅验证端点可达）
#[command]
pub async fn test_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use std::time::Instant;

    let provider = axagent_dao::repo::search_provider::get_search_provider(state.harness.db(), &id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(endpoint) = &provider.endpoint else {
        return Ok(
            serde_json::json!({ "ok": false, "error": ErrorResponse::new(search_err::ENDPOINT_NOT_CONFIGURED) }),
        );
    };

    let start = Instant::now();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    // Simple GET to check host reachability — doesn't validate API credentials
    match client.get(endpoint).send().await {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            let status = resp.status().as_u16();
            if status == 200 || status == 401 || status == 403 || status == 404 {
                // Server is reachable (401/403 = ok but needs auth, 404 = endpoint exists)
                Ok(serde_json::json!({ "ok": true, "latencyMs": latency, "resultCount": 0 }))
            } else {
                Ok(serde_json::json!({
                    "ok": false, "latencyMs": latency,
                    "error": format!("服务器返回 HTTP {}", status)
                }))
            }
        },
        Err(e) => Ok(serde_json::json!({
            "ok": false, "latencyMs": start.elapsed().as_millis() as u64,
            "error": e.to_string()
        })),
    }
}

/// 执行搜索
/// 通过 search crate 统一执行，当 provider 无效或未配置时自动降级到 DuckDuckGo。
#[command]
pub async fn execute_search(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    query: String,
) -> Result<serde_json::Value, String> {
    // 尝试从 DB 获取提供商配置
    let provider =
        axagent_dao::repo::search_provider::get_search_provider(state.harness.db(), &provider_id)
            .await
            .ok();

    let provider = match provider {
        Some(p) => p,
        None => {
            // 无匹配提供商 — 走 search crate 的 DDG 免费搜索
            let resp = axagent_search::search::execute_search("ddg", None, "", &query, 5, 15000)
                .await
                .map_err(|e| e.to_string())?;
            return serde_json::to_value(&resp).map_err(|e| e.to_string());
        },
    };

    // 尝试获取 API Key
    let api_key = get_search_api_key(state.harness.db(), &provider_id, state.harness.master_key())
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let has_api_key = !api_key.is_empty();
    let endpoint = provider.endpoint.clone();
    let has_endpoint = endpoint.as_ref().is_some_and(|e| !e.is_empty());

    let provider_type = &provider.provider_type;

    if has_api_key && has_endpoint {
        // 有完整配置 — 走 search crate 的统一入口
        let config = SearchServiceConfig {
            provider_type: provider_type.clone(),
            endpoint: endpoint.clone(),
            api_key: Some(api_key),
            max_results: provider.result_limit,
            timeout_ms: provider.timeout_ms,
            region: provider.region.clone(),
            safe_search: provider.safe_search.map(|b| if b { 1i32 } else { 0 }),
        };
        let resp = execute_search_with_config(&config, &query).await.map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?)
    } else if !api_key.is_empty() && !has_endpoint {
        // 有 API Key 但没 endpoint — 走 search crate 的 provider 特定搜索
        let config = SearchServiceConfig {
            provider_type: provider_type.clone(),
            endpoint: None,
            api_key: Some(api_key),
            max_results: provider.result_limit,
            timeout_ms: provider.timeout_ms,
            region: provider.region.clone(),
            safe_search: provider.safe_search.map(|b| if b { 1i32 } else { 0 }),
        };
        let resp = execute_search_with_config(&config, &query).await.map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?)
    } else {
        // 无 API Key 或无 endpoint — DDG 免费搜索降级
        let resp = axagent_search::search::execute_search("ddg", None, "", &query, 5, 15000)
            .await
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?)
    }
}
