// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { GeneratedToolInfo, LocalToolGroupInfo } from "@/types";
import { create } from "zustand";

interface LocalToolState {
  groups: LocalToolGroupInfo[];
  loading: boolean;
  error: string | null;

  // --- Generated tools (merged from generatedToolStore) ---
  tools: GeneratedToolInfo[];

  loadTools: () => Promise<void>;
  deleteTool: (id: string) => Promise<void>;

  // --- Local tool groups ---
  loadGroups: () => Promise<void>;
  toggleGroup: (groupId: string) => Promise<void>;
  toggleTool: (toolName: string) => Promise<void>;
}

export const useLocalToolStore = create<LocalToolState>((set) => ({
  groups: [],
  loading: false,
  error: null,

  tools: [],
  loadTools: async () => {
    set({ loading: true });
    try {
      const tools = await invoke<GeneratedToolInfo[]>("list_generated_tools");
      set({ tools, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  deleteTool: async (id: string) => {
    try {
      await invoke<boolean>("delete_generated_tool", { id });
      set((s) => ({
        tools: s.tools.filter((t) => t.id !== id),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },,

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
      const updatedGroup = await invoke<LocalToolGroupInfo>(
        "toggle_local_tool_group",
        { groupId },
      );
      set((s) => ({
        groups: s.groups.map((g) => (g.groupId === groupId ? updatedGroup : g)),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },

  toggleTool: async (toolName: string) => {
    try {
      const updatedGroups = await invoke<LocalToolGroupInfo[]>(
        "toggle_single_tool",
        { toolName },
      );
      set({ groups: updatedGroups, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
