// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { MemoryItem, MemoryNamespace, UpdateMemoryItemInput, UpdateMemoryNamespaceInput } from "@/types";
import { create } from "zustand";

interface MemoryState {
  namespaces: MemoryNamespace[];
  items: MemoryItem[];
  loading: boolean;
  error: string | null;
  selectedNamespaceId: string | null;

  loadNamespaces: () => Promise<void>;
  createNamespace: (
    name: string,
    scope: string,
    embeddingProvider?: string,
  ) => Promise<MemoryNamespace | null>;
  deleteNamespace: (id: string) => Promise<void>;
  updateNamespace: (
    id: string,
    input: UpdateMemoryNamespaceInput,
  ) => Promise<void>;
  loadItems: (namespaceId: string) => Promise<void>;
  addItem: (
    namespaceId: string,
    title: string,
    content: string,
  ) => Promise<void>;
  deleteItem: (namespaceId: string, itemId: string) => Promise<void>;
  updateItem: (
    namespaceId: string,
    itemId: string,
    input: UpdateMemoryItemInput,
  ) => Promise<void>;
  // 三层记忆系统：晋升 / 降级 / 记录访问（可能触发自动晋升）
  promoteItem: (namespaceId: string, itemId: string) => Promise<void>;
  demoteItem: (namespaceId: string, itemId: string) => Promise<void>;
  recordItemAccess: (namespaceId: string, itemId: string) => Promise<void>;
  // v108: 自进化闭环 — 确认记忆项（晋升 core 层的确认门）
  confirmItem: (namespaceId: string, itemId: string) => Promise<void>;
  // v108: 自进化闭环 — DB memory 导出到文件级 ProjectMemory
  exportMemoriesToProject: () => Promise<{ exported: number; totalCandidates: number }>;
  setSelectedNamespaceId: (id: string | null) => void;
  reorderNamespaces: (namespaceIds: string[]) => Promise<void>;
}

export const useMemoryStore = create<MemoryState>((set, get) => ({
  namespaces: [],
  items: [],
  loading: false,
  error: null,
  selectedNamespaceId: null,

  loadNamespaces: async () => {
    set({ loading: true });
    try {
      const namespaces = await invoke<MemoryNamespace[]>(
        "list_memory_namespaces",
      );
      set({ namespaces: Array.isArray(namespaces) ? namespaces : [], loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createNamespace: async (name, scope, embeddingProvider) => {
    try {
      const ns = await invoke<MemoryNamespace>("create_memory_namespace", {
        input: { name, scope, embeddingProvider },
      });
      set((s) => ({ namespaces: [...s.namespaces, ns], error: null }));
      return ns;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteNamespace: async (id) => {
    try {
      await invoke("delete_memory_namespace", { id });
      set((s) => {
        const isCurrentSelected = s.selectedNamespaceId === id;
        return {
          namespaces: s.namespaces.filter((n) => n.id !== id),
          // 如果删除的是当前选中的 namespace，清空 items 和选中状态
          ...(isCurrentSelected ? { items: [], selectedNamespaceId: null } : {}),
          error: null,
        };
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateNamespace: async (id, input) => {
    try {
      const updated = await invoke<MemoryNamespace>("update_memory_namespace", {
        id,
        input,
      });
      set((s) => ({
        namespaces: s.namespaces.map((n) => (n.id === id ? updated : n)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  loadItems: async (namespaceId) => {
    set({ loading: true });
    try {
      const items = await invoke<MemoryItem[]>("list_memory_items", {
        namespaceId: namespaceId,
      });
      set({ items, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  addItem: async (namespaceId, title, content) => {
    try {
      await invoke("add_memory_item", {
        input: { namespace_id: namespaceId, title, content },
      });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  deleteItem: async (namespaceId, itemId) => {
    try {
      await invoke("delete_memory_item", { namespaceId, id: itemId });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  updateItem: async (namespaceId, itemId, input) => {
    try {
      await invoke<MemoryItem>("update_memory_item", {
        namespaceId,
        id: itemId,
        input,
      });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  // 三层记忆系统：晋升 / 降级 / 记录访问（可能触发自动晋升）
  promoteItem: async (namespaceId, itemId) => {
    try {
      await invoke<MemoryItem>("promote_user_memory_item", { itemId });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  demoteItem: async (namespaceId, itemId) => {
    try {
      await invoke<MemoryItem>("demote_user_memory_item", { itemId });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  recordItemAccess: async (namespaceId, itemId) => {
    try {
      await invoke<MemoryItem>("record_user_memory_access", { itemId });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  // v108: 自进化闭环 — 确认记忆项
  confirmItem: async (namespaceId, itemId) => {
    try {
      await invoke<MemoryItem>("confirm_memory_item", { itemId });
      await get().loadItems(namespaceId);
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  // v108: 自进化闭环 — DB memory 导出到文件级 ProjectMemory
  exportMemoriesToProject: async () => {
    const result = await invoke<{ exported: number; totalCandidates: number }>(
      "export_memories_to_project",
    );
    return result;
  },

  setSelectedNamespaceId: (id) => {
    set({ selectedNamespaceId: id });
  },

  reorderNamespaces: async (namespaceIds) => {
    await invoke("reorder_memory_namespaces", { namespaceIds });
    set((s) => {
      const ordered = namespaceIds.flatMap((id, i) => {
        const n = s.namespaces.find((n) => n.id === id);
        return n ? [{ ...n, sortOrder: i }] : [];
      }) as MemoryNamespace[];
      return { namespaces: ordered };
    });
  },
}));
