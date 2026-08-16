// SPDX-License-Identifier: AGPL-3.0-only

// === 设备同步类型定义 ===
//
// 与后端 DTO 对齐：后端 struct 字段保持 snake_case，通过
// `#[serde(rename_all = "camelCase")]` 输出 camelCase，前端消费 camelCase。

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
  deviceId: string;
  name: string;
  hostname: string;
  os: string;
  appVersion: string;
  deviceType: DeviceType;
  trustLevel: TrustLevel;
  isPaired: boolean;
  lastActiveAt: string;
  registeredAt: string;
}

/** 配对码 */
export interface PairingCode {
  code: string;
  createdAt: string;
  expiresAt: string;
  pendingDeviceId: string;
}

/** 配对请求 */
export interface PairingRequest {
  requestId: string;
  device: DeviceInfo;
  code: string;
  requestedAt: string;
}

/** 配对响应 */
export interface PairingResponse {
  success: boolean;
  message: string;
  assignedTrustLevel: TrustLevel;
  sessionToken: string | null;
  peerPublicKey: string | null;
}

/** 版本向量条目 */
export interface VersionVectorEntry {
  deviceId: string;
  counter: u64;
}

/** 变更日志条目 */
export interface ChangeLogEntry {
  id: string;
  entityType: EntityType;
  entityId: string;
  operation: ChangeOperation;
  deviceId: string;
  timestamp: u64;
  versionVector: VersionVectorEntry[];
  data: string | null;
}

/** 冲突信息 */
export interface ConflictInfo {
  id: string;
  entityType: EntityType;
  entityId: string;
  conflictingDevices: string[];
  localVector: VersionVectorEntry[];
  remoteVector: VersionVectorEntry[];
  localData: string | null;
  remoteData: string | null;
  detectedAt: string;
}

/** 同步结果 */
export interface SyncResult {
  success: boolean;
  filesSynced: u64;
  filesUploaded: u64;
  filesDownloaded: u64;
  conflictsDetected: u64;
  errorMessage: string | null;
  durationMs: u64;
}

/** 同步状态 */
export interface DeviceSyncStatus {
  localDeviceId: string;
  connectedDevices: u64;
  pendingChanges: u64;
  lastSyncAt: u64 | null;
  isSyncing: boolean;
  syncProgress: number;
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
  deviceId?: string;
  sinceTimestamp?: u64;
  changes?: ChangeLogEntry[];
  conflictId?: string;
  strategy?: ConflictResolutionStrategy;
}

/** 信令响应 */
export interface SyncSignalResponse {
  type: SyncSignalResponseType;
  deviceId?: string;
  timestamp?: u64;
  result?: SyncResult;
  changesCount?: u64;
  conflicts?: ConflictInfo[];
  changes?: ChangeLogEntry[];
  conflictId?: string;
  success?: boolean;
  code?: string;
  message?: string;
}

/** WebSocket 连接状态 */
export type WebSocketStatus = "connecting" | "connected" | "disconnected" | "error";

/** 实时推送状态 */
export interface RealtimePushState {
  wsStatus: WebSocketStatus;
  wsConnectionId: string | null;
  lastSignalAt: u64 | null;
  pendingSignals: SyncSignal[];
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
  keyDerivation: KeyDerivation;
  keyHash: string | null;
}

/** 加密状态 */
export interface EncryptionState {
  config: SyncEncryptionConfig;
  isEncrypting: boolean;
  lastEncryptedAt: u64 | null;
  encryptionError: string | null;
}

/** 加密同步数据 */
export interface EncryptedSyncData {
  version: number;
  algorithm: string;
  ciphertext: string;
  nonce: string;
  sourceDeviceId: string;
  targetDeviceId: string | null;
  encryptedAt: u64;
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
  conflictStrategy: ConflictResolutionStrategy;
  autoSyncIntervalSecs: u64;
  syncScope: EntityType[];
  autoResolveConflicts: boolean;
  maxConflictThreshold: u64;
  changeLogRetentionEnabled: boolean;
  changeLogRetentionDays: number;
  enabled: boolean;
  updatedAt: string;
}

/** 同步策略更新请求 */
export interface SyncPolicyUpdate {
  name?: string;
  conflictStrategy?: ConflictResolutionStrategy;
  autoSyncIntervalSecs?: u64;
  syncScope?: EntityType[];
  autoResolveConflicts?: boolean;
  maxConflictThreshold?: u64;
  changeLogRetentionEnabled?: boolean;
  changeLogRetentionDays?: number;
  enabled?: boolean;
}

// === 同步历史记录类型（P2） ===

/** 同步历史记录条目 */
export interface SyncHistoryEntry {
  id: string;
  deviceId: string;
  direction: SyncDirection;
  syncType: SyncType;
  result: SyncResult;
  conflicts: ConflictInfo[];
  startedAt: string;
  completedAt: string;
  initiatedBy: string;
}

// === 设备权限类型（P2） ===

/** 设备操作权限 */
export interface DevicePermissions {
  deviceId: string;
  trustLevel: TrustLevel;
  allowPush: boolean;
  allowPull: boolean;
  allowFullSync: boolean;
  allowResolveConflicts: boolean;
  allowManageDevices: boolean;
  allowModifyPolicy: boolean;
  updatedAt: string;
}

/** 权限更新请求 */
export interface PermissionUpdate {
  trustLevel?: TrustLevel;
  allowPush?: boolean;
  allowPull?: boolean;
  allowFullSync?: boolean;
  allowResolveConflicts?: boolean;
  allowManageDevices?: boolean;
  allowModifyPolicy?: boolean;
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
  entityType: string;
  entityId: string;
  deviceId: string;
  details: string | null;
  success: boolean;
  errorMessage: string | null;
  timestamp: string;
}
