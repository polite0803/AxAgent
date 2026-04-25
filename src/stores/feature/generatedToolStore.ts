import { create } from 'zustand';
import { invoke } from '@/lib/invoke';
import type { GeneratedToolInfo } from '@/types';

interface GeneratedToolState {
  tools: GeneratedToolInfo[];
  loading: boolean;
  error: string | null;

  loadTools: () => Promise<void>;
  deleteTool: (id: string) => Promise<void>;
}

export const useGeneratedToolStore = create<GeneratedToolState>((set) => ({
  tools: [],
  loading: false,
  error: null,

  loadTools: async () => {
    set({ loading: true });
    try {
      const tools = await invoke<GeneratedToolInfo[]>('list_generated_tools');
      set({ tools, loading: false, error: null });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  deleteTool: async (id: string) => {
    try {
      await invoke<boolean>('delete_generated_tool', { id });
      set((s) => ({
        tools: s.tools.filter((t) => t.id !== id),
        error: null,
      }));
    } catch (e) {
      set({ error: String(e) });
    }
  },
}));
