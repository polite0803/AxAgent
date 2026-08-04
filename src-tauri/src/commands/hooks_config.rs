// SPDX-License-Identifier: AGPL-3.0-only

//! HooksTab 钩子配置持久化命令（P1-5）
//!
//! 复用 `axagent_dao::repo::settings` 的通用 KV 存储（key = "hooks_config"），
//! 避免新建数据库表。前端 HooksTab 在挂载时调用 `get_hooks_config` 加载，
//! 任意修改（toggle / addCommand / removeCommand）后调用 `save_hooks_config` 持久化。

use crate::AppState;
use crate::commands::error::ErrorResponse;
use crate::commands::error_code::storage as storage_err;
use agent_macro::agent_command;
use serde::{Deserialize, Serialize};
use tauri::State;

const HOOKS_CONFIG_KEY: &str = "hooks_config";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCommandDto {
    pub id: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookStateDto {
    pub event: String,
    pub enabled: bool,
    pub commands: Vec<HookCommandDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HooksConfigDto {
    pub hooks: Vec<HookStateDto>,
}

#[agent_command(domain = hooks, safety = Safe, call_mode = StateOnly, description = "获取钩子配置")]
#[tauri::command]
pub async fn get_hooks_config(state: State<'_, AppState>) -> Result<HooksConfigDto, String> {
    let db = state.harness.db();
    match axagent_dao::repo::settings::get_setting(db, HOOKS_CONFIG_KEY).await {
        Ok(Some(json_str)) => {
            let cfg: HooksConfigDto = serde_json::from_str(&json_str)
                .map_err(|e| format!("解析 hooks_config 失败: {}", e))?;
            Ok(cfg)
        },
        Ok(None) => Ok(HooksConfigDto::default()),
        Err(e) => Err(ErrorResponse::new(storage_err::READ_FILE_FAILED)
            .with_detail(format!("读取 hooks_config 失败: {}", e))
            .into()),
    }
}

#[agent_command(domain = hooks, safety = Caution, call_mode = StateInput, description = "保存钩子配置")]
#[tauri::command]
pub async fn save_hooks_config(
    state: State<'_, AppState>,
    config: HooksConfigDto,
) -> Result<(), String> {
    let db = state.harness.db();
    let json_str =
        serde_json::to_string(&config).map_err(|e| format!("序列化 hooks_config 失败: {}", e))?;
    axagent_dao::repo::settings::set_setting(db, HOOKS_CONFIG_KEY, &json_str)
        .await
        .map_err(|e| format!("保存 hooks_config 失败: {}", e))
}
