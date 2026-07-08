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
    // 2.9: Optimistic removal — remove from UI first, restore on backend failure
    let previousTools: GeneratedToolInfo[] = [];
    set((s) => {
      previousTools = s.tools;
      return {
        tools: s.tools.filter((t) => t.id !== id),
        error: null,
      };
    });
    try {
      await invoke<boolean>("delete_generated_tool", { id });
    } catch (e) {
      // Restore on failure
      set({ tools: previousTools, error: String(e) });
    }
  },

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
      // 2.4: Partial merge strategy instead of full replacement —
      // preserves local state not reflected in the backend response
      set((s) => {
        const existingMap = new Map(s.groups.map((g) => [g.groupId, g]));
        const merged = updatedGroups.map((ug) => {
          const existing = existingMap.get(ug.groupId);
          return existing ? { ...existing, ...ug } : ug;
        });
        // Append groups from existing that are not in the updated response
        for (const [groupId, g] of existingMap) {
          if (!updatedGroups.some((ug) => ug.groupId === groupId)) {
            merged.push(g);
          }
        }
        return { groups: merged, error: null };
      });
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
