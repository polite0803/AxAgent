import { invoke } from "@/lib/invoke";
import type { LocalToolGroupInfo } from "@/types/localTool";
import { create } from "zustand";

interface LocalToolState {
  groups: LocalToolGroupInfo[];
  loading: boolean;
  error: string | null;

  loadGroups: () => Promise<void>;
  toggleGroup: (groupId: string) => Promise<void>;
}

export const useLocalToolStore = create<LocalToolState>((set) => ({
  groups: [],
  loading: false,
  error: null,

  loadGroups: async () => {
    set({ loading: true });
    try {
      const groups = await invoke<LocalToolGroupInfo[]>("list_local_tools");
      set({ groups, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  toggleGroup: async (groupId: string) => {
    try {
      const updatedGroup = await invoke<LocalToolGroupInfo>("toggle_local_tool", { groupId });
      set((s) => ({
        groups: s.groups.map((g) => g.groupId === groupId ? updatedGroup : g),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
