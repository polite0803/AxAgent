use axagent_core::sandbox_runner::{self, ExecutionResult};
use tauri::command;

#[command]
pub async fn execute_sandbox(code: String, language: String) -> Result<ExecutionResult, String> {
    let runner = sandbox_runner::create_sandbox_runner();
    runner
        .execute(&code, &language)
        .await
        .map_err(|e| e.to_string())
}
