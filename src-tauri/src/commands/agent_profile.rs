// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::{CommandError, ErrorCategory};
use agent_macro::agent_command;
use axagent_dao::repo::agent_profile;
use axagent_harness::types::{AgentProfile, CreateAgentProfileInput, UpdateAgentProfileInput};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct ImportAgentProfilesResult {
    pub count: u32,
    pub errors: Vec<String>,
}

/// 列出所有智能体能力集
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "列出智能体能力集")]
#[tauri::command]
pub async fn list_agent_profiles(
    app_state: State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<AgentProfile>, String> {
    let db = app_state.harness.db();
    Ok(agent_profile::list_agent_profiles(db, source.as_deref())
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

/// 根据 ID 获取智能体能力集
#[agent_command(domain = agent, safety = Safe, call_mode = StateInput, description = "获取智能体能力集详情")]
#[tauri::command]
pub async fn get_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<AgentProfile, String> {
    let db = app_state.harness.db();
    Ok(agent_profile::get_agent_profile(db, &id)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

/// 创建新的智能体能力集
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "创建智能体能力集")]
#[tauri::command]
pub async fn create_agent_profile(
    app_state: State<'_, AppState>,
    input: CreateAgentProfileInput,
) -> Result<AgentProfile, String> {
    let db = app_state.harness.db();
    let id = format!("custom-{}", axagent_kit::utils::now_ts());

    let tags = input.tags.unwrap_or_default();
    Ok(agent_profile::upsert_agent_profile(
        db,
        &id,
        &input.name,
        input.description.as_deref(),
        input.category.as_deref().unwrap_or("general"),
        input.icon.as_deref().unwrap_or("🤖"),
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
        None, // expert_id is set via import or manual binding
        input.business_role_id.as_deref(),
    )
    .await
    .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

/// 更新智能体能力集
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "更新智能体能力集")]
#[tauri::command]
pub async fn update_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
    input: UpdateAgentProfileInput,
) -> Result<AgentProfile, String> {
    let db = app_state.harness.db();
    Ok(agent_profile::update_agent_profile(
        db,
        &id,
        input.name.as_deref(),
        input.description.as_ref().map(|d| d.as_deref()),
        input.category.as_deref(),
        input.icon.as_deref(),
        input.agent_role.as_ref().map(|r| r.as_deref()),
        input.tags.as_deref(),
        input.is_enabled,
        input.business_role_id.as_ref().map(|r| r.as_deref()),
    )
    .await
    .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

/// 删除智能体能力集
#[agent_command(domain = agent, safety = Dangerous, call_mode = StateInput, description = "删除智能体能力集")]
#[tauri::command]
pub async fn delete_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let db = app_state.harness.db();
    Ok(agent_profile::delete_agent_profile(db, &id)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?)
}

/// 从 agency_experts 导入到 agent_profiles（兼容导入）
///
/// 修复问题 13：导入时显式赋 "executor" 作为 agent_role，并确保 DB 中存在该 role。
/// 原实现 agent_role 留空，导致所有从 agency 导入的 profile 永远没有 role 提示词。
#[agent_command(domain = agent, safety = Caution, call_mode = StateOnly, description = "从机构专家导入能力集")]
#[tauri::command]
pub async fn import_agent_profiles_from_agency(
    app_state: State<'_, AppState>,
) -> Result<ImportAgentProfilesResult, String> {
    let db = app_state.harness.db();
    let mut count = 0u32;
    let mut errors = Vec::new();

    // 修复问题 13+14：确保 DB 中存在 "executor" 种子 role，作为导入 profile 的默认岗位
    ensure_default_executor_role(db).await;

    let rows = axagent_entities::agency_experts::Entity::find()
        .filter(axagent_entities::agency_experts::Column::IsEnabled.eq(1))
        .all(db)
        .await
        .map_err(|e| CommandError::from_error(e, ErrorCategory::Unrecoverable))?;

    for row in rows {
        let agent_profile_id = row.id.clone();
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
            Some("executor"), // 修复问题 13：显式赋 "executor" role，避免 profile 永远无 role 提示词
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
            Some(&row.id), // 关联 Expert，运行时拼接其 system_prompt
            None,          // agency 导入不带 business_role_id，由用户手动关联
        )
        .await
        {
            Ok(_) => count += 1,
            Err(e) => errors.push(format!("{}: {}", row.id, e)),
        }
    }

    Ok(ImportAgentProfilesResult { count, errors })
}

/// 确保 DB 中存在 "executor" 默认 role（导入 agency profile 时使用）。
///
/// 修复问题 14：原 `resolve_role` 在 profile 无 agent_role 时 unwrap_or("executor")，
/// 但 DB 中不一定有 "executor" 记录，导致 domain_constraints 查不到约束。
/// 此函数在导入流程中幂等地确保该种子记录存在。
async fn ensure_default_executor_role(db: &sea_orm::DatabaseConnection) {
    use axagent_entities::agent_roles;

    const DEFAULT_ROLE_NAME: &str = "executor";

    let existing = agent_roles::Entity::find()
        .filter(agent_roles::Column::Name.eq(DEFAULT_ROLE_NAME))
        .one(db)
        .await;

    if matches!(existing, Ok(None)) {
        let now = chrono::Utc::now().timestamp_millis();
        let am = agent_roles::ActiveModel {
            id: Set(DEFAULT_ROLE_NAME.to_string()),
            name: Set(DEFAULT_ROLE_NAME.to_string()),
            description: Set(Some("默认执行器岗位（agency 导入自动创建）".to_string())),
            system_prompt: Set(String::new()),
            default_tools: Set(None),
            active_domains: Set(None),
            max_concurrent: Set(3),
            timeout_seconds: Set(600),
            source: Set("builtin".to_string()),
            sort_order: Set(0),
            created_at: Set(now),
            updated_at: Set(now),
        };
        if let Err(e) = agent_roles::Entity::insert(am).exec(db).await {
            tracing::warn!(
                role = DEFAULT_ROLE_NAME,
                error = %e,
                "ensure_default_executor_role: 创建默认 executor role 失败（可能已被并发创建）"
            );
        }
    }
}

/// 确保 AgentProfile 在 DB 中存在（选择 Expert/Role 时自动调用）
/// 如果已有同 ID profile 则跳过，否则创建最小 profile（仅绑定 expert_id）
#[agent_command(domain = agent, safety = Caution, call_mode = StateInput, description = "确保 AgentProfile 存在")]
#[tauri::command]
pub async fn ensure_agent_profile(
    app_state: State<'_, AppState>,
    id: String,
    name: String,
    expert_id: Option<String>,
    agent_role: Option<String>,
) -> Result<String, String> {
    let db = app_state.harness.db();

    // 已存在则直接返回
    if axagent_dao::repo::agent_profile::get_agent_profile(db, &id).await.is_ok() {
        return Ok(id);
    }

    // 创建最小 profile
    axagent_dao::repo::agent_profile::create_agent_profile(
        db,
        &id,
        &name,
        None,
        "general",
        "🤖",
        agent_role.as_deref(),
        "agency",
        &[],
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })?;

    // 如果有 expert_id，设置绑定
    if let Some(ref eid) = expert_id {
        use sea_orm::{ActiveModelTrait, EntityTrait, Set};
        if let Some(row) = axagent_entities::agent_profiles::Entity::find_by_id(&id)
            .one(db)
            .await
            .map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })? {
            let mut am: axagent_entities::agent_profiles::ActiveModel = row.into();
            am.expert_id = Set(Some(eid.clone()));
            am.updated_at = Set(axagent_kit::utils::now_ts());
            am.update(db).await.map_err(|e| {
                String::from(crate::commands::error::ErrorResponse::from_error(
                    e,
                    crate::commands::error::ErrorCategory::Unrecoverable,
                ))
            })?;
        }
    }

    Ok(id)
}
