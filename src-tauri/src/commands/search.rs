use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::search as search_err;
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
    // Encrypt API key before storing
    let mut input = input;
    if let Some(ref key) = input.api_key {
        if !key.is_empty() {
            input.api_key = Some(
                axagent_core::crypto::encrypt_key(key, state.harness.master_key())
                    .map_err(|e| e.to_string())?,
            );
        }
    }
    axagent_core::repo::search_provider::create_search_provider(&state.sea_db, input)
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
                axagent_core::crypto::encrypt_key(key, state.harness.master_key())
                    .map_err(|e| e.to_string())?,
            );
        }
    }
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
async fn get_search_api_key(
    db: &sea_orm::DatabaseConnection,
    id: &str,
    master_key: &[u8; 32],
) -> Result<Option<String>, String> {
    use axagent_core::entity::search_providers;
    use sea_orm::EntityTrait;

    let model = search_providers::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("SearchProvider {} not found", id))?;

    match model.api_key_ref {
        Some(ref encrypted) if !encrypted.is_empty() => {
            axagent_core::crypto::decrypt_key(encrypted, master_key)
                .map(Some)
                .map_err(|e| e.to_string())
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

    let provider = axagent_core::repo::search_provider::get_search_provider(&state.sea_db, &id)
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
/// 当 provider_id 无效或提供商未配置时，自动降级到 DuckDuckGo 免费搜索。
#[command]
pub async fn execute_search(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    query: String,
) -> Result<serde_json::Value, String> {
    // 尝试从 DB 获取提供商配置，失败则走 DDG 免费搜索
    let provider =
        match axagent_core::repo::search_provider::get_search_provider(&state.sea_db, &provider_id)
            .await
        {
            Ok(p) => p,
            Err(_) => {
                // 无匹配提供商 — 直接走 DuckDuckGo 免费搜索
                return search_via_ddg(&query).await;
            },
        };

    // 提供商无 API Key 或 endpoint → 走 DDG 免费搜索
    let api_key: Option<String> =
        match get_search_api_key(&state.sea_db, &provider_id, state.harness.master_key()).await {
            Ok(Some(k)) if !k.is_empty() => Some(k),
            _ => None,
        };

    let Some(endpoint) = &provider.endpoint else {
        return search_via_ddg(&query).await;
    };
    if api_key.is_none() {
        return search_via_ddg(&query).await;
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
                .map(|item| {
                    serde_json::json!({
                        "title": item.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        "content": item.get("snippet").or(item.get("content")).or(item.get("description")).and_then(|v| v.as_str()).unwrap_or(""),
                        "url": item.get("url").or(item.get("link")).and_then(|v| v.as_str()).unwrap_or(""),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(serde_json::json!({
        "ok": true,
        "results": results,
    }))
}

/// DuckDuckGo 免费搜索兜底 — 无需 API Key，始终可用。
async fn search_via_ddg(query: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    tracing::debug!("[search] DDG fallback: GET {}", url);

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .map_err(|e| format!("DDG request failed: {e}"))?;

    let status = resp.status();
    let html = resp
        .text()
        .await
        .map_err(|e| format!("DDG read failed: {e}"))?;

    if !status.is_success() {
        tracing::warn!("[search] DDG returned HTTP {}", status.as_u16());
        return Ok(serde_json::json!({"ok": true, "results": [], "provider": "ddg"}));
    }

    // 简易 HTML 解析：提取 .result__a 和 .result__snippet
    let results: Vec<serde_json::Value> = html
        .split("result__a")
        .skip(1)
        .filter_map(|chunk| {
            let title = chunk
                .split("</a>")
                .next()?
                .rsplit('>')
                .next()?
                .trim()
                .to_string();
            let snippet = chunk
                .split("result__snippet")
                .nth(1)?
                .split("</td>")
                .next()?
                .rsplit('>')
                .next()?
                .trim()
                .to_string();
            if title.is_empty() && snippet.is_empty() {
                return None;
            }
            Some(serde_json::json!({
                "title": title,
                "content": snippet,
                "url": "",
            }))
        })
        .take(5)
        .collect();

    tracing::debug!("[search] DDG parsed {} results for '{}'", results.len(), query);

    Ok(serde_json::json!({
        "ok": true,
        "results": results,
        "provider": "ddg",
    }))
}
