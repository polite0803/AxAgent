// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步相关 Tauri 命令
//!
//! 提供设备管理、配对、同步、加密等 IPC 接口。

use std::sync::Arc;

use crate::AppState;
use crate::commands::error::{ErrorCategory, ErrorResponse};
use axagent_device::conflict_resolver::ConflictResolver;
use axagent_device::encryption::{EncryptedSyncData, SyncEncryptor};
use axagent_device::history_store::HistoryStore;
use axagent_device::manager::{DeviceManagerImpl, DeviceStore};
use axagent_device::permission_store::PermissionStore;
use axagent_device::policy_store::PolicyStore;
use axagent_device::sync_engine::{ChangeLogStore, SyncEngineImpl};
use axagent_harness::device_sync::{
    AuditLogEntry, ChangeLogEntry, ChangeOperation, ConflictInfo,
    ConflictResolutionStrategy, DeviceInfo, DeviceManager, DevicePermissions,
    DeviceSyncStatus, EntityType, PairingCode, PairingRequest, PairingResponse,
    PermissionUpdate, SyncEngine, SyncHistoryEntry, SyncPolicy, SyncPolicyUpdate,
    SyncResult, SyncStorage, TrustLevel,
};
use tauri::State;

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
    pub fn with_storage(
        local_device_id: String,
        sync_storage: Arc<dyn SyncStorage>,
    ) -> Self {
        let device_store = Arc::new(DeviceStore::with_storage(sync_storage));
        Self::create_with_store(device_store, local_device_id)
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

fn internal_err(e: impl std::fmt::Display) -> ErrorResponse {
    ErrorResponse::from_error(e, ErrorCategory::Unrecoverable)
}

// ─── 设备管理命令 ───────────────────────────────────────────────────────────

/// 注册当前设备
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
        .map_err(internal_err)
}

/// 获取本地设备信息
#[tauri::command]
pub async fn get_local_device(
    state: State<'_, AppState>,
) -> Result<DeviceInfo, ErrorResponse> {
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
#[tauri::command]
pub async fn list_devices(
    state: State<'_, AppState>,
) -> Result<Vec<DeviceInfo>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .list_devices()
        .await
        .map_err(internal_err)
}

/// 生成配对码
#[tauri::command]
pub async fn generate_pairing_code(
    state: State<'_, AppState>,
) -> Result<PairingCode, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .device_manager
        .generate_pairing_code()
        .await
        .map_err(internal_err)
}

/// 验证配对码并返回配对请求
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
        .map_err(internal_err)
}

/// 接受设备配对请求
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
        .map_err(internal_err)
}

/// 撤销设备配对
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
        .map_err(internal_err)
}

// ─── 同步命令 ───────────────────────────────────────────────────────────────

/// 执行全量同步
#[tauri::command]
pub async fn full_sync(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .sync_engine
        .full_sync(&device_id)
        .await
        .map_err(internal_err)
}

/// 执行增量同步
#[tauri::command]
pub async fn incremental_sync(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<SyncResult, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .sync_engine
        .incremental_sync(&device_id)
        .await
        .map_err(internal_err)
}

/// 推送变更日志
#[tauri::command]
pub async fn push_changes(
    state: State<'_, AppState>,
    changes: Vec<ChangeLogEntry>,
) -> Result<Vec<ConflictInfo>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .sync_engine
        .push_changes(changes)
        .await
        .map_err(internal_err)
}

/// 拉取变更日志
#[tauri::command]
pub async fn pull_changes(
    state: State<'_, AppState>,
    since_timestamp: u64,
) -> Result<Vec<ChangeLogEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .sync_engine
        .pull_changes(since_timestamp)
        .await
        .map_err(internal_err)
}

/// 解决冲突
#[tauri::command]
pub async fn resolve_conflict(
    state: State<'_, AppState>,
    conflict_id: String,
    strategy: String,
) -> Result<(), ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    let strategy = match strategy.as_str() {
        "keep_local" => ConflictResolutionStrategy::KeepLocal,
        "keep_remote" => ConflictResolutionStrategy::KeepRemote,
        "keep_both" => ConflictResolutionStrategy::KeepBoth,
        "last_write_wins" => ConflictResolutionStrategy::LastWriteWins,
        _ => ConflictResolutionStrategy::LastWriteWins,
    };

    // 从变更日志中查找冲突记录
    let changes = sync_state.change_log.get_all_entries().await;
    let conflict = ConflictResolver::find_conflict_by_id(&changes, &conflict_id)
        .ok_or_else(|| ErrorResponse::from_error("冲突记录不存在", ErrorCategory::General))?;

    sync_state
        .sync_engine
        .resolve_conflict(&conflict, strategy)
        .await
        .map_err(internal_err)
}

/// 获取设备同步状态
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
        .map_err(internal_err)
}

/// 记录本地变更
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

    Ok(sync_state
        .sync_engine
        .record_change(entity_type, &entity_id, operation, data)
        .await)
}

// ─── 加密命令 ───────────────────────────────────────────────────────────────

/// 加密同步数据
#[tauri::command]
pub async fn encrypt_sync_data(
    data: String,
    password: Option<String>,
    salt: Option<String>,
) -> Result<EncryptedSyncData, ErrorResponse> {
    let pwd = password.unwrap_or_else(|| "axagent-default-encryption-password".to_string());
    let salt_bytes = match salt {
        Some(s) => {
            // 尝试将 salt 字符串解码为 base64，如果失败则使用其字节
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            STANDARD.decode(&s).unwrap_or_else(|_| s.as_bytes().to_vec())
        },
        None => SyncEncryptor::generate_salt().to_vec(),
    };
    let encryptor = SyncEncryptor::from_password(&pwd, &salt_bytes);
    
    encryptor
        .encrypt(&data)
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

/// 解密同步数据
#[tauri::command]
pub async fn decrypt_sync_data(
    data: EncryptedSyncData,
    password: Option<String>,
    salt: Option<String>,
) -> Result<String, ErrorResponse> {
    let pwd = password.unwrap_or_else(|| "axagent-default-encryption-password".to_string());
    let salt_bytes = match salt {
        Some(s) => {
            use base64::{Engine as _, engine::general_purpose::STANDARD};
            STANDARD.decode(&s).unwrap_or_else(|_| s.as_bytes().to_vec())
        },
        None => {
            return Err(ErrorResponse::from_error(
                "解密需要提供盐值".to_string(),
                ErrorCategory::General,
            ));
        },
    };
    let encryptor = SyncEncryptor::from_password(&pwd, &salt_bytes);
    
    encryptor
        .decrypt(&data)
        .map_err(|e| ErrorResponse::from_error(e, ErrorCategory::General))
}

// ─── P2: 同步策略命令 ───────────────────────────────────────────────────────

/// 获取当前活动策略
#[tauri::command]
pub async fn get_sync_policy(
    state: State<'_, AppState>,
) -> Result<Option<SyncPolicy>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.get_active_policy().await)
}

/// 列出所有策略
#[tauri::command]
pub async fn list_sync_policies(
    state: State<'_, AppState>,
) -> Result<Vec<SyncPolicy>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.list_policies().await)
}

/// 创建策略
#[tauri::command]
pub async fn create_sync_policy(
    state: State<'_, AppState>,
    policy: SyncPolicy,
) -> Result<SyncPolicy, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.policy_store.create_policy(policy).await)
}

/// 更新策略
#[tauri::command]
pub async fn update_sync_policy(
    state: State<'_, AppState>,
    update: SyncPolicyUpdate,
) -> Result<SyncPolicy, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    // 如果有活动策略，更新活动策略；否则创建新的
    if let Some(active) = sync_state.policy_store.get_active_policy().await {
        sync_state
            .policy_store
            .update_policy(&active.id, update)
            .await
            .map_err(internal_err)
    } else {
        // 创建新的默认策略并设置为活动
        let default = PolicyStore::default_policy();
        let created = sync_state.policy_store.create_policy(default).await;
        sync_state
            .policy_store
            .set_active_policy(&created.id)
            .await
            .map_err(internal_err)?;
        sync_state
            .policy_store
            .update_policy(&created.id, update)
            .await
            .map_err(internal_err)
    }
}

/// 删除策略
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
        .map_err(internal_err)
}

// ─── P2: 同步历史记录命令 ───────────────────────────────────────────────────

/// 获取同步历史记录
#[tauri::command]
pub async fn get_sync_history(
    state: State<'_, AppState>,
    limit: u64,
) -> Result<Vec<SyncHistoryEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state
        .history_store
        .get_history(limit as usize)
        .await)
}

/// 获取审计日志
#[tauri::command]
pub async fn get_audit_logs(
    state: State<'_, AppState>,
    limit: u64,
) -> Result<Vec<AuditLogEntry>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state
        .history_store
        .get_audit_logs(limit as usize)
        .await)
}

// ─── P2: 设备权限命令 ───────────────────────────────────────────────────────

/// 获取设备权限
#[tauri::command]
pub async fn get_device_permissions(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<DevicePermissions, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    sync_state
        .permission_store
        .get_permissions(&device_id)
        .await
        .ok_or_else(|| ErrorResponse::from_error("设备权限不存在", ErrorCategory::General))
}

/// 更新设备权限
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
        .map_err(internal_err)
}

/// 列出所有设备权限
#[tauri::command]
pub async fn list_all_permissions(
    state: State<'_, AppState>,
) -> Result<Vec<DevicePermissions>, ErrorResponse> {
    let sync_state = state.device_sync_state.read().await;
    Ok(sync_state.permission_store.get_all_permissions().await)
}
