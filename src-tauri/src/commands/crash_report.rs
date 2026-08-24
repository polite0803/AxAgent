// SPDX-License-Identifier: AGPL-3.0-only

use axagent_agent_macro::agent_command;

#[agent_command(domain = "system", safety = Safe, call_mode = Manual, description = "获取崩溃日志")]
#[tauri::command]
pub fn get_crash_log() -> Result<Option<String>, String> {
    Ok(crate::android_utils::consume_crash_log())
}
