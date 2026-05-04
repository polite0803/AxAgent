//! 应用配置持久化命令
//!
//! 提供前端 appConfigStore 的后端持久化支持。

use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_app_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = &state.sea_db;
    match axagent_core::repo::settings::get_setting(db, "app_config").await {
        Ok(Some(json_str)) => {
            serde_json::from_str(&json_str).map_err(|e| format!("解析配置失败: {}", e))
        },
        Ok(None) => Ok(serde_json::json!({})),
        Err(e) => Err(format!("读取配置失败: {}", e)),
    }
}

#[tauri::command]
pub async fn save_app_config(
    state: State<'_, AppState>,
    config: serde_json::Value,
) -> Result<(), String> {
    let db = &state.sea_db;
    let json_str = serde_json::to_string(&config).map_err(|e| format!("序列化配置失败: {}", e))?;
    axagent_core::repo::settings::set_setting(db, "app_config", &json_str)
        .await
        .map_err(|e| format!("保存配置失败: {}", e))
}
