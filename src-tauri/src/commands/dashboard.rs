// SPDX-License-Identifier: AGPL-3.0-only

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::dashboard as dashboard_err;
use axagent_runtime::dashboard_plugin::{DashboardPluginAdapter, DashboardPluginManifest};
use axagent_runtime::dashboard_registry::DashboardPluginInfo;
use sea_orm::entity::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

fn default_plugins_dir() -> PathBuf {
    axagent_storage::storage_paths::documents_root().join("dashboard-plugins")
}

#[tauri::command]
pub async fn dashboard_list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<DashboardPluginInfo>, String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    Ok(registry.list_plugins().await)
}

#[tauri::command]
pub async fn dashboard_register_plugin(
    state: State<'_, AppState>,
    manifest_json: String,
) -> Result<(), String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    let manifest: DashboardPluginManifest =
        serde_json::from_str(&manifest_json).map_err(|e| e.to_string())?;

    let frontend_entry = manifest.frontend_entry.clone();
    let plugin = DashboardPluginAdapter::new(manifest, move |panel_id, props| {
        let panel_info = serde_json::json!({
            "panel_id": panel_id,
            "props": props,
            "frontend_entry": frontend_entry,
        });
        axagent_runtime::dashboard_plugin::RenderOutput::Html { content: panel_info.to_string() }
    });

    registry.register(Box::new(plugin)).await
}

#[tauri::command]
pub async fn dashboard_unregister_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.unregister(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_enable_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.enable(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_disable_plugin(
    state: State<'_, AppState>,
    plugin_id: String,
) -> Result<(), String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.disable(&plugin_id).await
}

#[tauri::command]
pub async fn dashboard_render_panel(
    state: State<'_, AppState>,
    plugin_id: String,
    panel_id: String,
    props: std::collections::HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.render_panel(&plugin_id, &panel_id, props).await.map(|r| match r {
        axagent_runtime::dashboard_plugin::RenderOutput::Html { content } => content,
        axagent_runtime::dashboard_plugin::RenderOutput::Data { payload } => payload.to_string(),
        axagent_runtime::dashboard_plugin::RenderOutput::Directive(d) => {
            serde_json::to_string(&d).unwrap_or_default()
        },
    })
}

#[tauri::command]
pub async fn dashboard_reload_plugins(state: State<'_, AppState>) -> Result<(), String> {
    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.reload().await
}

#[tauri::command]
pub async fn dashboard_open_plugins_folder(app: tauri::AppHandle) -> Result<(), String> {
    let dir = default_plugins_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create plugins dir: {}", e))?;
    use tauri_plugin_opener::OpenerExt;
    app.opener().reveal_item_in_dir(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dashboard_install_plugin(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<(), String> {
    let source = PathBuf::from(&source_path);
    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    let plugins_dir = default_plugins_dir();
    std::fs::create_dir_all(&plugins_dir)
        .map_err(|e| format!("Failed to create plugins dir: {}", e))?;

    let plugin_dir_name =
        source.file_stem().and_then(|s| s.to_str()).unwrap_or("plugin").to_string();
    let dest_dir = plugins_dir.join(&plugin_dir_name);

    if source.is_dir() {
        if source.join("manifest.json").exists() {
            let dest = dest_dir;
            copy_dir_recursive(&source, &dest)?;
        } else {
            return Err(ErrorResponse::err(dashboard_err::NO_MANIFEST));
        }
    } else if source.extension().and_then(|e| e.to_str()) == Some("json") {
        let manifest_str = std::fs::read_to_string(&source)
            .map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest: DashboardPluginManifest =
            serde_json::from_str(&manifest_str).map_err(|e| format!("Invalid manifest: {}", e))?;
        let dest_dir = plugins_dir.join(&manifest.id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("Failed to create plugin dir: {}", e))?;
        std::fs::copy(&source, dest_dir.join("manifest.json")).map_err(|e| {
            ErrorResponse::new(dashboard_err::COPY_MANIFEST_FAILED)
                .with_detail(format!("Failed to copy manifest: {}", e))
        })?;
    } else {
        return Err(ErrorResponse::err(dashboard_err::NO_MANIFEST));
    }

    let registry = state.dashboard_registry.as_ref().ok_or("Dashboard registry not initialized")?;
    registry.reload().await
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir: {}", e))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub total_conversations: i64,
    pub total_messages: i64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub total_tokens: i64,
    pub total_agent_sessions: i64,
    pub completed_agent_sessions: i64,
    pub failed_agent_sessions: i64,
    pub total_agent_tokens: i64,
    pub total_cost_usd: f64,
    pub total_tool_calls: i64,
    /// 今日（本地时区）消息数
    pub today_messages: i64,
    /// 今日（本地时区）输入 token 数
    pub today_prompt_tokens: i64,
    /// 今日（本地时区）输出 token 数
    pub today_completion_tokens: i64,
    /// 今日（本地时区）总 token 数 = today_prompt_tokens + today_completion_tokens
    pub today_tokens: i64,
}

#[tauri::command]
pub async fn get_dashboard_stats(state: State<'_, AppState>) -> Result<DashboardStats, String> {
    let db = state.harness.db();

    // 会话总数
    let total_conversations = axagent_entities::conversations::Entity::find()
        .count(db)
        .await
        .map_err(|e| e.to_string())?;

    // 消息聚合查询：COUNT + SUM tokens，避免全表加载到内存
    let msg_stats = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT \
             COUNT(*) as total_messages, \
             COALESCE(SUM(prompt_tokens), 0) as total_prompt_tokens, \
             COALESCE(SUM(completion_tokens), 0) as total_completion_tokens, \
             COALESCE(SUM(token_count), 0) as total_token_count \
             FROM messages WHERE is_active = 1",
            vec![],
        ))
        .await
        .map_err(|e| e.to_string())?;

    let first_row = msg_stats.first();
    let total_messages: i64 =
        first_row.and_then(|r| r.try_get("", "total_messages").ok()).unwrap_or(0);
    let total_prompt_tokens: i64 =
        first_row.and_then(|r| r.try_get("", "total_prompt_tokens").ok()).unwrap_or(0);
    let total_completion_tokens: i64 =
        first_row.and_then(|r| r.try_get("", "total_completion_tokens").ok()).unwrap_or(0);
    let total_token_count: i64 =
        first_row.and_then(|r| r.try_get("", "total_token_count").ok()).unwrap_or(0);

    let total_tokens = if total_prompt_tokens > 0 || total_completion_tokens > 0 {
        total_prompt_tokens + total_completion_tokens
    } else {
        total_token_count
    };

    // 今日（本地时区）消息统计：messages.created_at 是毫秒时间戳
    let today_start_millis = axagent_harness::util_fns::today_start_local_ts() * 1000;
    let today_msg_stats = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT \
             COUNT(*) as today_messages, \
             COALESCE(SUM(prompt_tokens), 0) as today_prompt_tokens, \
             COALESCE(SUM(completion_tokens), 0) as today_completion_tokens \
             FROM messages WHERE is_active = 1 AND created_at >= ?",
            vec![today_start_millis.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
    let today_row = today_msg_stats.first();
    let today_messages: i64 =
        today_row.and_then(|r| r.try_get("", "today_messages").ok()).unwrap_or(0);
    let today_prompt_tokens: i64 =
        today_row.and_then(|r| r.try_get("", "today_prompt_tokens").ok()).unwrap_or(0);
    let today_completion_tokens: i64 =
        today_row.and_then(|r| r.try_get("", "today_completion_tokens").ok()).unwrap_or(0);
    let today_tokens = today_prompt_tokens + today_completion_tokens;

    // 智能体会话聚合查询
    let session_stats = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT \
             COUNT(*) as total_sessions, \
             COALESCE(SUM(total_tokens), 0) as total_agent_tokens, \
             COALESCE(SUM(total_cost_usd), 0.0) as total_cost_usd, \
             COALESCE(SUM(CASE WHEN runtime_status = 'completed' THEN 1 ELSE 0 END), 0) as completed, \
             COALESCE(SUM(CASE WHEN runtime_status = 'failed' THEN 1 ELSE 0 END), 0) as failed \
             FROM agent_sessions",
            vec![],
        ))
        .await
        .map_err(|e| e.to_string())?;

    let session_row = session_stats.first();
    let total_agent_sessions: i64 =
        session_row.and_then(|r| r.try_get("", "total_sessions").ok()).unwrap_or(0);
    let total_agent_tokens: i64 =
        session_row.and_then(|r| r.try_get("", "total_agent_tokens").ok()).unwrap_or(0);
    let total_cost_usd: f64 =
        session_row.and_then(|r| r.try_get("", "total_cost_usd").ok()).unwrap_or(0.0);
    let completed_agent_sessions: i64 =
        session_row.and_then(|r| r.try_get("", "completed").ok()).unwrap_or(0);
    let failed_agent_sessions: i64 =
        session_row.and_then(|r| r.try_get("", "failed").ok()).unwrap_or(0);

    // 工具调用统计
    let total_tool_calls = axagent_entities::tool_executions::Entity::find()
        .count(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(DashboardStats {
        total_conversations: total_conversations as i64,
        total_messages,
        total_prompt_tokens,
        total_completion_tokens,
        total_tokens,
        total_agent_sessions,
        completed_agent_sessions,
        failed_agent_sessions,
        total_agent_tokens,
        total_cost_usd,
        total_tool_calls: total_tool_calls as i64,
        today_messages,
        today_prompt_tokens,
        today_completion_tokens,
        today_tokens,
    })
}

/// 按提供商统计网关使用量。成本数据由 agent_sessions 跟踪，此处仅返回用量。
#[tauri::command]
pub async fn get_cost_by_provider(
    state: State<'_, AppState>,
) -> Result<Vec<axagent_harness::types::CostByProvider>, String> {
    let db = state.harness.db();

    let rows = db
        .query_all_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            "SELECT gu.provider_id, \
             COUNT(*) as request_count, \
             COALESCE(SUM(gu.request_tokens + gu.response_tokens), 0) as token_count \
             FROM gateway_usage gu \
             GROUP BY gu.provider_id \
             ORDER BY token_count DESC",
            vec![],
        ))
        .await
        .map_err(|e| e.to_string())?;

    let results: Vec<axagent_harness::types::CostByProvider> = rows
        .iter()
        .map(|r| axagent_harness::types::CostByProvider {
            provider_id: r.try_get("", "provider_id").unwrap_or_default(),
            request_count: r.try_get("", "request_count").unwrap_or(0),
            token_count: r.try_get("", "token_count").unwrap_or(0),
            cost_usd: 0.0,
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn get_usage_trend(
    state: State<'_, AppState>,
    days: Option<u32>,
) -> Result<Vec<axagent_harness::types::DailyUsage>, String> {
    let db = state.harness.db();
    let days = days.unwrap_or(30);
    axagent_dao::repo::message::get_daily_message_usage(db, days).await.map_err(|e| e.to_string())
}
