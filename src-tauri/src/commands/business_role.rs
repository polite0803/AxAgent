// SPDX-License-Identifier: AGPL-3.0-only

//! 业务岗位命令 —— 对应现实业务岗位（CEO/CTO/产品经理 等）。
//!
//! 与 `agent_role` 命令（抽象执行器类型）区别：
//! - `agent_role`：executor/planner/researcher 等执行器类型，描述「怎么干活」
//! - `business_role`：CEO/CTO/产品经理 等业务岗位，描述「在组织里担什么责」
//!
//! 二者通过 `agent_profiles.business_role_id` + `agent_profiles.agent_role` 同时关联。

use agent_macro::agent_command;

use crate::AppState;
use axagent_dao::repo::business_role;
use axagent_harness::repo_dtos::BusinessRoleDto;
use serde::Deserialize;
use tauri::State;

/// 列出所有业务岗位
#[agent_command(domain = "agent", safety = Safe, call_mode = StateInput, description = "列出所有业务岗位")]
#[tauri::command]
pub async fn list_business_roles(
    app_state: State<'_, AppState>,
    source: Option<String>,
) -> Result<Vec<BusinessRoleDto>, String> {
    business_role::list_business_roles(app_state.harness.db(), source.as_deref()).await.map_err(
        |e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        },
    )
}

/// 查询业务岗位树（按 sort_order 排序，前端按 reports_to 自行构造树）
#[agent_command(domain = "agent", safety = Safe, call_mode = StateOnly, description = "查询业务岗位树")]
#[tauri::command]
pub async fn list_business_role_tree(
    app_state: State<'_, AppState>,
) -> Result<Vec<BusinessRoleDto>, String> {
    business_role::list_business_role_tree(app_state.harness.db()).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 获取单个业务岗位
#[agent_command(domain = "agent", safety = Safe, call_mode = StateInput, description = "获取单个业务岗位")]
#[tauri::command]
pub async fn get_business_role(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<BusinessRoleDto, String> {
    business_role::get_business_role(app_state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 创建/更新业务岗位的输入参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveBusinessRoleInput {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub responsibilities: Option<Vec<String>>,
    pub decision_authority: Option<String>,
    pub reports_to: Option<String>,
    pub managed_expert_ids: Option<Vec<String>>,
    pub required_certifications: Option<Vec<String>>,
    pub active_domains: Option<Vec<String>>,
    pub system_prompt: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub source: Option<String>,
    pub sort_order: Option<i32>,
}

/// 保存（创建/更新）业务岗位
#[agent_command(domain = "agent", safety = Caution, call_mode = StateInput, description = "保存业务岗位")]
#[tauri::command]
pub async fn save_business_role(
    app_state: State<'_, AppState>,
    input: SaveBusinessRoleInput,
) -> Result<BusinessRoleDto, String> {
    let source = input.source.as_deref().unwrap_or("custom");
    let sort_order = input.sort_order.unwrap_or(0);
    business_role::upsert_business_role(
        app_state.harness.db(),
        &input.id,
        &input.name,
        input.description.as_deref(),
        input.responsibilities.as_deref(),
        input.decision_authority.as_deref(),
        input.reports_to.as_deref(),
        input.managed_expert_ids.as_deref(),
        input.required_certifications.as_deref(),
        input.active_domains.as_deref(),
        &input.system_prompt,
        input.icon.as_deref(),
        input.color.as_deref(),
        source,
        sort_order,
    )
    .await
    .map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}

/// 删除业务岗位（builtin 不可删除；有下属岗位时拒绝删除）
#[agent_command(domain = "agent", safety = Dangerous, call_mode = StateInput, description = "删除业务岗位")]
#[tauri::command]
pub async fn delete_business_role(
    app_state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let role =
        business_role::get_business_role(app_state.harness.db(), &id).await.map_err(|e| {
            String::from(crate::commands::error::ErrorResponse::from_error(
                e,
                crate::commands::error::ErrorCategory::Unrecoverable,
            ))
        })?;

    if role.source == "builtin" {
        return Err("内置业务岗位不可删除".to_string());
    }

    business_role::delete_business_role(app_state.harness.db(), &id).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
