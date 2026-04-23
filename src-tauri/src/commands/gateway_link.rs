use crate::AppState;
use axagent_core::types::*;
use tauri::State;

/// Resolve the API key for a gateway link: if api_key_id is set, decrypt it;
/// otherwise return None.
async fn resolve_link_api_key(
    state: &AppState,
    link: &GatewayLink,
) -> Result<Option<String>, String> {
    if let Some(ref key_id) = link.api_key_id {
        let plain_key =
            axagent_core::repo::gateway_key::get_plain_key(&state.sea_db, &state.master_key, key_id)
                .await
                .map_err(|e| e.to_string())?;
        Ok(Some(plain_key))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn list_gateway_links(state: State<'_, AppState>) -> Result<Vec<GatewayLink>, String> {
    axagent_core::repo::gateway_link::list_gateway_links(&state.sea_db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_gateway_link(
    state: State<'_, AppState>,
    input: CreateGatewayLinkInput,
) -> Result<GatewayLink, String> {
    // If a plain-text api_key was provided, create a gateway key for it
    let resolved_input = if input.api_key_id.is_none() && input.api_key.is_some() {
        let key_name = format!("link:{}", input.name);
        let result = axagent_core::repo::gateway_key::create_gateway_key(
            &state.sea_db,
            &key_name,
            Some(&state.master_key),
        )
        .await
        .map_err(|e| e.to_string())?;

        CreateGatewayLinkInput {
            name: input.name,
            link_type: input.link_type,
            endpoint: input.endpoint,
            api_key_id: Some(result.gateway_key.id),
            api_key: None,
            auto_sync_models: input.auto_sync_models,
            auto_sync_skills: input.auto_sync_skills,
        }
    } else {
        input
    };

    axagent_core::repo::gateway_link::create_gateway_link(&state.sea_db, &resolved_input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_gateway_link(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    axagent_core::repo::gateway_link::delete_gateway_link(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn toggle_gateway_link(
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    axagent_core::repo::gateway_link::toggle_gateway_link(&state.sea_db, &id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_gateway_link(
    state: State<'_, AppState>,
    id: String,
) -> Result<GatewayLink, String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::connect_gateway_link(
        &state.sea_db,
        &id,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_gateway_link(
    state: State<'_, AppState>,
    id: String,
) -> Result<GatewayLink, String> {
    axagent_core::repo::gateway_link::disconnect_gateway_link(&state.sea_db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_link_status(
    state: State<'_, AppState>,
    id: String,
    status: String,
    error_message: Option<String>,
    latency_ms: Option<i64>,
    version: Option<String>,
) -> Result<GatewayLink, String> {
    axagent_core::repo::gateway_link::update_gateway_link_status(
        &state.sea_db,
        &id,
        &status,
        error_message.as_deref(),
        latency_ms,
        version.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_gateway_link_sync_settings(
    state: State<'_, AppState>,
    id: String,
    auto_sync_models: bool,
    auto_sync_skills: bool,
) -> Result<GatewayLink, String> {
    axagent_core::repo::gateway_link::update_gateway_link_sync_settings(
        &state.sea_db,
        &id,
        auto_sync_models,
        auto_sync_skills,
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_gateway_link_model_syncs(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<Vec<GatewayLinkModelSync>, String> {
    axagent_core::repo::gateway_link::get_gateway_link_model_syncs(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn push_gateway_link_models(
    state: State<'_, AppState>,
    link_id: String,
    model_ids: Vec<String>,
) -> Result<(), String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::push_gateway_link_models(
        &state.sea_db,
        &link_id,
        &model_ids,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_all_gateway_link_models(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<(), String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::sync_all_gateway_link_models(
        &state.sea_db,
        &link_id,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_gateway_link_skill_syncs(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<Vec<GatewayLinkSkillSync>, String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::get_gateway_link_skill_syncs(
        &state.sea_db,
        &link_id,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn push_gateway_link_skills(
    state: State<'_, AppState>,
    link_id: String,
    skill_names: Vec<String>,
) -> Result<(), String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::push_gateway_link_skills(
        &state.sea_db,
        &link_id,
        &skill_names,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_all_gateway_link_skills(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<(), String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    let api_key = resolve_link_api_key(&state, &link).await?;

    axagent_core::repo::gateway_link::sync_all_gateway_link_skills(
        &state.sea_db,
        &link_id,
        api_key.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_gateway_link_policy(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<Option<GatewayLinkPolicy>, String> {
    axagent_core::repo::gateway_link::get_gateway_link_policy(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_gateway_link_policy(
    state: State<'_, AppState>,
    link_id: String,
    input: SaveGatewayLinkPolicyInput,
) -> Result<GatewayLinkPolicy, String> {
    axagent_core::repo::gateway_link::save_gateway_link_policy(&state.sea_db, &link_id, &input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_gateway_link_activities(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<Vec<GatewayLinkActivity>, String> {
    axagent_core::repo::gateway_link::get_gateway_link_activities(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_gateway_conversation(
    state: State<'_, AppState>,
    link_id: String,
) -> Result<String, String> {
    let link = axagent_core::repo::gateway_link::get_gateway_link(&state.sea_db, &link_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Gateway link not found".to_string())?;

    if link.status != "connected" {
        return Err("Gateway is not connected".to_string());
    }

    let conversation = axagent_core::repo::conversation::create_conversation(
        &state.sea_db,
        &format!("Gateway: {}", link.name),
        &link.endpoint,
        &link_id,
        None,
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(conversation.id)
}
