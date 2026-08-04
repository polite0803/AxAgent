// SPDX-License-Identifier: AGPL-3.0-only

//! 设备同步核心类型定义
//!
//! 定义多设备同步所需的 DTO 和 trait 接口：
//! - DeviceInfo: 设备身份信息
//! - PairingRequest/Response: 设备配对协议
//! - ChangeLogEntry: 增量日志条目
//! - ConflictResolution: 冲突解决策略

use serde::{Deserialize, Serialize};

// ─── 设备身份 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    /// 唯一设备 ID（UUID v4）
    pub device_id: String,
    /// 设备显示名称（用户可修改）
    pub name: String,
    /// 主机名
    pub hostname: String,
    /// 操作系统
    pub os: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 应用版本
    pub app_version: String,
    /// 首次注册时间（RFC3339）
    pub registered_at: String,
    /// 最后活跃时间（RFC3339）
    pub last_active_at: String,
    /// 是否已配对
    pub is_paired: bool,
    /// 配对级别
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    Desktop,
    Mobile,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// 仅备份权限
    BackupOnly,
    /// 标准同步权限
    Standard,
    /// 完全控制权限
    Full,
}

// ─── 设备配对协议 ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequest {
    /// 请求配对的设备信息
    pub device: DeviceInfo,
    /// 配对码（6位数字，用于手动确认）
    pub pairing_code: String,
    /// 可选：公钥（用于端到端加密）
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingResponse {
    /// 是否接受配对
    pub success: bool,
    /// 响应消息
    pub message: String,
    /// 分配的信任级别
    pub assigned_trust_level: TrustLevel,
    /// 会话令牌（用于后续同步认证）
    pub session_token: Option<String>,
    /// 可选：对方公钥（用于端到端加密）
    pub peer_public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    /// 6位配对码
    pub code: String,
    /// 生成时间
    pub created_at: String,
    /// 过期时间
    pub expires_at: String,
    /// 设备 ID（待配对设备）
    pub pending_device_id: String,
}

// ─── 增量日志 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLogEntry {
    /// 变更 ID
    pub id: String,
    /// 实体类型
    pub entity_type: EntityType,
    /// 实体 ID
    pub entity_id: String,
    /// 操作类型
    pub operation: ChangeOperation,
    /// 变更时间（Unix epoch ms）
    pub timestamp: u64,
    /// 来源设备 ID
    pub device_id: String,
    /// 版本向量（用于冲突检测）
    pub version_vector: Vec<VersionVectorEntry>,
    /// 变更数据（JSON）
    pub data: Option<String>,
    /// 已同步到的目标设备 ID 列表
    #[serde(default)]
    pub synced_to: Vec<String>,
    /// 是否已同步
    #[serde(default)]
    pub is_synced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Conversation,
    Message,
    Setting,
    File,
    Wiki,
    Knowledge,
    Agent,
    Workflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOperation {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionVectorEntry {
    pub device_id: String,
    pub counter: u64,
}

// ─── 冲突解决 ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictInfo {
    /// 冲突 ID
    pub id: String,
    /// 冲突的实体类型
    pub entity_type: EntityType,
    /// 冲突的实体 ID
    pub entity_id: String,
    /// 涉及的设备
    pub conflicting_devices: Vec<String>,
    /// 本地版本向量
    pub local_vector: Vec<VersionVectorEntry>,
    /// 远程版本向量
    pub remote_vector: Vec<VersionVectorEntry>,
    /// 本地数据快照（JSON）
    pub local_data: Option<String>,
    /// 远程数据快照（JSON）
    pub remote_data: Option<String>,
    /// 本地数据时间戳（Unix epoch ms）
    #[serde(default)]
    pub local_timestamp: u64,
    /// 远程数据时间戳（Unix epoch ms）
    #[serde(default)]
    pub remote_timestamp: u64,
    /// 冲突发生时间
    pub detected_at: String,
    /// 是否已解决
    #[serde(default)]
    pub resolved: bool,
    /// 解决策略（已解决的冲突）
    #[serde(default)]
    pub resolution_applied: Option<String>,
    /// 解决时间（已解决的冲突）
    #[serde(default)]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// 保留本地版本
    KeepLocal,
    /// 保留远程版本
    KeepRemote,
    /// 保留双方（标记为冲突，手动合并）
    KeepBoth,
    /// 最后写入胜出（按时间戳）
    LastWriteWins,
    /// 自定义合并策略
    CustomMerge,
}

// ─── 同步状态 ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSyncStatus {
    /// 本地设备 ID
    pub local_device_id: String,
    /// 最后同步时间（Unix epoch ms）
    pub last_sync_at: Option<u64>,
    /// 最后同步状态
    pub last_sync_result: Option<SyncResult>,
    /// 待处理的变更数
    pub pending_changes: u64,
    /// 同步进度（0-100）
    pub sync_progress: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    /// 是否成功
    pub success: bool,
    /// 同步的文件数
    pub files_synced: u64,
    /// 上传的文件数
    pub files_uploaded: u64,
    /// 下载的文件数
    pub files_downloaded: u64,
    /// 冲突数
    pub conflicts_detected: u64,
    /// 错误信息
    pub error_message: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: u64,
}

// ─── 设备管理 Trait ──────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait DeviceManager: Send + Sync {
    /// 注册新设备
    async fn register_device(&self, device: DeviceInfo) -> Result<DeviceInfo, String>;

    /// 列出所有已配对设备
    async fn list_devices(&self) -> Result<Vec<DeviceInfo>, String>;

    /// 接受设备配对请求
    async fn accept_pairing(
        &self,
        request: PairingRequest,
        trust_level: TrustLevel,
    ) -> Result<PairingResponse, String>;

    /// 拒绝设备配对请求
    async fn reject_pairing(&self, device_id: &str) -> Result<(), String>;

    /// 撤销设备配对
    async fn unpair_device(&self, device_id: &str) -> Result<(), String>;

    /// 生成配对码
    async fn generate_pairing_code(&self) -> Result<PairingCode, String>;

    /// 验证配对码
    async fn verify_pairing_code(&self, code: &str) -> Result<PairingRequest, String>;

    /// 更新设备最后活跃时间
    async fn update_device_activity(&self, device_id: &str) -> Result<(), String>;
}

// ─── 同步引擎 Trait ──────────────────────────────────────────────────────

#[async_trait::async_trait]
pub trait SyncEngine: Send + Sync {
    /// 执行全量同步
    async fn full_sync(&self, device_id: &str) -> Result<SyncResult, String>;

    /// 执行增量同步
    async fn incremental_sync(&self, device_id: &str) -> Result<SyncResult, String>;

    /// 推送变更日志
    async fn push_changes(&self, changes: Vec<ChangeLogEntry>)
    -> Result<Vec<ConflictInfo>, String>;

    /// 拉取变更日志
    async fn pull_changes(&self, since_timestamp: u64) -> Result<Vec<ChangeLogEntry>, String>;

    /// 解决冲突
    async fn resolve_conflict(
        &self,
        conflict: &ConflictInfo,
        strategy: ConflictResolutionStrategy,
    ) -> Result<(), String>;

    /// 获取设备同步状态
    async fn get_sync_status(&self, device_id: &str) -> Result<DeviceSyncStatus, String>;
}

// ─── 同步策略配置 ─────────────────────────────────────────────────────

/// 同步策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPolicy {
    /// 策略 ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 冲突解决策略
    pub conflict_strategy: ConflictResolutionStrategy,
    /// 自动同步频率（秒），0 表示手动同步
    pub auto_sync_interval_secs: u64,
    /// 同步的实体类型范围
    pub sync_scope: Vec<EntityType>,
    /// 是否启用自动冲突解决
    pub auto_resolve_conflicts: bool,
    /// 允许的最大同步冲突数，超过则停止自动同步
    pub max_conflict_threshold: u64,
    /// 是否启用变更日志保留
    pub change_log_retention_enabled: bool,
    /// 变更日志保留天数
    pub change_log_retention_days: u32,
    /// 策略是否启用
    pub enabled: bool,
    /// 更新时间
    pub updated_at: String,
}

/// 同步策略更新请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncPolicyUpdate {
    pub name: Option<String>,
    pub conflict_strategy: Option<ConflictResolutionStrategy>,
    pub auto_sync_interval_secs: Option<u64>,
    pub sync_scope: Option<Vec<EntityType>>,
    pub auto_resolve_conflicts: Option<bool>,
    pub max_conflict_threshold: Option<u64>,
    pub change_log_retention_enabled: Option<bool>,
    pub change_log_retention_days: Option<u32>,
    pub enabled: Option<bool>,
}

// ─── 同步历史记录 ─────────────────────────────────────────────────────

/// 同步历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncHistoryEntry {
    /// 历史记录 ID
    pub id: String,
    /// 设备 ID
    pub device_id: String,
    /// 同步方向
    pub direction: SyncDirection,
    /// 同步类型（全量/增量）
    pub sync_type: SyncType,
    /// 同步结果
    pub result: SyncResult,
    /// 冲突详情（如果有）
    pub conflicts: Vec<ConflictInfo>,
    /// 开始时间（RFC3339）
    pub started_at: String,
    /// 结束时间（RFC3339）
    pub completed_at: String,
    /// 发起人
    pub initiated_by: String,
}

/// 同步方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncDirection {
    Push,
    Pull,
    Both,
}

/// 同步类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncType {
    Full,
    Incremental,
    Manual,
    Scheduled,
}

// ─── 设备权限管理 ─────────────────────────────────────────────────────

/// 设备操作权限
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePermissions {
    /// 设备 ID
    pub device_id: String,
    /// 信任级别
    pub trust_level: TrustLevel,
    /// 是否允许推送变更
    pub allow_push: bool,
    /// 是否允许拉取变更
    pub allow_pull: bool,
    /// 是否允许全量同步
    pub allow_full_sync: bool,
    /// 是否允许解决冲突
    pub allow_resolve_conflicts: bool,
    /// 是否允许管理其他设备
    pub allow_manage_devices: bool,
    /// 是否允许修改同步策略
    pub allow_modify_policy: bool,
    /// 权限更新时间
    pub updated_at: String,
}

/// 权限更新请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PermissionUpdate {
    pub trust_level: Option<TrustLevel>,
    pub allow_push: Option<bool>,
    pub allow_pull: Option<bool>,
    pub allow_full_sync: Option<bool>,
    pub allow_resolve_conflicts: Option<bool>,
    pub allow_manage_devices: Option<bool>,
    pub allow_modify_policy: Option<bool>,
}

/// 权限类型（用于权限检查中间件）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionType {
    Push,
    Pull,
    FullSync,
    ResolveConflicts,
    ManageDevices,
    ModifyPolicy,
}

// ─── 审计日志 ──────────────────────────────────────────────────────────

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogEntry {
    /// 日志 ID
    pub id: String,
    /// 操作类型
    pub action: AuditAction,
    /// 操作实体类型
    pub entity_type: String,
    /// 操作实体 ID
    pub entity_id: String,
    /// 操作设备 ID
    pub device_id: String,
    /// 操作详情（JSON）
    pub details: Option<String>,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
    /// 操作时间（RFC3339）
    pub timestamp: String,
}

/// 审计操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    DeviceRegistered,
    DevicePaired,
    DeviceUnpaired,
    SyncStarted,
    SyncCompleted,
    SyncFailed,
    ConflictDetected,
    ConflictResolved,
    PolicyUpdated,
    PermissionChanged,
    EncryptionEnabled,
    EncryptionDisabled,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeviceRegistered => write!(f, "device_registered"),
            Self::DevicePaired => write!(f, "device_paired"),
            Self::DeviceUnpaired => write!(f, "device_unpaired"),
            Self::SyncStarted => write!(f, "sync_started"),
            Self::SyncCompleted => write!(f, "sync_completed"),
            Self::SyncFailed => write!(f, "sync_failed"),
            Self::ConflictDetected => write!(f, "conflict_detected"),
            Self::ConflictResolved => write!(f, "conflict_resolved"),
            Self::PolicyUpdated => write!(f, "policy_updated"),
            Self::PermissionChanged => write!(f, "permission_changed"),
            Self::EncryptionEnabled => write!(f, "encryption_enabled"),
            Self::EncryptionDisabled => write!(f, "encryption_disabled"),
        }
    }
}

// ─── 信令消息类型（WebSocket 实时推送）──────────────────────────────────

/// 信令消息类型（客户端 → 服务端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncSignal {
    /// 设备上线
    DeviceOnline { device_id: String },
    /// 设备下线
    DeviceOffline { device_id: String },
    /// 请求同步
    SyncRequest { device_id: String, since_timestamp: Option<u64> },
    /// 推送变更
    PushChanges { device_id: String, changes: Vec<ChangeLogEntry> },
    /// 请求解决冲突
    ResolveConflict { device_id: String, conflict_id: String, strategy: ConflictResolutionStrategy },
    /// 心跳
    Ping { device_id: String },
    /// 注册设备
    RegisterDevice { device: DeviceInfo },
}

/// 信令响应类型（服务端 → 客户端）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SyncSignalResponse {
    /// 设备已上线
    DeviceOnlineAck { device_id: String, timestamp: u64 },
    /// 设备已下线
    DeviceOfflineAck { device_id: String, timestamp: u64 },
    /// 同步请求响应
    SyncResponse { device_id: String, result: SyncResult },
    /// 接收变更
    ChangesReceived { device_id: String, changes_count: u64, conflicts: Vec<ConflictInfo> },
    /// 拉取变更
    PullChanges { device_id: String, changes: Vec<ChangeLogEntry> },
    /// 冲突已解决
    ConflictResolved { device_id: String, conflict_id: String, success: bool },
    /// 心跳响应
    Pong { device_id: String },
    /// 错误
    Error { code: String, message: String },
}

/// 信令服务 Trait
#[async_trait::async_trait]
pub trait SignalService: Send + Sync {
    /// 发送信令消息到指定设备
    async fn send_signal(
        &self,
        target_device_id: &str,
        signal: SyncSignalResponse,
    ) -> Result<(), String>;

    /// 广播信令消息到所有在线设备
    async fn broadcast_signal(&self, signal: SyncSignalResponse) -> Result<(), String>;

    /// 设备上线
    async fn mark_online(&self, device_id: &str, connection_id: &str) -> Result<(), String>;

    /// 设备下线
    async fn mark_offline(&self, device_id: &str) -> Result<(), String>;

    /// 检查设备是否在线
    async fn is_online(&self, device_id: &str) -> bool;
}

// ─── 数据持久化 Trait ──────────────────────────────────────────────────

/// 同步数据持久化接口
///
/// 定义同步模块所需的所有数据持久化操作，
/// 通过 Sea-ORM 抽象层支持 PostgreSQL/SQLite 双数据库，
/// 同时允许内存存储等其他实现用于测试场景。
#[async_trait::async_trait]
pub trait SyncStorage: Send + Sync {
    // ─── 设备存储 ──────────────────────────────────────────────────────

    /// 保存设备信息
    async fn save_device(&self, device: &DeviceInfo) -> Result<(), String>;

    /// 获取所有设备
    async fn get_all_devices(&self) -> Result<Vec<DeviceInfo>, String>;

    /// 获取已配对设备
    async fn get_paired_devices(&self) -> Result<Vec<DeviceInfo>, String>;

    /// 根据 ID 获取设备
    async fn get_device_by_id(&self, device_id: &str) -> Result<Option<DeviceInfo>, String>;

    /// 按 ID 更新设备
    async fn update_device(&self, device: &DeviceInfo) -> Result<(), String>;

    /// 按 ID 删除设备
    async fn delete_device(&self, device_id: &str) -> Result<(), String>;

    // ─── 变更日志存储 ──────────────────────────────────────────────────

    /// 添加变更日志
    async fn add_change_log(&self, entry: &ChangeLogEntry) -> Result<(), String>;

    /// 批量添加变更日志
    async fn batch_add_change_logs(&self, entries: &[ChangeLogEntry]) -> Result<(), String>;

    /// 获取指定设备的变更日志
    async fn get_change_logs_by_device(
        &self,
        device_id: &str,
        since_timestamp: Option<u64>,
    ) -> Result<Vec<ChangeLogEntry>, String>;

    /// 获取未同步的变更日志
    async fn get_unsynced_change_logs(
        &self,
        device_id: &str,
    ) -> Result<Vec<ChangeLogEntry>, String>;

    /// 标记变更为已同步
    async fn mark_changes_as_synced(
        &self,
        change_ids: &[String],
        target_device_id: &str,
    ) -> Result<(), String>;

    // ─── 策略存储 ──────────────────────────────────────────────────────

    /// 保存同步策略
    async fn save_policy(&self, policy: &SyncPolicy) -> Result<(), String>;

    /// 获取所有策略
    async fn get_all_policies(&self) -> Result<Vec<SyncPolicy>, String>;

    /// 获取启用的策略
    async fn get_enabled_policies(&self) -> Result<Vec<SyncPolicy>, String>;

    /// 根据 ID 获取策略
    async fn get_policy_by_id(&self, policy_id: &str) -> Result<Option<SyncPolicy>, String>;

    /// 删除策略
    async fn delete_policy(&self, policy_id: &str) -> Result<(), String>;

    // ─── 历史记录存储 ──────────────────────────────────────────────────

    /// 添加同步历史记录
    async fn add_history_entry(&self, entry: &SyncHistoryEntry) -> Result<(), String>;

    /// 获取指定设备的同步历史
    async fn get_history_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<SyncHistoryEntry>, String>;

    // ─── 权限存储 ──────────────────────────────────────────────────────

    /// 保存设备权限
    async fn save_permissions(&self, permissions: &DevicePermissions) -> Result<(), String>;

    /// 获取设备权限
    async fn get_permissions_by_device(
        &self,
        device_id: &str,
    ) -> Result<Option<DevicePermissions>, String>;

    /// 获取所有权限配置
    async fn get_all_permissions(&self) -> Result<Vec<DevicePermissions>, String>;

    /// 删除设备权限
    async fn delete_permissions(&self, device_id: &str) -> Result<(), String>;

    // ─── 审计日志存储 ──────────────────────────────────────────────────

    /// 添加审计日志
    async fn add_audit_log(&self, entry: &AuditLogEntry) -> Result<(), String>;

    /// 批量添加审计日志
    async fn batch_add_audit_logs(&self, entries: &[AuditLogEntry]) -> Result<(), String>;

    /// 根据设备获取审计日志
    async fn get_audit_logs_by_device(
        &self,
        device_id: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String>;

    /// 根据操作类型获取审计日志
    async fn get_audit_logs_by_action(
        &self,
        action: &str,
        limit: Option<u64>,
    ) -> Result<Vec<AuditLogEntry>, String>;
}

// ─── 远程云存储（WebDAV/S3） ────────────────────────────────────────────

/// 远程存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStorageConfig {
    /// 存储类型
    pub storage_type: RemoteStorageType,
    /// 服务器地址（WebDAV URL 或 S3 endpoint）
    pub endpoint: String,
    /// 访问凭据
    pub credentials: RemoteStorageCredentials,
    /// 存储桶/根路径
    pub bucket_or_path: String,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// 远程存储类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteStorageType {
    /// WebDAV 存储
    Webdav,
    /// Amazon S3 兼容存储
    S3,
    /// 本地文件系统（用于测试）
    Local,
}

/// 远程存储访问凭据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteStorageCredentials {
    /// 访问密钥
    pub access_key: String,
    /// 密钥/密码
    pub secret_key: String,
    /// 可选的会话令牌
    #[serde(default)]
    pub session_token: Option<String>,
}

/// 远程存储 trait，用于云同步备份
#[async_trait::async_trait]
pub trait RemoteStorage: Send + Sync {
    /// 上传数据到远程存储
    async fn upload(&self, path: &str, data: &[u8]) -> Result<(), String>;

    /// 从远程存储下载数据
    async fn download(&self, path: &str) -> Result<Vec<u8>, String>;

    /// 列出远程存储中的文件
    async fn list(&self, prefix: &str) -> Result<Vec<RemoteFileInfo>, String>;

    /// 删除远程存储中的文件
    async fn delete(&self, path: &str) -> Result<(), String>;

    /// 检查远程存储连通性
    async fn health_check(&self) -> Result<bool, String>;

    /// 获取远程存储配置
    fn config(&self) -> &RemoteStorageConfig;
}

/// 远程文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileInfo {
    /// 文件路径
    pub path: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 最后修改时间（Unix epoch ms）
    pub last_modified: u64,
    /// 内容类型
    pub content_type: Option<String>,
}
