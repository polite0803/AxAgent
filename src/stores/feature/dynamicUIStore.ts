// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type {
  CreateDynamicUISchemaParams,
  DynamicUIFormDataRecord,
  DynamicUISchemaRecord,
  SaveDynamicUIFormDataParams,
  UpdateDynamicUISchemaParams,
} from "@/types";
import { create } from "zustand";

interface DynamicUIState {
  schemas: DynamicUISchemaRecord[];
  loading: boolean;
  currentSchema: DynamicUISchemaRecord | null;
  formDataCache: Map<string, Record<string, unknown>>;

  fetchSchemas: (category?: string) => Promise<void>;
  getSchema: (id: string) => Promise<DynamicUISchemaRecord>;
  createSchema: (params: CreateDynamicUISchemaParams) => Promise<DynamicUISchemaRecord>;
  updateSchema: (id: string, params: UpdateDynamicUISchemaParams) => Promise<DynamicUISchemaRecord>;
  deleteSchema: (id: string) => Promise<void>;

  saveFormData: (params: SaveDynamicUIFormDataParams) => Promise<DynamicUIFormDataRecord>;
  loadFormData: (schemaId: string, instanceKey?: string) => Promise<Record<string, unknown> | null>;
  clearFormData: (schemaId: string, instanceKey?: string) => Promise<void>;

  setCurrentSchema: (schema: DynamicUISchemaRecord | null) => void;
}

export const useDynamicUIStore = create<DynamicUIState>((set, get) => ({
  schemas: [],
  loading: false,
  currentSchema: null,
  formDataCache: new Map(),

  fetchSchemas: async (category) => {
    set({ loading: true });
    try {
      const schemas = await invoke<DynamicUISchemaRecord[]>("list_dynamic_ui_schemas", {
        category: category || null,
      });
      set({ schemas, loading: false });
    } catch {
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
    return schema;
  },

  deleteSchema: async (id) => {
    await invoke<void>("delete_dynamic_ui_schema", { id });
    set((state) => ({
      schemas: state.schemas.filter((s) => s.id !== id),
      currentSchema: state.currentSchema?.id === id ? null : state.currentSchema,
    }));
  },

  saveFormData: async (params) => {
    const record = await invoke<DynamicUIFormDataRecord>("save_dynamic_ui_form_data", {
      req: params,
    });
    try {
      const data = JSON.parse(params.form_data_json) as Record<string, unknown>;
      const cacheKey = `${params.schema_id}:${params.instance_key || "default"}`;
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
    const cacheKey = `${schemaId}:${instanceKey || "default"}`;
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
      const data = JSON.parse(record.form_data_json) as Record<string, unknown>;
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
    const cacheKey = `${schemaId}:${instanceKey || "default"}`;
    set((state) => {
      const newCache = new Map(state.formDataCache);
      newCache.delete(cacheKey);
      return { formDataCache: newCache };
    });
  },

  setCurrentSchema: (schema) => set({ currentSchema: schema }),
}));
