use axagent_core::model_downloader::{LocalModelInfo, ModelDownloader, PresetModel};

#[tauri::command]
pub async fn list_local_models() -> Result<Vec<LocalModelInfo>, String> {
    let dl = ModelDownloader::new();
    Ok(dl.list_all_models())
}

#[tauri::command]
pub async fn download_model(filename: String) -> Result<(), String> {
    let dl = ModelDownloader::new();
    let presets = ModelDownloader::preset_models();
    let preset = presets
        .iter()
        .find(|p| p.filename == filename)
        .ok_or_else(|| format!("Unknown model: {}", filename))?;
    dl.ensure_model(preset)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_model(filename: String) -> Result<(), String> {
    let dl = ModelDownloader::new();
    dl.remove_model(&filename).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_preset_models() -> Result<Vec<PresetModel>, String> {
    Ok(ModelDownloader::preset_models())
}
