// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type {
  CreateDynamicUISchemaParams,
  DynamicUIFormDataRecord,
  DynamicUIPinRecord,
  DynamicUISchemaRecord,
  DynamicUISchemaVersion,
  ListVersionsResponse,
  PinDynamicUISchemaParams,
  SaveDynamicUIFormDataParams,
  UpdateDynamicUIPinParams,
  UpdateDynamicUISchemaParams,
} from "@/types";
import { create } from "zustand";

const FORM_CACHE_UNDEFINED_KEY = "__undefined__";

function formCacheKey(schemaId: string, instanceKey?: string): string {
  return `${schemaId}:${instanceKey ?? FORM_CACHE_UNDEFINED_KEY}`;
}

interface DynamicUIState {
  schemas: DynamicUISchemaRecord[];
  loading: boolean;
  currentSchema: DynamicUISchemaRecord | null;
  formDataCache: Map<string, Record<string, unknown>>;
  /** 当前 schema 的版本列表缓存 */
  versionList: DynamicUISchemaVersion[];
  versionLoading: boolean;

  fetchSchemas: (category?: string) => Promise<void>;
  getSchema: (id: string) => Promise<DynamicUISchemaRecord>;
  createSchema: (params: CreateDynamicUISchemaParams) => Promise<DynamicUISchemaRecord>;
  updateSchema: (id: string, params: UpdateDynamicUISchemaParams) => Promise<DynamicUISchemaRecord>;
  deleteSchema: (id: string) => Promise<void>;

  saveFormData: (params: SaveDynamicUIFormDataParams) => Promise<DynamicUIFormDataRecord>;
  loadFormData: (schemaId: string, instanceKey?: string) => Promise<Record<string, unknown> | null>;
  clearFormData: (schemaId: string, instanceKey?: string) => Promise<void>;

  setCurrentSchema: (schema: DynamicUISchemaRecord | null) => void;

  // ── 导航钉入配置（后端持久化） ──
  pins: DynamicUIPinRecord[];
  fetchPins: () => Promise<void>;
  pinSchema: (params: PinDynamicUISchemaParams) => Promise<DynamicUIPinRecord>;
  unpinSchema: (schemaId: string) => Promise<void>;
  updatePin: (schemaId: string, params: UpdateDynamicUIPinParams) => Promise<void>;

  // ── 版本管理 ──
  loadVersions: (schemaId: string) => Promise<DynamicUISchemaVersion[]>;
  getVersion: (versionId: number) => Promise<DynamicUISchemaVersion | null>;
  restoreVersion: (schemaId: string, versionId: number) => Promise<DynamicUISchemaRecord | null>;
}

export const useDynamicUIStore = create<DynamicUIState>((set, get) => ({
  schemas: [],
  loading: false,
  currentSchema: null,
  formDataCache: new Map(),
  versionList: [],
  versionLoading: false,
  pins: [],

  fetchSchemas: async (category) => {
    set({ loading: true });
    try {
      const schemas = await invoke<DynamicUISchemaRecord[]>("list_dynamic_ui_schemas", {
        category: category || null,
      });
      set({ schemas, loading: false });
    } catch (e) {
      // DUI-P2-01: 记录错误日志便于排查
      console.error("fetchSchemas failed:", e);
      set({ loading: false });
    }
  },

  getSchema: async (id) => {
    const existing = get().schemas.find((s) => s.id === id);
    if (existing) {
      set({ currentSchema: existing });
      return existing;
    }
    const schema = await invoke<DynamicUISchemaRecord>("get_dynamic_ui_schema", { id });
    set({ currentSchema: schema });
    return schema;
  },

  createSchema: async (params) => {
    const schema = await invoke<DynamicUISchemaRecord>("create_dynamic_ui_schema", { req: params });
    set((state) => ({ schemas: [schema, ...state.schemas], currentSchema: schema }));
    return schema;
  },

  updateSchema: async (id, params) => {
    const schema = await invoke<DynamicUISchemaRecord>("update_dynamic_ui_schema", {
      id,
      req: params,
    });
    set((state) => ({
      schemas: state.schemas.map((s) => (s.id === id ? schema : s)),
      currentSchema: state.currentSchema?.id === id ? schema : state.currentSchema,
    }));
    // 缺陷 6：标题变更时同步钉入导航配置的标题，避免侧栏显示旧标题
    if (params.title) {
      const pin = get().pins.find((p) => p.schemaId === id);
      if (pin && pin.title !== params.title) {
        await get().updatePin(id, { title: params.title });
      }
    }
    return schema;
  },

  deleteSchema: async (id) => {
    await invoke<void>("delete_dynamic_ui_schema", { id });
    // 同步清理钉入导航配置，避免残留脏数据（缺陷 3）
    if (get().pins.some((p) => p.schemaId === id)) {
      await get().unpinSchema(id);
    }
    set((state) => ({
      schemas: state.schemas.filter((s) => s.id !== id),
      currentSchema: state.currentSchema?.id === id ? null : state.currentSchema,
    }));
  },

  // ── 导航钉入配置（后端持久化） ──

  fetchPins: async () => {
    try {
      const pins = await invoke<DynamicUIPinRecord[]>("list_dynamic_ui_pins");
      set({ pins });
    } catch (e) {
      // DUI-P2-01: pins 加载失败可静默，但记录日志便于排查
      console.warn("fetchPins failed (ignorable):", e);
    }
  },

  pinSchema: async (params) => {
    const record = await invoke<DynamicUIPinRecord>("pin_dynamic_ui_schema", {
      schema_id: params.schemaId,
      title: params.title,
      group_name: params.groupName,
      position: params.position ?? null,
    });
    set((state) => {
      const others = state.pins.filter((p) => p.schemaId !== record.schemaId);
      return { pins: [...others, record] };
    });
    return record;
  },

  unpinSchema: async (schemaId) => {
    await invoke<void>("unpin_dynamic_ui_schema", { schemaId });
    set((state) => ({
      pins: state.pins.filter((p) => p.schemaId !== schemaId),
    }));
  },

  updatePin: async (schemaId, params) => {
    const existing = get().pins.find((p) => p.schemaId === schemaId);
    if (!existing) {
      return;
    }
    const merged = {
      schemaId: schemaId,
      title: params.title ?? existing.title,
      groupName: params.groupName ?? existing.groupName,
      position: params.position ?? existing.position,
    };
    const record = await invoke<DynamicUIPinRecord>("pin_dynamic_ui_schema", {
      schema_id: merged.schemaId,
      title: merged.title,
      group_name: merged.groupName,
      position: merged.position,
    });
    set((state) => {
      const others = state.pins.filter((p) => p.schemaId !== record.schemaId);
      return { pins: [...others, record] };
    });
  },

  saveFormData: async (params) => {
    const record = await invoke<DynamicUIFormDataRecord>("save_dynamic_ui_form_data", {
      req: params,
    });
    try {
      const data = JSON.parse(params.formDataJson) as Record<string, unknown>;
      const cacheKey = formCacheKey(params.schemaId, params.instanceKey);
      set((state) => {
        const newCache = new Map(state.formDataCache);
        newCache.set(cacheKey, data);
        return { formDataCache: newCache };
      });
    } catch {
      // ignore parse errors
    }
    return record;
  },

  loadFormData: async (schemaId, instanceKey) => {
    const cacheKey = formCacheKey(schemaId, instanceKey);
    const cached = get().formDataCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const record = await invoke<DynamicUIFormDataRecord | null>("get_dynamic_ui_form_data", {
      schema_id: schemaId,
      instance_key: instanceKey || null,
    });
    if (!record) {
      return null;
    }
    try {
      const data = JSON.parse(record.formDataJson) as Record<string, unknown>;
      set((state) => {
        const newCache = new Map(state.formDataCache);
        newCache.set(cacheKey, data);
        return { formDataCache: newCache };
      });
      return data;
    } catch {
      return null;
    }
  },

  clearFormData: async (schemaId, instanceKey) => {
    await invoke<void>("delete_dynamic_ui_form_data", {
      schema_id: schemaId,
      instance_key: instanceKey || null,
    });
    const cacheKey = formCacheKey(schemaId, instanceKey);
    set((state) => {
      const newCache = new Map(state.formDataCache);
      newCache.delete(cacheKey);
      return { formDataCache: newCache };
    });
  },

  setCurrentSchema: (schema) => set({ currentSchema: schema }),

  // ── 版本管理 ──

  loadVersions: async (schemaId) => {
    set({ versionLoading: true });
    try {
      const result = await invoke<ListVersionsResponse>("list_dynamic_ui_schema_versions", {
        schema_id: schemaId,
      });
      set({ versionList: result.versions, versionLoading: false });
      return result.versions;
    } catch (e) {
      // DUI-P2-01: 版本列表加载失败需记录日志，否则 UI 显示空状态无法区分"无数据"还是"加载失败"
      console.error("loadVersions failed:", e);
      set({ versionLoading: false });
      return [];
    }
  },

  getVersion: async (versionId) => {
    try {
      return await invoke<DynamicUISchemaVersion>("get_dynamic_ui_schema_version", {
        version_id: versionId,
      });
    } catch (e) {
      console.error("getVersion failed:", e);
      return null;
    }
  },

  restoreVersion: async (schemaId, versionId) => {
    try {
      const updated = await invoke<DynamicUISchemaRecord>("restore_dynamic_ui_schema_version", {
        schema_id: schemaId,
        version_id: versionId,
      });
      set((state) => ({
        schemas: state.schemas.map((s) => (s.id === schemaId ? updated : s)),
        currentSchema: state.currentSchema?.id === schemaId ? updated : state.currentSchema,
      }));
      return updated;
    } catch (e) {
      console.error("restoreVersion failed:", e);
      return null;
    }
  },
}));
