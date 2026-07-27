// SPDX-License-Identifier: AGPL-3.0-only

// 阅读列表 Store
// 对接后端 commands::reading_list 模块

import { invoke } from "@/lib/invoke";
import type {
  CreateReadingListInput,
  CreateReadingListItemInput,
  ReadingList,
  ReadingListItem,
  UpdateReadingListInput,
  UpdateReadingListItemInput,
} from "@/types";
import { create } from "zustand";

interface ReadingListState {
  /** 全部阅读列表 */
  lists: ReadingList[];
  /** 当前选中列表下的条目 */
  items: ReadingListItem[];
  /** 当前选中的列表 ID */
  selectedListId: string | null;
  loading: boolean;
  error: string | null;

  /** 加载全部阅读列表 */
  loadLists: () => Promise<void>;
  /** 新建阅读列表 */
  createList: (input: CreateReadingListInput) => Promise<ReadingList | null>;
  /** 更新阅读列表 */
  updateList: (id: string, input: UpdateReadingListInput) => Promise<ReadingList | null>;
  /** 删除阅读列表（同时清理本地条目缓存） */
  deleteList: (id: string) => Promise<void>;
  /** 重排阅读列表顺序 */
  reorderLists: (ids: string[]) => Promise<void>;
  /** 选中某个列表（同时清空当前条目缓存，由调用方决定是否触发 loadItems） */
  setSelectedList: (id: string | null) => void;
  /** 加载指定列表下的条目 */
  loadItems: (listId: string) => Promise<void>;
  /** 新建阅读条目 */
  createItem: (input: CreateReadingListItemInput) => Promise<ReadingListItem | null>;
  /** 更新阅读条目 */
  updateItem: (id: string, input: UpdateReadingListItemInput) => Promise<ReadingListItem | null>;
  /** 删除阅读条目 */
  deleteItem: (id: string) => Promise<void>;
  /** 修改阅读条目状态（unread / reading / read / skipped） */
  setItemStatus: (id: string, status: string) => Promise<ReadingListItem | null>;
  /** 重排指定列表下的条目顺序 */
  reorderItems: (listId: string, ids: string[]) => Promise<void>;
  /** 清空错误状态 */
  clearError: () => void;
}

export const useReadingListStore = create<ReadingListState>((set, get) => ({
  lists: [],
  items: [],
  selectedListId: null,
  loading: false,
  error: null,

  loadLists: async () => {
    set({ loading: true, error: null });
    try {
      const lists = await invoke<ReadingList[]>("list_reading_lists");
      set({ lists: Array.isArray(lists) ? lists : [], loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createList: async (input) => {
    try {
      const list = await invoke<ReadingList>("create_reading_list", { input });
      set((s) => ({ lists: [...s.lists, list], error: null }));
      return list;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateList: async (id, input) => {
    try {
      const list = await invoke<ReadingList>("update_reading_list", { id, input });
      set((s) => ({
        lists: s.lists.map((l) => (l.id === id ? list : l)),
        error: null,
      }));
      return list;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteList: async (id) => {
    try {
      await invoke("delete_reading_list", { id });
      set((s) => {
        const isCurrent = s.selectedListId === id;
        return {
          lists: s.lists.filter((l) => l.id !== id),
          // 删除的是当前选中列表时，同步清空条目与选中状态
          ...(isCurrent ? { items: [], selectedListId: null } : {}),
          error: null,
        };
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  reorderLists: async (ids) => {
    try {
      await invoke("reorder_reading_lists", { ids });
      // 按传入顺序重排本地 sortOrder
      set((s) => {
        const ordered = ids.flatMap((id, i) => {
          const l = s.lists.find((x) => x.id === id);
          return l ? [{ ...l, sortOrder: i }] : [];
        }) as ReadingList[];
        return { lists: ordered };
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  setSelectedList: (id) => {
    set({ selectedListId: id, items: id === null ? [] : get().items });
  },

  loadItems: async (listId) => {
    set({ loading: true, error: null });
    try {
      const items = await invoke<ReadingListItem[]>("list_reading_list_items", { readingListId: listId });
      set({ items: Array.isArray(items) ? items : [], loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  createItem: async (input) => {
    try {
      const item = await invoke<ReadingListItem>("create_reading_list_item", { input });
      set((s) => ({ items: [...s.items, item], error: null }));
      return item;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  updateItem: async (id, input) => {
    try {
      const item = await invoke<ReadingListItem>("update_reading_list_item", { id, input });
      set((s) => ({
        items: s.items.map((it) => (it.id === id ? item : it)),
        error: null,
      }));
      return item;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  deleteItem: async (id) => {
    try {
      await invoke("delete_reading_list_item", { id });
      set((s) => ({ items: s.items.filter((it) => it.id !== id), error: null }));
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  setItemStatus: async (id, status) => {
    try {
      const item = await invoke<ReadingListItem>("set_reading_list_item_status", { id, status });
      set((s) => ({
        items: s.items.map((it) => (it.id === id ? item : it)),
        error: null,
      }));
      return item;
    } catch (e) {
      set({ error: String(e) });
      return null;
    }
  },

  reorderItems: async (listId, ids) => {
    try {
      await invoke("reorder_reading_list_items", { readingListId: listId, ids });
      set((s) => {
        const ordered = ids.flatMap((id, i) => {
          const it = s.items.find((x) => x.id === id);
          return it ? [{ ...it, position: i }] : [];
        }) as ReadingListItem[];
        return { items: ordered };
      });
    } catch (e) {
      set({ error: String(e) });
      throw e;
    }
  },

  clearError: () => {
    set({ error: null });
  },
}));
