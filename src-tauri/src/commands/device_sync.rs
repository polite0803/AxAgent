// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步相关 Tauri 命令
//!
//! 提供设备管理、配对、同步、加密等 IPC 接口。

use std::sync::Arc;

use agent_macro::agent_command;

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use axagent_device::conflict_resolver::ConflictResolver;
use axagent_device::encryption::{EncryptedSyncData, SyncEncryptor};
use axagent_device::error_codes::{ErrorCategory as SyncErrorCategory, SyncErrorCode};
use axagent_device::history_store::HistoryStore;
use axagent_device::manager::{DeviceManagerImpl, DeviceStore};
use axagent_device::permission_store::PermissionStore;
use axagent_device::policy_store::PolicyStore;
use axagent_device::sync_engine::{ChangeLogStore, SyncEngineImpl};
use axagent_harness::device_sync::{
    AuditLogEntry, ChangeLogEntry, ChangeOperation, ConflictInfo, ConflictResolutionStrategy,
    DeviceInfo, DeviceManager, DevicePermissions, DeviceSyncStatus, EntityType, PairingCode,
    PairingRequest, PairingResponse, PermissionUpdate, SyncEngine, SyncHistoryEntry, SyncPolicy,
    SyncPolicyUpdate, SyncResult, SyncStorage, TrustLevel,
};
use tauri::State;

/// 将 SyncErrorCode 转换为 ErrorResponse
fn to_error_response(code: SyncErrorCode) -> ErrorResponse {
    let code_str = code.as_str();
    let category = match code.category() {
        SyncErrorCategory::Device => ErrorCategory::Unrecoverable,
        SyncErrorCategory::Permission => ErrorCategory::PermissionDenied,
        SyncErrorCategory::Sync => ErrorCategory::Retryable,
        SyncErrorCategory::Encryption => ErrorCategory::Unrecoverable,
        SyncErrorCategory::Transport => ErrorCategory::Retryable,
        SyncErrorCategory::Crdt => ErrorCategory::Unrecoverable,
        SyncErrorCategory::Scheduler => ErrorCategory::Retryable,
        SyncErrorCategory::Storage => ErrorCategory::Unrecoverable,
    };

    ErrorResponse::new(code_str).with_category(category).with_detail(code.default_message())
}

/// 将错误信息转换为 ErrorResponse
fn sync_err(code: SyncErrorCode, e: impl std::fmt::Display) -> ErrorResponse {
    let mut response = to_error_response(code);
    response.detail = Some(e.to_string());
    response
}

/// 创建验证错误
fn validation_err(code: SyncErrorCode, msg: impl Into<String>) -> ErrorResponse {
    ErrorResponse::new(code.as_str()).with_category(ErrorCategory::Validation).with_detail(msg)
}

/// 设备同步状态（AppState 中持有）
pub struct DeviceSyncState {
    pub device_store: Arc<DeviceStore>,
    pub change_log: Arc<ChangeLogStore>,
    pub device_manager: Arc<DeviceManagerImpl>,
    pub sync_engine: Arc<SyncEngineImpl>,
    pub policy_store: Arc<PolicyStore>,
    pub history_store: Arc<HistoryStore>,
    pub permission_store: Arc<PermissionStore>,
    pub local_device_id: String,
}

impl DeviceSyncState {
    /// 创建设备同步状态（内存模式，用于测试）
    pub fn new(local_device_id: String) -> Self {
        let device_store = Arc::new(DeviceStore::new());
        Self::create_with_store(device_store, local_device_id)
    }

    /// 创建设备同步状态（数据库模式）
    pub async fn with_storage(local_device_id: String, sync_storage: Arc<dyn SyncStorage>) -> Self {
        let device_store = Arc::new(DeviceStore::with_storage(sync_storage.clone()));
        let change_log = Arc::new(ChangeLogStore::with_storage(sync_storage.clone()));
        let history_store = Arc::new(HistoryStore::with_storage(sync_storage.clone()));
        let policy_store = Arc::new(PolicyStore::with_storage(sync_storage.clone()));
        let permission_store = Arc::new(PermissionStore::with_storage(sync_storage));

        let device_manager = Arc::new(DeviceManagerImpl::new(device_store.clone()));
        let sync_engine = Arc::new(SyncEngineImpl::new(
            change_log.clone(),
            device_store.clone(),
            history_store.clone(),
            local_device_id.clone(),
        ));

        // 从数据库加载已有数据到缓存
        if let Ok(()) = device_store.load_devices_from_db().await {
            tracing::info!("已从数据库加载设备列表");
        }
        if let Ok(()) = change_log.load_from_db().await {
            tracing::info!("已从数据库加载变更日志");
        }

        Self {
            device_store,
            change_log,
            device_manager,
            sync_engine,
            policy_store,
            history_store,
            permission_store,
            local_device_id,
        }
    }

    fn create_with_store(device_store: Arc<DeviceStore>, local_device_id: String) -> Self {
        let change_log = Arc::new(ChangeLogStore::new());
        let device_manager = Arc::new(DeviceManagerImpl::new(device_store.clone()));
        let history_store = Arc::new(HistoryStore::new());
        let sync_engine = Arc::new(SyncEngineImpl::new(
            change_log.clone(),
            device_store.clone(),
            history_store.clone(),
            local_device_id.clone(),
        ));
        let policy_store = Arc::new(PolicyStore::new());
        let permission_store = Arc::new(PermissionStore::new());

        Self {
            device_store,
            change_log,
            device_manager,
            sync_engine,
            policy_store,
            history_store,
            permission_store,
            local_device_id,
        }
    }
}

/// 检查设备是否有权执行指定操作
async fn check_device_permission(
    sync_state: &DeviceSyncState,
    device_id: &str,
    operation: &str,
) -> Result<(), ErrorResponse> {
    let has_permission = match operation {
        "push" => sync_state.permission_store.can_push(device_id).await,
        "pull" => sync_state.permission_store.can_pull(device_id).await,
        "full_sync" => sync_state.permission_store.can_full_sync(device_id).await,
        "resolve_conflicts" => sync_state.permission_store.can_resolve_conflicts(device_id).await,
        "manage_devices" => sync_state.permission_store.can_manage_devices(device_id).await,
        "modify_policy" => sync_state.permission_store.can_modify_policy(device_id).await,
        _ => true,
    };

    if !has_permission {
        let mut response = to_error_response(SyncErrorCode::PermissionDenied);
        response = response.with_param("operation", operation);
        return Err(response);
    }

    Ok(())
}

// ─── 设备管理命令 ───────────────────────────────────────────────────────────

/// 注册当前设备
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "注册当前设备")]
#[tauri::command]
pub async fn register_device(
    state: State<'_, AppState>,
    name: String,
    hostname: String,
    os: String,
    app_version: String,
) -> Result<DeviceInfo, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    let device = DeviceManagerImpl::create_local_device(name, hostname, os, app_version);
    sync_state
        .device_manager
        .register_device(device)
        .await
        .map_err(|e| sync_err(SyncErrorCode::DeviceNotFound, e))
}

/// 获取本地设备信息
#[agent_command(domain = "device", safety = Safe, call_mode = StateOnly, description = "获取本地设备信息")]
#[tauri::command]
pub async fn get_local_device(state: State<'_, AppState>) -> Result<DeviceInfo, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(DeviceInfo {
        device_id: sync_state.local_device_id.clone(),
        name: "This Device".to_string(),
        hostname: "localhost".to_string(),
        os: std::env::consts::OS.to_string(),
        device_type: axagent_harness::device_sync::DeviceType::Desktop,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        registered_at: chrono::Utc::now().to_rfc3339(),
        last_active_at: chrono::Utc::now().to_rfc3339(),
        is_paired: true,
        trust_level: TrustLevel::Full,
    })
}

/// 列出所有设备
#[agent_command(domain = "device", safety = Safe, call_mode = StateOnly, description = "列出所有设备")]
#[tauri::command]
pub async fn list_devices(state: State<'_, AppState>) -> Result<Vec<DeviceInfo>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .list_devices()
        .await
        .map_err(|e| sync_err(SyncErrorCode::DeviceNotFound, e))
}

/// 生成配对码
#[agent_command(domain = "device", safety = Caution, call_mode = StateOnly, description = "生成设备配对码")]
#[tauri::command]
pub async fn generate_pairing_code(
    state: State<'_, AppState>,
) -> Result<PairingCode, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .generate_pairing_code()
        .await
        .map_err(|e| sync_err(SyncErrorCode::SyncFailed, e))
}

/// 验证配对码并返回配对请求
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "验证设备配对码")]
#[tauri::command]
pub async fn verify_pairing_code(
    state: State<'_, AppState>,
    code: String,
) -> Result<PairingRequest, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .verify_pairing_code(&code)
        .await
        .map_err(|e| sync_err(SyncErrorCode::InvalidPairingCode, e))
}

/// 接受设备配对请求
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "接受设备配对请求")]
#[tauri::command]
pub async fn accept_pairing(
    state: State<'_, AppState>,
    request: PairingRequest,
    trust_level: String,
) -> Result<PairingResponse, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    let level = match trust_level.as_str() {
        "backup_only" => TrustLevel::BackupOnly,
        "full" => TrustLevel::Full,
        _ => TrustLevel::Standard,
    };

    sync_state
        .device_manager
        .accept_pairing(request, level)
        .await
        .map_err(|e| sync_err(SyncErrorCode::DeviceAlreadyPaired, e))
}

/// 撤销设备配对
#[agent_command(domain = "device", safety = Dangerous, call_mode = StateInput, description = "撤销设备配对")]
#[tauri::command]
pub async fn unpair_device(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<(), ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .unpair_device(&device_id)
        .await
        .map_err(|e| sync_err(SyncErrorCode::DeviceNotFound, e))
}

// ─── 同步命令 ───────────────────────────────────────────────────────────────

/// 执行全量同步
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "执行全量设备同步")]
#[tauri::command]
pub async fn full_sync(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;

    check_device_permission(&sync_state, &device_id, "full_sync").await?;

    sync_state
        .sync_engine
        .full_sync(&device_id)
        .await
        .map_err(|e| sync_err(SyncErrorCode::SyncFailed, e))
}

/// 执行增量同步
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "执行增量设备同步")]
#[tauri::command]
pub async fn incremental_sync(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;

    check_device_permission(&sync_state, &device_id, "pull").await?;

    sync_state
        .sync_engine
        .incremental_sync(&device_id)
        .await
        .map_err(|e| sync_err(SyncErrorCode::SyncFailed, e))
}

/// 推送变更日志
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "推送设备变更日志")]
#[tauri::command]
pub async fn push_changes(
    state: State<'_, AppState>,
    device_id: String,
    changes: Vec<ChangeLogEntry>,
) -> Result<Vec<ConflictInfo>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;

    check_device_permission(&sync_state, &device_id, "push").await?;

    sync_state
        .sync_engine
        .push_changes(changes)
        .await
        .map_err(|e| sync_err(SyncErrorCode::SyncFailed, e))
}

/// 拉取变更日志
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "拉取设备变更日志")]
#[tauri::command]
pub async fn pull_changes(
    state: State<'_, AppState>,
    device_id: String,
    since_timestamp: u64,
) -> Result<Vec<ChangeLogEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;

    check_device_permission(&sync_state, &device_id, "pull").await?;

    sync_state
        .sync_engine
        .pull_changes(since_timestamp)
        .await
        .map_err(|e| sync_err(SyncErrorCode::SyncFailed, e))
}

/// 解决冲突
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "解决设备同步冲突")]
#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, AppState>,
    device_id: String,
    conflict_id: String,
    strategy: String,
) -> Result<(), ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;

    check_device_permission(&sync_state, &device_id, "resolve_conflicts").await?;

    let strategy = match strategy.as_str() {
        "keep_local" => ConflictResolutionStrategy::KeepLocal,
        "keep_remote" => ConflictResolutionStrategy::KeepRemote,
        "keep_both" => ConflictResolutionStrategy::KeepBoth,
        "last_write_wins" => ConflictResolutionStrategy::LastWriteWins,
        _ => ConflictResolutionStrategy::LastWriteWins,
    };

    let changes = sync_state.change_log.get_all_entries().await;
    let conflict =
        ConflictResolver::find_conflict_by_id(&changes, &conflict_id).ok_or_else(|| {
            let mut params = std::collections::HashMap::new();
            params.insert("conflictId".to_string(), conflict_id.clone());
            ErrorResponse::new(SyncErrorCode::ConflictNotFound.as_str())
                .with_category(ErrorCategory::Validation)
                .with_params(params)
        })?;

    sync_state
        .sync_engine
        .resolve_conflict(&conflict, strategy)
        .await
        .map_err(|e| sync_err(SyncErrorCode::ConflictResolutionFailed, e))
}

/// 获取设备同步状态
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "获取设备同步状态")]
#[tauri::command]
pub async fn get_sync_status(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<DeviceSyncStatus, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .sync_engine
        .get_sync_status(&device_id)
        .await
        .map_err(|e| sync_err(SyncErrorCode::DeviceNotFound, e))
}

/// 记录本地变更
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "记录本地变更")]
#[tauri::command]
pub async fn record_change(
    state: State<'_, AppState>,
    entity_type: String,
    entity_id: String,
    operation: String,
    data: Option<String>,
) -> Result<ChangeLogEntry, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    let entity_type = match entity_type.as_str() {
        "conversation" => EntityType::Conversation,
        "message" => EntityType::Message,
        "setting" => EntityType::Setting,
        "file" => EntityType::File,
        "wiki" => EntityType::Wiki,
        "knowledge" => EntityType::Knowledge,
        "agent" => EntityType::Agent,
        "workflow" => EntityType::Workflow,
        _ => EntityType::Conversation,
    };

    let operation = match operation.as_str() {
        "create" => ChangeOperation::Create,
        "delete" => ChangeOperation::Delete,
        _ => ChangeOperation::Update,
    };

    Ok(sync_state.sync_engine.record_change(entity_type, &entity_id, operation, data).await)
}

// ─── 加密命令 ───────────────────────────────────────────────────────────────

/// 加密同步数据
#[agent_command(domain = "device", safety = Caution, call_mode = Manual, description = "加密设备同步数据")]
#[tauri::command]
pub async fn encrypt_sync_data(
    data: String,
    password: Option<String>,
    salt: Option<String>,
) -> Result<EncryptedSyncData, ErrorResponse> {
    let pwd = password
        .ok_or_else(|| validation_err(SyncErrorCode::PasswordRequired, "加密必须提供密码"))?;

    if pwd.is_empty() {
        return Err(validation_err(SyncErrorCode::PasswordEmpty, "加密密码不能为空"));
    }

    let salt_bytes = match salt {
        Some(s) => {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            STANDARD.decode(&s).unwrap_or_else(|_| s.as_bytes().to_vec())
        },
        None => SyncEncryptor::generate_salt().to_vec(),
    };
    let encryptor = SyncEncryptor::from_password(&pwd, &salt_bytes);

    encryptor.encrypt(&data).map_err(|e| sync_err(SyncErrorCode::EncryptionFailed, e))
}

/// 解密同步数据
#[agent_command(domain = "device", safety = Caution, call_mode = Manual, description = "解密设备同步数据")]
#[tauri::command]
pub async fn decrypt_sync_data(
    data: EncryptedSyncData,
    password: Option<String>,
    salt: Option<String>,
) -> Result<String, ErrorResponse> {
    let pwd = password
        .ok_or_else(|| validation_err(SyncErrorCode::PasswordRequired, "解密必须提供密码"))?;

    if pwd.is_empty() {
        return Err(validation_err(SyncErrorCode::PasswordEmpty, "解密密码不能为空"));
    }

    let salt_bytes = match salt {
        Some(s) => {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            STANDARD.decode(&s).unwrap_or_else(|_| s.as_bytes().to_vec())
        },
        None => {
            return Err(validation_err(SyncErrorCode::SaltRequired, "解密需要提供盐值"));
        },
    };
    let encryptor = SyncEncryptor::from_password(&pwd, &salt_bytes);

    encryptor.decrypt(&data).map_err(|e| sync_err(SyncErrorCode::DecryptionFailed, e))
}

// ─── P2: 同步策略命令 ───────────────────────────────────────────────────────

/// 获取当前活动策略
#[agent_command(domain = "device", safety = Safe, call_mode = StateOnly, description = "获取当前同步策略")]
#[tauri::command]
pub async fn get_sync_policy(
    state: State<'_, AppState>,
) -> Result<Option<SyncPolicy>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.get_active_policy().await)
}

/// 列出所有策略
#[agent_command(domain = "device", safety = Safe, call_mode = StateOnly, description = "列出所有同步策略")]
#[tauri::command]
pub async fn list_sync_policies(
    state: State<'_, AppState>,
) -> Result<Vec<SyncPolicy>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.list_policies().await)
}

/// 创建策略
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "创建同步策略")]
#[tauri::command]
pub async fn create_sync_policy(
    state: State<'_, AppState>,
    policy: SyncPolicy,
) -> Result<SyncPolicy, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.create_policy(policy).await)
}

/// 更新策略
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "更新同步策略")]
#[tauri::command]
pub async fn update_sync_policy(
    state: State<'_, AppState>,
    update: SyncPolicyUpdate,
) -> Result<SyncPolicy, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    if let Some(active) = sync_state.policy_store.get_active_policy().await {
        sync_state
            .policy_store
            .update_policy(&active.id, update)
            .await
            .map_err(|e| sync_err(SyncErrorCode::PolicyOperationFailed, e))
    } else {
        let default = PolicyStore::default_policy();
        let created = sync_state.policy_store.create_policy(default).await;
        sync_state
            .policy_store
            .set_active_policy(&created.id)
            .await
            .map_err(|e| sync_err(SyncErrorCode::PolicyOperationFailed, e))?;
        sync_state
            .policy_store
            .update_policy(&created.id, update)
            .await
            .map_err(|e| sync_err(SyncErrorCode::PolicyOperationFailed, e))
    }
}

/// 删除策略
#[agent_command(domain = "device", safety = Dangerous, call_mode = StateInput, description = "删除同步策略")]
#[tauri::command]
pub async fn delete_sync_policy(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .policy_store
        .delete_policy(&id)
        .await
        .map_err(|e| sync_err(SyncErrorCode::PolicyOperationFailed, e))
}

// ─── P2: 同步历史记录命令 ───────────────────────────────────────────────────

/// 获取同步历史记录
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "获取同步历史记录")]
#[tauri::command]
pub async fn get_sync_history(
    state: State<'_, AppState>,
    limit: u64,
) -> Result<Vec<SyncHistoryEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.history_store.get_history(limit as usize).await)
}

/// 获取审计日志
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "获取审计日志")]
#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, AppState>,
    limit: u64,
) -> Result<Vec<AuditLogEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.history_store.get_audit_logs(limit as usize).await)
}

// ─── P2: 设备权限命令 ───────────────────────────────────────────────────────

/// 获取设备权限
#[agent_command(domain = "device", safety = Safe, call_mode = StateInput, description = "获取设备权限")]
#[tauri::command]
pub async fn get_device_permissions(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<DevicePermissions, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state.permission_store.get_permissions(&device_id).await.ok_or_else(|| {
        let mut params = std::collections::HashMap::new();
        params.insert("deviceId".to_string(), device_id);
        ErrorResponse::new(SyncErrorCode::PermissionsNotFound.as_str())
            .with_category(ErrorCategory::Validation)
            .with_params(params)
    })
}

/// 更新设备权限
#[agent_command(domain = "device", safety = Caution, call_mode = StateInput, description = "更新设备权限")]
#[tauri::command]
pub async fn update_device_permissions(
    state: State<'_, AppState>,
    device_id: String,
    update: PermissionUpdate,
) -> Result<DevicePermissions, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .permission_store
        .update_permissions(&device_id, update)
        .await
        .map_err(|e| sync_err(SyncErrorCode::PermissionDenied, e))
}

/// 列出所有设备权限
#[agent_command(domain = "device", safety = Safe, call_mode = StateOnly, description = "列出所有设备权限")]
#[tauri::command]
pub async fn list_all_permissions(
    state: State<'_, AppState>,
) -> Result<Vec<DevicePermissions>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.permission_store.get_all_permissions().await)
}
