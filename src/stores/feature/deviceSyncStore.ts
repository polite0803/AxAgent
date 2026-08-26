// SPDX-License-Identifier: AGPL-3.0-only

import i18n from "@/i18n";
import { invoke, logIpcError } from "@/lib/invoke";
import type {
  AuditLogEntry,
  ChangeLogEntry,
  ConflictInfo,
  ConflictResolutionStrategy,
  DeviceInfo,
  DevicePermissions,
  DeviceSyncStatus,
  EncryptedSyncData,
  EncryptionState,
  PairingCode,
  PairingRequest,
  PairingResponse,
  PermissionUpdate,
  RealtimePushState,
  SyncEncryptionConfig,
  SyncHistoryEntry,
  SyncPolicy,
  SyncPolicyUpdate,
  SyncResult,
  SyncSignal,
  TrustLevel,
} from "@/types";
import { useTranslation } from "react-i18next";
import { create } from "zustand";

/**
 * 解析后端 ErrorResponse 并返回 i18n 翻译后的错误消息
 */
function translateErrorMessage(err: unknown): string {
  const { t } = useTranslation();
  const message = err instanceof Error ? err.message : String(err);

  try {
    const parsed = JSON.parse(message);
    if (parsed?.code && typeof parsed.code === "string") {
      return String(t(`error.${parsed.code}`, parsed.params ?? {}));
    }
  } catch {
    // 非 JSON 格式，直接返回原始消息
  }

  return message;
}

interface DeviceSyncState {
  /** 本地设备信息 */
  localDevice: DeviceInfo | null;
  /** 本地设备 ID */
  localDeviceId: string | null;
  /** 已配对设备列表 */
  devices: DeviceInfo[];
  /** 当前同步状态 */
  syncStatus: DeviceSyncStatus | null;
  /** 变更日志 */
  changeLog: ChangeLogEntry[];
  /** 待解决的冲突 */
  pendingConflicts: ConflictInfo[];
  /** 当前配对码 */
  currentPairingCode: PairingCode | null;
  /** 当前配对请求 */
  currentPairingRequest: PairingRequest | null;
  /** 加载状态 */
  loading: boolean;
  /** 同步中 */
  isSyncing: boolean;
  /** 错误信息 */
  error: string | null;

  // === 实时推送状态 ===
  realtimePush: RealtimePushState;
  connectWebSocket: () => Promise<void>;
  disconnectWebSocket: () => void;
  sendSignal: (signal: SyncSignal) => void;

  // === 加密状态 ===
  encryption: EncryptionState;
  updateEncryptionConfig: (config: Partial<SyncEncryptionConfig>) => void;
  encryptData: (data: string, password?: string) => Promise<EncryptedSyncData | null>;
  decryptData: (data: EncryptedSyncData, password?: string) => Promise<string | null>;

  // === 设备管理 ===
  /** 注册当前设备 */
  registerDevice: (
    name: string,
    hostname: string,
    os: string,
    appVersion: string,
  ) => Promise<DeviceInfo | null>;
  /** 获取本地设备信息 */
  getLocalDevice: () => Promise<DeviceInfo | null>;
  /** 获取已配对设备列表 */
  listDevices: () => Promise<void>;
  /** 取消配对 */
  unpairDevice: (deviceId: string) => Promise<void>;

  // === 配对流程 ===
  /** 生成配对码 */
  generatePairingCode: () => Promise<PairingCode | null>;
  /** 验证配对码 */
  verifyPairingCode: (code: string) => Promise<PairingRequest | null>;
  /** 接受配对请求 */
  acceptPairing: (
    request: PairingRequest,
    trustLevel: TrustLevel,
  ) => Promise<PairingResponse | null>;

  // === 同步操作 ===
  /** 全量同步 */
  fullSync: (deviceId: string) => Promise<SyncResult | null>;
  /** 增量同步 */
  incrementalSync: (deviceId: string) => Promise<SyncResult | null>;
  /** 推送变更 */
  pushChanges: (changes: ChangeLogEntry[]) => Promise<ConflictInfo[]>;
  /** 拉取变更 */
  pullChanges: (sinceTimestamp: number) => Promise<ChangeLogEntry[]>;
  /** 解决冲突 */
  resolveConflict: (
    conflictId: string,
    strategy: ConflictResolutionStrategy,
  ) => Promise<void>;

  // === 状态查询 ===
  /** 获取同步状态 */
  getSyncStatus: () => Promise<DeviceSyncStatus | null>;
  recordChange: (
    entityType: string,
    entityId: string,
    operation: string,
    data: string | null,
  ) => Promise<void>;

  // === 清理 ===
  clearError: () => void;
  reset: () => void;

  // === P2: 同步策略管理 ===
  syncPolicy: SyncPolicy | null;
  syncPolicies: SyncPolicy[];
  loadSyncPolicy: () => Promise<void>;
  updateSyncPolicy: (update: SyncPolicyUpdate) => Promise<SyncPolicy | null>;
  createSyncPolicy: (policy: Omit<SyncPolicy, "id" | "updatedAt">) => Promise<SyncPolicy | null>;
  deleteSyncPolicy: (id: string) => Promise<void>;
  listSyncPolicies: () => Promise<void>;

  // === P2: 同步历史记录 ===
  syncHistory: SyncHistoryEntry[];
  auditLogs: AuditLogEntry[];
  loadSyncHistory: (limit?: number) => Promise<void>;
  loadAuditLogs: (limit?: number) => Promise<void>;

  // === P2: 设备权限管理 ===
  devicePermissions: Map<string, DevicePermissions>;
  loadDevicePermissions: (deviceId: string) => Promise<DevicePermissions | null>;
  updateDevicePermissions: (deviceId: string, update: PermissionUpdate) => Promise<void>;
  listAllPermissions: () => Promise<void>;
}

export const useDeviceSyncStore = create<DeviceSyncState>((set, get) => ({
  localDevice: null,
  localDeviceId: null,
  devices: [],
  syncStatus: null,
  changeLog: [],
  pendingConflicts: [],
  currentPairingCode: null,
  currentPairingRequest: null,
  loading: false,
  isSyncing: false,
  error: null,

  // === 实时推送状态 ===
  realtimePush: {
    wsStatus: "disconnected",
    wsConnectionId: null,
    lastSignalAt: null,
    pendingSignals: [],
  },

  connectWebSocket: async () => {
    set((state) => ({
      realtimePush: {
        ...state.realtimePush,
        wsStatus: "connecting",
      },
    }));
    // WebSocket 连接逻辑将在运行时实现
    set((state) => ({
      realtimePush: {
        ...state.realtimePush,
        wsStatus: "connected",
        wsConnectionId: `conn-${Date.now()}`,
        lastSignalAt: Date.now(),
      },
    }));
  },

  disconnectWebSocket: () => {
    set((state) => ({
      realtimePush: {
        ...state.realtimePush,
        wsStatus: "disconnected",
        wsConnectionId: null,
      },
    }));
  },

  sendSignal: (signal) => {
    set((state) => ({
      realtimePush: {
        ...state.realtimePush,
        pendingSignals: [...state.realtimePush.pendingSignals, signal as never],
        lastSignalAt: Date.now(),
      },
    }));
  },

  // === 加密状态 ===
  encryption: {
    config: {
      enabled: false,
      algorithm: "aes256_gcm",
      keyDerivation: "pre_shared_key",
      keyHash: null,
    },
    isEncrypting: false,
    lastEncryptedAt: null,
    encryptionError: null,
  },

  updateEncryptionConfig: (config) => {
    set((state) => ({
      encryption: {
        ...state.encryption,
        config: {
          ...state.encryption.config,
          ...config,
        },
      },
    }));
  },

  encryptData: async (data, password) => {
    set((state) => ({
      encryption: {
        ...state.encryption,
        isEncrypting: true,
        encryptionError: null,
      },
    }));
    try {
      const encrypted = await invoke<EncryptedSyncData>("encrypt_sync_data", {
        data,
        password: password || undefined,
      });
      set((state) => ({
        encryption: {
          ...state.encryption,
          isEncrypting: false,
          lastEncryptedAt: Date.now(),
        },
      }));
      return encrypted;
    } catch (e) {
      logIpcError("encrypt_sync_data")(e);
      set((state) => ({
        encryption: {
          ...state.encryption,
          isEncrypting: false,
          encryptionError: translateErrorMessage(e),
        },
      }));
      return null;
    }
  },

  decryptData: async (data, password) => {
    set((state) => ({
      encryption: {
        ...state.encryption,
        isEncrypting: true,
        encryptionError: null,
      },
    }));
    try {
      const decrypted = await invoke<string>("decrypt_sync_data", {
        data,
        password: password || undefined,
      });
      set((state) => ({
        encryption: {
          ...state.encryption,
          isEncrypting: false,
          lastEncryptedAt: Date.now(),
        },
      }));
      return decrypted;
    } catch (e) {
      logIpcError("decrypt_sync_data")(e);
      set((state) => ({
        encryption: {
          ...state.encryption,
          isEncrypting: false,
          encryptionError: translateErrorMessage(e),
        },
      }));
      return null;
    }
  },

  // === 设备管理 ===
  registerDevice: async (name, hostname, os, appVersion) => {
    set({ loading: true, error: null });
    try {
      const device = await invoke<DeviceInfo>("register_device", {
        name,
        hostname,
        os,
        app_version: appVersion,
      });
      set({ localDevice: device, loading: false });
      return device;
    } catch (e) {
      logIpcError("register_device")(e);
      set({ loading: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  getLocalDevice: async () => {
    try {
      const device = await invoke<DeviceInfo>("get_local_device");
      set({ localDevice: device, localDeviceId: device.deviceId });
      return device;
    } catch (e) {
      logIpcError("get_local_device")(e);
      return null;
    }
  },

  listDevices: async () => {
    set({ loading: true });
    try {
      const devices = await invoke<DeviceInfo[]>("list_devices");
      set({ devices, loading: false });
    } catch (e) {
      logIpcError("list_devices")(e);
      set({ loading: false, error: translateErrorMessage(e) });
    }
  },

  unpairDevice: async (deviceId) => {
    try {
      await invoke<void>("unpair_device", { deviceId });
      const devices = get().devices.filter((d) => d.deviceId !== deviceId);
      set({ devices });
    } catch (e) {
      logIpcError("unpair_device")(e);
      set({ error: translateErrorMessage(e) });
    }
  },

  // === 配对流程 ===
  generatePairingCode: async () => {
    set({ loading: true, error: null });
    try {
      const code = await invoke<PairingCode>("generate_pairing_code");
      set({ currentPairingCode: code, loading: false });
      return code;
    } catch (e) {
      logIpcError("generate_pairing_code")(e);
      set({ loading: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  verifyPairingCode: async (code) => {
    set({ loading: true, error: null });
    try {
      const request = await invoke<PairingRequest>("verify_pairing_code", { code });
      set({ currentPairingRequest: request, loading: false });
      return request;
    } catch (e) {
      logIpcError("verify_pairing_code")(e);
      set({ loading: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  acceptPairing: async (request, trustLevel) => {
    set({ loading: true, error: null });
    try {
      const response = await invoke<PairingResponse>("accept_pairing", {
        request,
        trust_level: trustLevel,
      });
      if (response.success) {
        const devices = await invoke<DeviceInfo[]>("list_devices");
        set({
          devices,
          currentPairingRequest: null,
          currentPairingCode: null,
          loading: false,
        });
      } else {
        set({ loading: false, error: response.message });
      }
      return response;
    } catch (e) {
      logIpcError("accept_pairing")(e);
      set({ loading: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  // === 同步操作 ===
  fullSync: async (deviceId) => {
    set({ isSyncing: true, error: null });
    try {
      const result = await invoke<SyncResult>("full_sync", {
        device_id: deviceId,
      });
      set({ isSyncing: false });
      if (!result.success) {
        set({ error: result.errorMessage || "Sync failed" });
      }
      return result;
    } catch (e) {
      logIpcError("full_sync")(e);
      set({ isSyncing: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  incrementalSync: async (deviceId) => {
    set({ isSyncing: true, error: null });
    try {
      const result = await invoke<SyncResult>("incremental_sync", {
        device_id: deviceId,
      });
      set({ isSyncing: false });
      if (!result.success) {
        set({ error: result.errorMessage || "Sync failed" });
      }
      return result;
    } catch (e) {
      logIpcError("incremental_sync")(e);
      set({ isSyncing: false, error: translateErrorMessage(e) });
      return null;
    }
  },

  pushChanges: async (changes) => {
    const deviceId = get().localDeviceId;
    if (!deviceId) {
      set({ error: i18n.t("error.DEVICE_SYNC_DEVICE_NOT_INITIALIZED") });
      return [];
    }
    try {
      const conflicts = await invoke<ConflictInfo[]>("push_changes", {
        device_id: deviceId,
        changes,
      });
      if (conflicts.length > 0) {
        set((state) => ({
          pendingConflicts: [...state.pendingConflicts, ...conflicts],
        }));
      }
      return conflicts;
    } catch (e) {
      logIpcError("push_changes")(e);
      set({ error: translateErrorMessage(e) });
      return [];
    }
  },

  pullChanges: async (sinceTimestamp) => {
    const deviceId = get().localDeviceId;
    if (!deviceId) {
      set({ error: i18n.t("error.DEVICE_SYNC_DEVICE_NOT_INITIALIZED") });
      return [];
    }
    try {
      const changes = await invoke<ChangeLogEntry[]>("pull_changes", {
        device_id: deviceId,
        since_timestamp: sinceTimestamp,
      });
      return changes;
    } catch (e) {
      logIpcError("pull_changes")(e);
      set({ error: translateErrorMessage(e) });
      return [];
    }
  },

  resolveConflict: async (conflictId, strategy) => {
    const deviceId = get().localDeviceId;
    if (!deviceId) {
      set({ error: i18n.t("error.DEVICE_SYNC_DEVICE_NOT_INITIALIZED") });
      return;
    }
    try {
      await invoke<void>("resolve_conflict", {
        device_id: deviceId,
        conflict_id: conflictId,
        strategy,
      });
      set((state) => ({
        pendingConflicts: state.pendingConflicts.filter(
          (c) => c.id !== conflictId,
        ),
      }));
    } catch (e) {
      logIpcError("resolve_conflict")(e);
      set({ error: translateErrorMessage(e) });
    }
  },

  // === 状态查询 ===
  getSyncStatus: async () => {
    const deviceId = get().localDeviceId;
    if (!deviceId) {
      set({ error: i18n.t("error.DEVICE_SYNC_DEVICE_NOT_INITIALIZED") });
      return null;
    }
    try {
      const status = await invoke<DeviceSyncStatus>("get_sync_status", {
        device_id: deviceId,
      });
      set({ syncStatus: status });
      return status;
    } catch (e) {
      logIpcError("get_sync_status")(e);
      return null;
    }
  },

  recordChange: async (entityType, entityId, operation, data) => {
    try {
      await invoke<void>("record_change", {
        entity_type: entityType,
        entity_id: entityId,
        operation,
        data,
      });
    } catch (e) {
      logIpcError("record_change")(e);
    }
  },

  // === P2: 同步策略管理 ===
  syncPolicy: null,
  syncPolicies: [],

  loadSyncPolicy: async () => {
    try {
      const policy = await invoke<SyncPolicy | null>("get_sync_policy");
      set({ syncPolicy: policy });
    } catch (e) {
      logIpcError("get_sync_policy")(e);
    }
  },

  updateSyncPolicy: async (update) => {
    try {
      const policy = await invoke<SyncPolicy>("update_sync_policy", { update });
      set({ syncPolicy: policy });
      return policy;
    } catch (e) {
      logIpcError("update_sync_policy")(e);
      set({ error: translateErrorMessage(e) });
      return null;
    }
  },

  createSyncPolicy: async (policy) => {
    try {
      const created = await invoke<SyncPolicy>("create_sync_policy", { policy });
      set((state) => ({
        syncPolicies: [...state.syncPolicies, created],
      }));
      return created;
    } catch (e) {
      logIpcError("create_sync_policy")(e);
      set({ error: translateErrorMessage(e) });
      return null;
    }
  },

  deleteSyncPolicy: async (id) => {
    try {
      await invoke<void>("delete_sync_policy", { id });
      set((state) => ({
        syncPolicies: state.syncPolicies.filter((p) => p.id !== id),
      }));
    } catch (e) {
      logIpcError("delete_sync_policy")(e);
      set({ error: translateErrorMessage(e) });
    }
  },

  listSyncPolicies: async () => {
    try {
      const policies = await invoke<SyncPolicy[]>("list_sync_policies");
      set({ syncPolicies: policies });
    } catch (e) {
      logIpcError("list_sync_policies")(e);
    }
  },

  // === P2: 同步历史记录 ===
  syncHistory: [],
  auditLogs: [],

  loadSyncHistory: async (limit) => {
    try {
      const history = await invoke<SyncHistoryEntry[]>("get_sync_history", {
        limit: limit ?? 50,
      });
      set({ syncHistory: history });
    } catch (e) {
      logIpcError("get_sync_history")(e);
    }
  },

  loadAuditLogs: async (limit) => {
    try {
      const logs = await invoke<AuditLogEntry[]>("get_audit_logs", {
        limit: limit ?? 100,
      });
      set({ auditLogs: logs });
    } catch (e) {
      logIpcError("get_audit_logs")(e);
    }
  },

  // === P2: 设备权限管理 ===
  devicePermissions: new Map(),

  loadDevicePermissions: async (deviceId) => {
    try {
      const perms = await invoke<DevicePermissions>("get_device_permissions", {
        device_id: deviceId,
      });
      set((state) => {
        const newMap = new Map(state.devicePermissions);
        newMap.set(deviceId, perms);
        return { devicePermissions: newMap };
      });
      return perms;
    } catch (e) {
      logIpcError("get_device_permissions")(e);
      return null;
    }
  },

  updateDevicePermissions: async (deviceId, update) => {
    try {
      const perms = await invoke<DevicePermissions>("update_device_permissions", {
        device_id: deviceId,
        update,
      });
      set((state) => {
        const newMap = new Map(state.devicePermissions);
        newMap.set(deviceId, perms);
        return { devicePermissions: newMap };
      });
    } catch (e) {
      logIpcError("update_device_permissions")(e);
      set({ error: translateErrorMessage(e) });
    }
  },

  listAllPermissions: async () => {
    try {
      const list = await invoke<DevicePermissions[]>("list_all_permissions");
      const newMap = new Map<string, DevicePermissions>();
      for (const p of list) {
        newMap.set(p.deviceId, p);
      }
      set({ devicePermissions: newMap });
    } catch (e) {
      logIpcError("list_all_permissions")(e);
    }
  },

  // === 清理 ===
  clearError: () => set({ error: null }),

  reset: () =>
    set({
      localDevice: null,
      localDeviceId: null,
      devices: [],
      syncStatus: null,
      changeLog: [],
      pendingConflicts: [],
      currentPairingCode: null,
      currentPairingRequest: null,
      loading: false,
      isSyncing: false,
      error: null,
      realtimePush: {
        wsStatus: "disconnected",
        wsConnectionId: null,
        lastSignalAt: null,
        pendingSignals: [],
      },
      encryption: {
        config: {
          enabled: false,
          algorithm: "aes256_gcm",
          keyDerivation: "pre_shared_key",
          keyHash: null,
        },
        isEncrypting: false,
        lastEncryptedAt: null,
        encryptionError: null,
      },
      // P2 reset
      syncPolicy: null,
      syncPolicies: [],
      syncHistory: [],
      auditLogs: [],
      devicePermissions: new Map(),
    }),
}));
