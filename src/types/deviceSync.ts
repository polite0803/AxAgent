// SPDX-License-Identifier: AGPL-3.0-only

// === 设备同步类型定义 ===

/** 信任级别 */
export type TrustLevel = "backup_only" | "standard" | "full";

/** 设备类型 */
export type DeviceType = "desktop" | "mobile" | "tablet" | "server";

/** 实体类型 */
export type EntityType =
  | "conversation"
  | "message"
  | "setting"
  | "file"
  | "wiki"
  | "knowledge"
  | "agent"
  | "workflow";

/** 变更操作 */
export type ChangeOperation = "create" | "update" | "delete";

/** 冲突解决策略 */
export type ConflictResolutionStrategy =
  | "last_write_wins"
  | "keep_local"
  | "keep_remote"
  | "keep_both";

/** 设备信息 */
export interface DeviceInfo {
  device_id: string;
  name: string;
  hostname: string;
  os: string;
  app_version: string;
  device_type: DeviceType;
  trust_level: TrustLevel;
  is_paired: boolean;
  last_active_at: string;
  registered_at: string;
}

/** 配对码 */
export interface PairingCode {
  code: string;
  created_at: string;
  expires_at: string;
  pending_device_id: string;
}

/** 配对请求 */
export interface PairingRequest {
  request_id: string;
  device: DeviceInfo;
  code: string;
  requested_at: string;
}

/** 配对响应 */
export interface PairingResponse {
  success: boolean;
  message: string;
  assigned_trust_level: TrustLevel;
  session_token: string | null;
  peer_public_key: string | null;
}

/** 版本向量条目 */
export interface VersionVectorEntry {
  device_id: string;
  counter: u64;
}

/** 变更日志条目 */
export interface ChangeLogEntry {
  id: string;
  entity_type: EntityType;
  entity_id: string;
  operation: ChangeOperation;
  device_id: string;
  timestamp: u64;
  version_vector: VersionVectorEntry[];
  data: string | null;
}

/** 冲突信息 */
export interface ConflictInfo {
  id: string;
  entity_type: EntityType;
  entity_id: string;
  conflicting_devices: string[];
  local_vector: VersionVectorEntry[];
  remote_vector: VersionVectorEntry[];
  local_data: string | null;
  remote_data: string | null;
  detected_at: string;
}

/** 同步结果 */
export interface SyncResult {
  success: boolean;
  files_synced: u64;
  files_uploaded: u64;
  files_downloaded: u64;
  conflicts_detected: u64;
  error_message: string | null;
  duration_ms: u64;
}

/** 同步状态 */
export interface DeviceSyncStatus {
  local_device_id: string;
  connected_devices: u64;
  pending_changes: u64;
  last_sync_at: u64 | null;
  is_syncing: boolean;
  sync_progress: number;
}

/** 简化的 u64 类型（与后端对齐） */
export type u64 = number;

// === 实时推送类型 ===

/** WebSocket 信令消息类型（客户端 → 服务端） */
export type SyncSignalType =
  | "device_online"
  | "device_offline"
  | "sync_request"
  | "push_changes"
  | "resolve_conflict"
  | "ping";

/** WebSocket 信令响应类型（服务端 → 客户端） */
export type SyncSignalResponseType =
  | "device_online_ack"
  | "device_offline_ack"
  | "sync_response"
  | "changes_received"
  | "pull_changes"
  | "conflict_resolved"
  | "pong"
  | "error";

/** 信令消息 */
export interface SyncSignal {
  type: SyncSignalType;
  device_id?: string;
  since_timestamp?: u64;
  changes?: ChangeLogEntry[];
  conflict_id?: string;
  strategy?: ConflictResolutionStrategy;
}

/** 信令响应 */
export interface SyncSignalResponse {
  type: SyncSignalResponseType;
  device_id?: string;
  timestamp?: u64;
  result?: SyncResult;
  changes_count?: u64;
  conflicts?: ConflictInfo[];
  changes?: ChangeLogEntry[];
  conflict_id?: string;
  success?: boolean;
  code?: string;
  message?: string;
}

/** WebSocket 连接状态 */
export type WebSocketStatus = "connecting" | "connected" | "disconnected" | "error";

/** 实时推送状态 */
export interface RealtimePushState {
  ws_status: WebSocketStatus;
  ws_connection_id: string | null;
  last_signal_at: u64 | null;
  pending_signals: SyncSignal[];
}

// === 加密同步类型 ===

/** 加密算法 */
export type EncryptionAlgorithm = "aes256_gcm";

/** 密钥派生方式 */
export type KeyDerivation = "pre_shared_key" | "x25519";

/** 同步加密配置 */
export interface SyncEncryptionConfig {
  enabled: boolean;
  algorithm: EncryptionAlgorithm;
  key_derivation: KeyDerivation;
  key_hash: string | null;
}

/** 加密状态 */
export interface EncryptionState {
  config: SyncEncryptionConfig;
  is_encrypting: boolean;
  last_encrypted_at: u64 | null;
  encryption_error: string | null;
}

/** 加密同步数据 */
export interface EncryptedSyncData {
  version: number;
  algorithm: string;
  ciphertext: string;
  nonce: string;
  source_device_id: string;
  target_device_id: string | null;
  encrypted_at: u64;
}

// === 同步策略类型（P2） ===

/** 同步方向 */
export type SyncDirection = "push" | "pull" | "both";

/** 同步类型 */
export type SyncType = "full" | "incremental" | "manual" | "scheduled";

/** 同步策略配置 */
export interface SyncPolicy {
  id: string;
  name: string;
  conflict_strategy: ConflictResolutionStrategy;
  auto_sync_interval_secs: u64;
  sync_scope: EntityType[];
  auto_resolve_conflicts: boolean;
  max_conflict_threshold: u64;
  change_log_retention_enabled: boolean;
  change_log_retention_days: number;
  enabled: boolean;
  updated_at: string;
}

/** 同步策略更新请求 */
export interface SyncPolicyUpdate {
  name?: string;
  conflict_strategy?: ConflictResolutionStrategy;
  auto_sync_interval_secs?: u64;
  sync_scope?: EntityType[];
  auto_resolve_conflicts?: boolean;
  max_conflict_threshold?: u64;
  change_log_retention_enabled?: boolean;
  change_log_retention_days?: number;
  enabled?: boolean;
}

// === 同步历史记录类型（P2） ===

/** 同步历史记录条目 */
export interface SyncHistoryEntry {
  id: string;
  device_id: string;
  direction: SyncDirection;
  sync_type: SyncType;
  result: SyncResult;
  conflicts: ConflictInfo[];
  started_at: string;
  completed_at: string;
  initiated_by: string;
}

// === 设备权限类型（P2） ===

/** 设备操作权限 */
export interface DevicePermissions {
  device_id: string;
  trust_level: TrustLevel;
  allow_push: boolean;
  allow_pull: boolean;
  allow_full_sync: boolean;
  allow_resolve_conflicts: boolean;
  allow_manage_devices: boolean;
  allow_modify_policy: boolean;
  updated_at: string;
}

/** 权限更新请求 */
export interface PermissionUpdate {
  trust_level?: TrustLevel;
  allow_push?: boolean;
  allow_pull?: boolean;
  allow_full_sync?: boolean;
  allow_resolve_conflicts?: boolean;
  allow_manage_devices?: boolean;
  allow_modify_policy?: boolean;
}

// === 审计日志类型（P2） ===

/** 审计操作类型 */
export type AuditAction =
  | "device_registered"
  | "device_paired"
  | "device_unpaired"
  | "sync_started"
  | "sync_completed"
  | "sync_failed"
  | "conflict_detected"
  | "conflict_resolved"
  | "policy_updated"
  | "permission_changed"
  | "encryption_enabled"
  | "encryption_disabled";

/** 审计日志条目 */
export interface AuditLogEntry {
  id: string;
  action: AuditAction;
  entity_type: string;
  entity_id: string;
  device_id: string;
  details: string | null;
  success: boolean;
  error_message: string | null;
  timestamp: string;
}
