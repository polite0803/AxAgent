// SPDX-License-Identifier: AGPL-3.0-only

//! 应用配置持久化命令
//!
//! 提供前端 appConfigStore 的后端持久化支持。

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::storage as storage_err;
use axagent_agent_macro::agent_command;
use tauri::State;

#[agent_command(domain = settings, safety = Safe, call_mode = StateOnly, description = "获取应用配置")]
#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.harness.db();
    match axagent_dao::repo::settings::get_setting(db, "app_config").await {
        Ok(Some(json_str)) => {
            serde_json::from_str(&json_str).map_err(|e| format!("解析配置失败: {}", e))
        },
        Ok(None) => Ok(serde_json::json!({})),
        Err(e) => Err(ErrorResponse::new(storage_err::READ_FILE_FAILED)
            .with_detail(format!("读取配置失败: {}", e))
            .into()),
    }
}

#[agent_command(domain = settings, safety = Caution, call_mode = StateInput, description = "保存应用配置")]
#[tauri::command]
pub async fn save_app_config(
    state: State<'_, AppState>,
    config: serde_json::Value,
) -> Result<(), String> {
    let db = state.harness.db();
    let json_str = serde_json::to_string(&config).map_err(|e| format!("序列化配置失败: {}", e))?;
    axagent_dao::repo::settings::set_setting(db, "app_config", &json_str)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))
}

/// 缺陷1修复:从数据库读取前端 FeatureFlag,返回自改进循环相关的两个 flag 值。
///
/// 供 wiring 层(`init/state.rs`)在构造 `AppState` 时调用,把前端开关
/// 桥接到 `SessionManager::set_self_improvement_flags()`:
/// - `self_improvement_enabled` ← `features.selfImprovingLoop`
/// - `final_output_reflection` ← `features.finalOutputReflection`
///
/// 数据库无配置或解析失败时返回全 false 的默认值,保持向后兼容。
pub async fn read_self_improvement_flags(
    db: &axagent_harness::DatabaseConnection,
) -> axagent_agent::SelfImprovementFlags {
    let json_str = match axagent_dao::repo::settings::get_setting(db, "app_config").await {
        Ok(Some(s)) => s,
        _ => return axagent_agent::SelfImprovementFlags::default(),
    };
    let value: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return axagent_agent::SelfImprovementFlags::default(),
    };
    let features = value.get("features");
    axagent_agent::SelfImprovementFlags {
        self_improvement_enabled: features
            .and_then(|f| f.get("selfImprovingLoop"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        final_output_reflection: features
            .and_then(|f| f.get("finalOutputReflection"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    }
}

/// 缺陷1修复:Tauri 命令,前端切换 selfImprovingLoop / finalOutputReflection 后调用,
/// 即时更新 SessionManager 的 flags(无需重启应用)。
///
/// 前端 `appConfigStore.toggleFeature` 在 saveConfig 后调用本命令,
/// 把最新 flag 值推送到后端 SessionManager。
#[agent_command(domain = settings, safety = Caution, call_mode = StateInput, description = "设置自我改进标志")]
#[tauri::command]
pub async fn set_self_improvement_flags(
    state: State<'_, AppState>,
    self_improvement_enabled: bool,
    final_output_reflection: bool,
) -> Result<(), String> {
    let flags =
        axagent_agent::SelfImprovementFlags { self_improvement_enabled, final_output_reflection };
    state.agent_session_manager.set_self_improvement_flags(flags).await;
    Ok(())
}
