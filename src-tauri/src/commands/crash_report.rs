#[tauri::command]
pub fn get_crash_log() -> Result<Option<String>, String> {
    Ok(crate::android_utils::consume_crash_log())
}
