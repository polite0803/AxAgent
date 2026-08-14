// SPDX-License-Identifier: AGPL-3.0-only
//! 能力发现 Tauri 命令集
//!
//! 暴露能力注册、发现、索引管理等命令给前端调用。
//! 所有命令从 AppState 获取已注入的路由器和索引器实例。

use crate::AppState;
use crate::commands::error::{CommandError, ErrorCategory, ErrorResponse};
use agent_macro::agent_command;
use axagent_harness::{
    CapabilityDiscoveryRequest, CapabilityDiscoveryResult, CapabilityIndexer,
    CapabilityPassportDto, CapabilityQuery, DiscoveryWeights, FilterContext, SessionBudget,
};
use serde::{Deserialize, Serialize};
use tauri::State;

/// 运行时能力注册表检视 DTO（P3：外部插件注册的可查询闭环）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRegistrationDetailDto {
    pub id: String,
    pub version: String,
    pub contract: String,
    pub description: String,
    pub origin: String,
    pub plugin_id: Option<String>,
}

// ── DTO 类型 ──────────────────────────────────────

/// 注册能力护照的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterPassportRequest {
    pub passport: CapabilityPassportDto,
}

/// 能力发现的请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverRequest {
    pub user_input: String,
    #[serde(default)]
    pub filter_context: Option<FilterContext>,
    #[serde(default)]
    pub query: Option<CapabilityQuery>,
    #[serde(default)]
    pub weights: Option<DiscoveryWeights>,
    #[serde(default)]
    pub budget: Option<SessionBudget>,
    #[serde(default = "default_true")]
    pub enable_completion: bool,
    #[serde(default = "default_false")]
    pub enable_circuit_breaker: bool,
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

// ── Tauri 命令 ────────────────────────────────────

/// 注册一个能力护照到索引
#[agent_command(domain = capability, safety = Caution, call_mode = StateInput, description = "注册能力护照")]
#[tauri::command]
pub async fn capability_register_passport(
    state: State<'_, AppState>,
    request: RegisterPassportRequest,
) -> Result<axagent_harness::IndexResult, CommandError> {
    state.capability_indexer.index_passport(&request.passport).await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::REGISTER_FAILED,
            e,
            ErrorCategory::Unrecoverable,
        )
    })
}

/// 批量注册能力护照
#[agent_command(domain = capability, safety = Caution, call_mode = StateInput, description = "批量注册能力护照")]
#[tauri::command]
pub async fn capability_register_batch(
    state: State<'_, AppState>,
    passports: Vec<CapabilityPassportDto>,
) -> Result<Vec<axagent_harness::IndexResult>, CommandError> {
    Ok(state.capability_indexer.index_batch(&passports).await)
}

/// 执行能力发现管线
#[agent_command(domain = capability, safety = Safe, call_mode = StateInput, description = "执行能力发现管线")]
#[tauri::command]
pub async fn capability_discover(
    state: State<'_, AppState>,
    request: DiscoverRequest,
) -> Result<CapabilityDiscoveryResult, CommandError> {
    let mut query = request.query.unwrap_or_default();
    // 确保 query.user_input 使用用户实际输入
    if query.user_input.is_empty() {
        query.user_input = request.user_input.clone();
    }
    let filter_context = request.filter_context.unwrap_or_default();
    let weights = request.weights.unwrap_or_default();
    let budget = request.budget.unwrap_or_default();

    let discovery_request = CapabilityDiscoveryRequest {
        user_input: request.user_input,
        filter_context,
        query,
        weights,
        budget,
        enable_completion: request.enable_completion,
        enable_circuit_breaker: request.enable_circuit_breaker,
        enable_rar: false,
        rar_top_k: 5,
    };

    axagent_harness::CapabilityRouter::discover(
        state.capability_router.as_ref(),
        &discovery_request,
    )
    .await
    .map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::DISCOVER_FAILED,
            e,
            ErrorCategory::Retryable,
        )
    })
}

/// 列出已注册的能力
#[agent_command(domain = capability, safety = Safe, call_mode = StateOnly, description = "列出已注册的能力")]
#[tauri::command]
pub async fn capability_list_passports(
    state: State<'_, AppState>,
) -> Result<Vec<CapabilityPassportDto>, CommandError> {
    let ids = state.capability_indexer.list_capability_ids().await;
    let mut passports = Vec::new();
    for id in ids {
        if let Some(passport) = state.capability_indexer.get_passport(&id).await {
            passports.push(passport);
        }
    }
    Ok(passports)
}

/// 删除一个能力
#[agent_command(domain = capability, safety = Dangerous, call_mode = StateInput, description = "删除能力护照")]
#[tauri::command]
pub async fn capability_remove_passport(
    state: State<'_, AppState>,
    capability_id: String,
) -> Result<(), CommandError> {
    state.capability_indexer.remove_index(&capability_id).await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::NOT_FOUND,
            e,
            ErrorCategory::Unrecoverable,
        )
    })
}

/// 获取索引统计信息
#[agent_command(domain = capability, safety = Safe, call_mode = StateOnly, description = "获取能力索引统计信息")]
#[tauri::command]
pub async fn capability_get_stats(
    state: State<'_, AppState>,
) -> Result<axagent_harness::CapabilityIndexStats, CommandError> {
    state.capability_indexer.get_stats().await.map_err(|e| {
        ErrorResponse::from_error_with_code(
            crate::commands::error_code::capability::STATS_FAILED,
            e,
            ErrorCategory::Retryable,
        )
    })
}

/// 列出运行时能力注册表（P3：内置与外部插件平权的可查询检视闭环）。
///
/// 返回全部已注册能力及其来源；外部插件注册的能力额外标注来源插件 ID。
#[agent_command(domain = plugin, safety = Safe, call_mode = StateOnly, description = "列出运行时能力注册表")]
#[tauri::command]
pub async fn capability_registry_dump()
-> Result<Vec<CapabilityRegistrationDetailDto>, ErrorResponse> {
    Ok(axagent_harness::get_capability_registry()
        .list_with_details()
        .into_iter()
        .map(|d| CapabilityRegistrationDetailDto {
            id: d.definition.id,
            version: d.definition.version,
            contract: d.definition.contract,
            description: d.definition.description,
            origin: d.origin.as_str().to_string(),
            plugin_id: d.plugin_id,
        })
        .collect())
}
