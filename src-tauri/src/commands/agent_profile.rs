use crate::AppState;
use axagent_core::repo::agent_profile;
use axagent_core::types::{AgentProfile, CreateAgentProfileInput, UpdateAgentProfileInput};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ImportAgentProfilesResult {
    pub count: u32,
    pub errors: Vec<String>,
}

/// 列出所有智能体能力集
#[tauri::command]
pub async fn list_agent_profiles(
    app_state: State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<AgentProfile>, String> {
    let db = &app_state.sea_db;
    agent_profile::list_agent_profiles(db, source.as_deref())
        .await
        .map_err(|e| e.to_string())
}

/// 根据 ID 获取智能体能力集
#[tauri::command]
pub async fn get_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<AgentProfile, String> {
    let db = &app_state.sea_db;
    agent_profile::get_agent_profile(db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 创建新的智能体能力集
#[tauri::command]
pub async fn create_agent_profile(
    app_state: State<'_, AppState>,
    input: CreateAgentProfileInput,
) -> Result<AgentProfile, String> {
    let db = &app_state.sea_db;
    let id = format!("custom-{}", axagent_core::utils::now_ts());

    let tags = input.tags.unwrap_or_default();
    agent_profile::upsert_agent_profile(
        db,
        &id,
        &input.name,
        input.description.as_deref(),
        input.category.as_deref().unwrap_or("general"),
        input.icon.as_deref().unwrap_or("🤖"),
        input.system_prompt.as_deref().unwrap_or(""),
        input.agent_role.as_deref(),
        input.source.as_deref().unwrap_or("custom"),
        &tags,
        input.suggested_provider_id.as_deref(),
        input.suggested_model_id.as_deref(),
        input.suggested_temperature,
        input.suggested_max_tokens.map(|v| v as i64),
        input.search_enabled,
        input.recommend_permission_mode.as_deref(),
        &input.recommended_tools.unwrap_or_default(),
        &input.disallowed_tools.unwrap_or_default(),
        &input.recommended_workflows.unwrap_or_default(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// 更新智能体能力集
#[tauri::command]
pub async fn update_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
    input: UpdateAgentProfileInput,
) -> Result<AgentProfile, String> {
    let db = &app_state.sea_db;
    agent_profile::update_agent_profile(
        db,
        &id,
        input.name.as_deref(),
        input.description.as_ref().map(|d| d.as_deref()),
        input.category.as_deref(),
        input.icon.as_deref(),
        input.system_prompt.as_deref(),
        input.agent_role.as_ref().map(|r| r.as_deref()),
        input.tags.as_deref(),
        input.is_enabled,
    )
    .await
    .map_err(|e| e.to_string())
}

/// 删除智能体能力集
#[tauri::command]
pub async fn delete_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = &app_state.sea_db;
    agent_profile::delete_agent_profile(db, &id)
        .await
        .map_err(|e| e.to_string())
}

/// 从 agency_experts 导入到 agent_profiles（兼容导入）
#[tauri::command]
pub async fn import_agent_profiles_from_agency(
    app_state: State<'_, AppState>,
) -> Result<ImportAgentProfilesResult, String> {
    let db = &app_state.sea_db;
    let mut count = 0u32;
    let mut errors = Vec::new();

    let rows = axagent_core::entity::agency_experts::Entity::find()
        .filter(axagent_core::entity::agency_experts::Column::IsEnabled.eq(1))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    for row in rows {
        let agent_profile_id = format!("agency-{}", row.id);
        let tags = vec![row.source_dir.clone(), row.category.clone()];
        let rec_tools = row
            .recommended_tools
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        let rec_wf = row
            .recommended_workflows
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();

        match agent_profile::upsert_agent_profile(
            db,
            &agent_profile_id,
            &row.name,
            row.description.as_deref(),
            &row.category,
            "🤖",
            &row.system_prompt,
            None, // agent_role 未在 agency_experts 中定义，留空
            "agency",
            &tags,
            None,
            None,
            None,
            None,
            None,
            None,
            &rec_tools,
            &[],
            &rec_wf,
        )
        .await
        {
            Ok(_) => count += 1,
            Err(e) => errors.push(format!("{}: {}", row.id, e)),
        }
    }

    Ok(ImportAgentProfilesResult { count, errors })
}
