use tauri::command;

#[command]
#[allow(dead_code)]
pub async fn shell_execute(_command: String, _args: Vec<String>) -> Result<String, String> {
    tracing::warn!("shell_execute called but shell functionality is intentionally disabled");
    Err("Shell functionality is not available in this build".to_string())
}

#[command]
#[allow(dead_code)]
pub async fn shell_spawn(_command: String, _args: Vec<String>) -> Result<String, String> {
    tracing::warn!("shell_spawn called but shell functionality is intentionally disabled");
    Err("Shell functionality is not available in this build".to_string())
}

#[command]
#[allow(dead_code)]
pub async fn shell_terminate(_pid: u32) -> Result<(), String> {
    tracing::warn!("shell_terminate called but shell functionality is intentionally disabled");
    Err("Shell functionality is not available in this build".to_string())
}
