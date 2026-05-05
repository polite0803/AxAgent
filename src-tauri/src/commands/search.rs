use crate::AppState;
use axagent_core::types::{CreateSearchProviderInput, SearchProvider};
use tauri::command;

/// 列出所有搜索提供商
#[command]
pub async fn list_search_providers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<SearchProvider>, String> {
    axagent_core::repo::search_provider::list_search_providers(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

/// 获取单个搜索提供商
#[command]
pub async fn get_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<SearchProvider, String> {
    axagent_core::repo::search_provider::get_search_provider(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 创建搜索提供商
#[command]
pub async fn create_search_provider(
    state: tauri::State<'_, AppState>,
    input: CreateSearchProviderInput,
) -> Result<SearchProvider, String> {
    axagent_core::repo::search_provider::create_search_provider(&state.sea_db, input)
        .await
        .map_err(|e| e.to_string())
}

/// 更新搜索提供商
#[command]
pub async fn update_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
    input: CreateSearchProviderInput,
) -> Result<SearchProvider, String> {
    axagent_core::repo::search_provider::update_search_provider(&state.sea_db, &id, input)
        .await
        .map_err(|e| e.to_string())
}

/// 删除搜索提供商
#[command]
pub async fn delete_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_core::repo::search_provider::delete_search_provider(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 获取搜索提供商的 API key
async fn get_search_api_key(db: &sea_orm::DatabaseConnection, id: &str) -> Result<Option<String>, String> {
    use axagent_core::entity::search_providers;
    use sea_orm::EntityTrait;

    let model = search_providers::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("SearchProvider {} not found", id))?;

    Ok(model.api_key_ref)
}

/// 测试搜索提供商网络连通性（仅验证端点可达）
#[command]
pub async fn test_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<serde_json::Value, String> {
    use std::time::Instant;

    let provider = axagent_core::repo::search_provider::get_search_provider(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?;

    let Some(endpoint) = &provider.endpoint else {
        return Ok(serde_json::json!({ "ok": false, "error": "未配置端点" }));
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
        }
        Err(e) => {
            Ok(serde_json::json!({
                "ok": false, "latencyMs": start.elapsed().as_millis() as u64,
                "error": e.to_string()
            }))
        }
    }
}

/// 执行搜索
#[command]
pub async fn execute_search(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    query: String,
) -> Result<serde_json::Value, String> {
    let provider = axagent_core::repo::search_provider::get_search_provider(&state.sea_db, &provider_id)
        .await
        .map_err(|e| e.to_string())?;

    let api_key = get_search_api_key(&state.sea_db, &provider_id).await?;

    let Some(endpoint) = &provider.endpoint else {
        return Err("搜索提供商未配置端点".to_string());
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(provider.timeout_ms as u64))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client
        .post(endpoint)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "q": query,
            "max_results": provider.result_limit
        }));
    if let Some(ref key) = api_key {
        req = req.header("X-API-Key", key);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;

    let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let results = json
        .get("results")
        .or_else(|| json.get("organic"))
        .or_else(|| json.get("data"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    Some(serde_json::json!({
                        "title": item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        "content": item.get("snippet").or(item.get("content")).or(item.get("description")).and_then(|v| v.as_str()).unwrap_or(""),
                        "url": item.get("url").or(item.get("link")).and_then(|v| v.as_str()).unwrap_or(""),
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(serde_json::json!({
        "ok": true,
        "results": results,
    }))
}
