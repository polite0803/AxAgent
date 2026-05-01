use crate::AppState;
use tauri::command;

/// 列出搜索引擎提供商（从 providers 表筛选 search 类型）
#[command]
pub async fn list_search_providers(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    use axagent_core::entity::providers;
    use sea_orm::EntityTrait;

    let providers = providers::Entity::find()
        .all(&state.sea_db)
        .await
        .map_err(|e| format!("DB error: {}", e))?;

    Ok(providers
        .into_iter()
        .filter(|p| p.provider_type == "search")
        .map(|p| serde_json::json!({
            "id": p.id,
            "name": p.name,
            "provider_type": p.provider_type,
            "api_host": p.api_host,
        }))
        .collect())
}

/// 删除搜索引擎提供商
#[command]
pub async fn delete_search_provider(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    use axagent_core::entity::providers;
    use sea_orm::EntityTrait;

    providers::Entity::delete_by_id(&id)
        .exec(&state.sea_db)
        .await
        .map_err(|e| format!("DB error: {}", e))?;
    Ok(())
}
