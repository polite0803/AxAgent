// SPDX-License-Identifier: AGPL-3.0-only

use axagent_core::cloud_storage::{
    BackendType, CloudStorageConfig, S3Config, S3ProviderPreset, WebDavConfig,
};
use axagent_core::sync_conflict::{ConflictResolution, ConflictStrategy};
use axagent_core::workspace_uri::WorkspaceUri;
use tauri::State;

use crate::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct CloudListRequest {
    pub workspace_uri: String,
    pub dir_path: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudListResponse {
    pub entries: Vec<CloudDirEntryDto>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct CloudDirEntryDto {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: i64,
    pub etag: Option<String>,
    pub conflict: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct CloudSyncRequest {
    pub workspace_uri: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudSyncResponse {
    pub downloaded: usize,
    pub uploaded: usize,
    pub local_deletions_synced: usize,
    pub remote_deletions_synced: usize,
    pub conflicts_detected: usize,
    pub conflicts_resolved: usize,
    pub pending_conflicts: usize,
    pub local_cache_dir: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudConflictDto {
    pub key: String,
    pub kind: String,
    pub resolution: Option<String>,
    pub local_size: i64,
    pub remote_size: i64,
    pub local_modified_at: u64,
    pub remote_modified_at: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudConflictsResponse {
    pub pending_conflicts: Vec<CloudConflictDto>,
    pub strategy: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ResolveConflictRequest {
    pub workspace_uri: String,
    pub key: String,
    pub resolution: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct SetConflictStrategyRequest {
    pub workspace_uri: String,
    pub strategy: String,
}

#[derive(Debug, serde::Serialize)]
pub struct CloudProviderPresetDto {
    pub key: String,
    pub display_name: String,
    pub endpoint_template: String,
    pub default_region: String,
    pub use_path_style: bool,
    pub category: String,
}

fn device_id() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown-device".to_string())
}

/// List S3 provider presets available for configuration.
#[tauri::command]
pub fn list_cloud_provider_presets() -> Vec<CloudProviderPresetDto> {
    let presets = S3ProviderPreset::all_presets();

    presets
        .into_iter()
        .map(|p| CloudProviderPresetDto {
            key: format!("{:?}", p),
            display_name: p.display_name().to_string(),
            endpoint_template: p.endpoint_template().to_string(),
            default_region: p.default_region().to_string(),
            use_path_style: p.default_use_path_style(),
            category: p.category().to_string(),
        })
        .collect()
}

fn build_cloud_workspace(
    state: &State<'_, AppState>,
    workspace_uri_str: &str,
) -> Result<(axagent_core::cloud_workspace::CloudWorkspace, String), String> {
    use axagent_core::cloud_workspace::CloudWorkspace;

    let workspace_uri = WorkspaceUri::parse(workspace_uri_str)
        .map_err(|e| format!("Invalid workspace URI: {}", e))?;

    if !workspace_uri.is_cloud() {
        return Err("Workspace URI is not a cloud URI".to_string());
    }

    let backend = state
        .sync_engine
        .as_ref()
        .ok_or("Cloud sync engine not available")?
        .backend
        .clone();

    let cache_base = dirs::cache_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".axagent")
        .join("cloud-cache");

    let raw_uri = workspace_uri.raw.clone();
    let workspace = CloudWorkspace::new(workspace_uri, backend, cache_base, device_id());
    Ok((workspace, raw_uri))
}

/// List directory contents on a cloud workspace.
#[tauri::command]
pub async fn list_cloud_directory(
    state: State<'_, AppState>,
    request: CloudListRequest,
) -> Result<CloudListResponse, String> {
    let (cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let entries = cloud_workspace
        .list_directory(&request.dir_path)
        .await
        .map_err(|e| format!("Failed to list cloud directory: {}", e))?;

    let entries_dto = entries
        .into_iter()
        .map(|e| CloudDirEntryDto {
            name: e.name,
            path: e.path,
            is_dir: e.is_dir,
            size: e.size,
            etag: e.etag,
            conflict: e.conflict,
        })
        .collect();

    Ok(CloudListResponse {
        entries: entries_dto,
    })
}

/// Sync a cloud workspace: bidirectional sync with conflict detection.
#[tauri::command]
pub async fn sync_cloud_workspace(
    state: State<'_, AppState>,
    request: CloudSyncRequest,
) -> Result<CloudSyncResponse, String> {
    let (mut cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let sync_result = cloud_workspace
        .sync()
        .await
        .map_err(|e| format!("Failed to sync cloud workspace: {}", e))?;

    // Trigger post-sync indexing
    let cache_dir = &sync_result.cached_dir;
    let indexing_report =
        crate::indexing_triggers::trigger_post_sync_indexing_for_cloud_workspace(cache_dir).await;

    if indexing_report.skipped {
        tracing::warn!("Post-sync indexing skipped: {:?}", indexing_report.reason);
    } else {
        tracing::info!(
            "Post-sync indexing complete: {} files, {} AST nodes",
            indexing_report.files_indexed,
            indexing_report.ast_nodes_indexed
        );
    }

    Ok(CloudSyncResponse {
        downloaded: sync_result.downloaded,
        uploaded: sync_result.uploaded,
        local_deletions_synced: sync_result.local_deletions_synced,
        remote_deletions_synced: sync_result.remote_deletions_synced,
        conflicts_detected: sync_result.conflicts_detected,
        conflicts_resolved: sync_result.conflicts_resolved,
        pending_conflicts: sync_result.pending_conflicts,
        local_cache_dir: sync_result.cached_dir.to_string_lossy().to_string(),
    })
}

/// Push local cache changes back to cloud with conflict detection.
#[tauri::command]
pub async fn push_cloud_workspace_changes(
    state: State<'_, AppState>,
    request: CloudSyncRequest,
) -> Result<CloudSyncResponse, String> {
    let (mut cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let sync_result = cloud_workspace
        .sync()
        .await
        .map_err(|e| format!("Failed to push changes to cloud: {}", e))?;

    // Trigger post-push indexing to update indexes with local changes
    let cache_dir = &sync_result.cached_dir;
    let indexing_report =
        crate::indexing_triggers::trigger_post_sync_indexing_for_cloud_workspace(cache_dir).await;

    if indexing_report.skipped {
        tracing::warn!("Post-push indexing skipped: {:?}", indexing_report.reason);
    } else {
        tracing::info!(
            "Post-push indexing complete: {} files, {} AST nodes",
            indexing_report.files_indexed,
            indexing_report.ast_nodes_indexed
        );
    }

    Ok(CloudSyncResponse {
        downloaded: sync_result.downloaded,
        uploaded: sync_result.uploaded,
        local_deletions_synced: sync_result.local_deletions_synced,
        remote_deletions_synced: sync_result.remote_deletions_synced,
        conflicts_detected: sync_result.conflicts_detected,
        conflicts_resolved: sync_result.conflicts_resolved,
        pending_conflicts: sync_result.pending_conflicts,
        local_cache_dir: sync_result.cached_dir.to_string_lossy().to_string(),
    })
}

/// Get pending conflicts for a workspace.
#[tauri::command]
pub async fn get_cloud_conflicts(
    state: State<'_, AppState>,
    request: CloudSyncRequest,
) -> Result<CloudConflictsResponse, String> {
    let (cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let conflicts = cloud_workspace.get_pending_conflicts();
    let strategy = cloud_workspace.sync_state().conflict_strategy;

    let pending = conflicts
        .into_iter()
        .map(|(key, info)| CloudConflictDto {
            key: key.to_string(),
            kind: format!("{:?}", info.kind),
            resolution: info.resolution.map(|r| format!("{:?}", r)),
            local_size: info.local_version.size,
            remote_size: info.remote_version.size,
            local_modified_at: info.local_version.modified_at,
            remote_modified_at: info.remote_version.modified_at,
        })
        .collect();

    Ok(CloudConflictsResponse {
        pending_conflicts: pending,
        strategy: format!("{:?}", strategy),
    })
}

/// Resolve a specific conflict.
#[tauri::command]
pub async fn resolve_cloud_conflict(
    state: State<'_, AppState>,
    request: ResolveConflictRequest,
) -> Result<(), String> {
    let (mut cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let resolution = match request.resolution.as_str() {
        "keep_local" => ConflictResolution::KeepLocal,
        "keep_remote" => ConflictResolution::KeepRemote,
        "keep_both" => ConflictResolution::KeepBoth,
        "keep_newer" => ConflictResolution::KeepNewer,
        _ => return Err(format!("Unknown conflict resolution: {}", request.resolution)),
    };

    cloud_workspace
        .resolve_conflict(&request.key, resolution)
        .map_err(|e| format!("Failed to resolve conflict: {}", e))
}

/// Set the conflict resolution strategy for a workspace.
#[tauri::command]
pub async fn set_cloud_conflict_strategy(
    state: State<'_, AppState>,
    request: SetConflictStrategyRequest,
) -> Result<(), String> {
    let (mut cloud_workspace, _uri) = build_cloud_workspace(&state, &request.workspace_uri)?;

    let strategy = match request.strategy.as_str() {
        "latest_wins" => ConflictStrategy::LatestWins,
        "local_wins" => ConflictStrategy::LocalWins,
        "remote_wins" => ConflictStrategy::RemoteWins,
        "manual" => ConflictStrategy::Manual,
        _ => return Err(format!("Unknown conflict strategy: {}", request.strategy)),
    };

    cloud_workspace.set_conflict_strategy(strategy);
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
pub struct CheckCloudConnectionRequest {
    pub storage_type: String,
    pub endpoint: Option<String>,
    pub region: Option<String>,
    pub bucket: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub root: Option<String>,
    pub use_path_style: Option<bool>,
    pub host: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub path: Option<String>,
}

#[tauri::command]
pub async fn check_cloud_connection(config: CheckCloudConnectionRequest) -> Result<bool, String> {
    let backend_type = match config.storage_type.as_str() {
        "s3" => BackendType::S3,
        "webdav" => BackendType::WebDav,
        other => return Err(format!("Unknown storage type: {}", other)),
    };

    let cloud_config = CloudStorageConfig {
        provider_preset: S3ProviderPreset::Custom,
        backend_type,
        sync_enabled: true,
        sync_mode: axagent_core::cloud_storage::SyncMode::Sync,
        profile_name: "test".to_string(),
        s3: if backend_type == BackendType::S3 {
            Some(S3Config {
                endpoint: config.endpoint.unwrap_or_default(),
                region: config.region.unwrap_or_else(|| "auto".to_string()),
                bucket: config.bucket.unwrap_or_default(),
                access_key_id: config.access_key_id.unwrap_or_default(),
                secret_access_key: config.secret_access_key.unwrap_or_default(),
                root: config.root.unwrap_or_default(),
                use_path_style: config.use_path_style.unwrap_or(false),
            })
        } else {
            None
        },
        webdav: if backend_type == BackendType::WebDav {
            Some(WebDavConfig {
                host: config.host.unwrap_or_default(),
                username: config.username.unwrap_or_default(),
                password: config.password.unwrap_or_default(),
                path: config.path.unwrap_or_else(|| "/".to_string()),
                accept_invalid_certs: false,
            })
        } else {
            None
        },
    };

    let backend = cloud_config
        .create_backend()
        .map_err(|e| format!("Failed to create backend: {}", e))?;

    backend
        .check_connection()
        .await
        .map_err(|e| format!("Connection check failed: {}", e))
}
