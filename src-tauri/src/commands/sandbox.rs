// SPDX-License-Identifier: AGPL-3.0-only

use axagent_kit::sandbox_runner::{self, ExecutionResult};
use agent_macro::agent_command;
use tauri::command;

#[agent_command(domain = sandbox, safety = Caution, call_mode = Manual, description = "在沙箱中执行代码")]
#[command]
pub async fn execute_sandbox(code: String, language: String) -> Result<ExecutionResult, String> {
    let runner = sandbox_runner::create_sandbox_runner();
    runner.execute(&code, &language).await.map_err(|e| {
        String::from(crate::commands::error::ErrorResponse::from_error(
            e,
            crate::commands::error::ErrorCategory::Unrecoverable,
        ))
    })
}
