import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export type PtySessionStatus = "starting" | "running" | "exited" | "error";

export interface PtySessionInfo {
  id: string;
  status: PtySessionStatus;
  shell?: string;
  cwd?: string;
  rows: number;
  cols: number;
}

export interface TerminalError {
  line_number: number;
  error_type: string;
  message: string;
  context: string[];
}

export interface TerminalAnalysis {
  has_errors: boolean;
  errors: TerminalError[];
  last_exit_code: number | null;
  last_command: string | null;
  summary: string;
}

export interface TerminalSuggestion {
  action: string;
  description: string;
  confidence: number;
}

interface TerminalStoreState {
  sessions: PtySessionInfo[];
  activeSessionId: string | null;
  outputBuffers: Record<string, string[]>;
  analysis: Record<string, TerminalAnalysis>;
  loading: boolean;
  error: string | null;

  createSession: (config?: {
    shell?: string;
    cwd?: string;
    rows?: number;
    cols?: number;
  }) => Promise<string>;
  killSession: (id: string) => Promise<void>;
  removeSession: (id: string) => Promise<void>;
  setActiveSession: (id: string | null) => void;
  writeToSession: (id: string, data: string) => Promise<void>;
  resizeSession: (id: string, rows: number, cols: number) => Promise<void>;
  refreshSessions: () => Promise<void>;
  appendOutput: (id: string, data: string) => void;
  clearOutput: (id: string) => void;
  setAnalysis: (id: string, analysis: TerminalAnalysis) => void;
  analyzeOutput: (id: string) => Promise<TerminalAnalysis>;
  getSuggestions: (id: string) => Promise<TerminalSuggestion[]>;
  clearError: () => void;
}

export const useTerminalStore = create<TerminalStoreState>((set) => ({
  sessions: [],
  activeSessionId: null,
  outputBuffers: {},
  analysis: {},
  loading: false,
  error: null,

  createSession: async (config) => {
    set({ loading: true, error: null });
    try {
      const id = await invoke<string>("pty_create_session", {
        config: {
          shell: config?.shell ?? null,
          cwd: config?.cwd ?? null,
          env: {},
          rows: config?.rows ?? 24,
          cols: config?.cols ?? 80,
        },
      });
      const newSession: PtySessionInfo = {
        id,
        status: "running",
        shell: config?.shell,
        cwd: config?.cwd,
        rows: config?.rows ?? 24,
        cols: config?.cols ?? 80,
      };
      set((state) => ({
        sessions: [...state.sessions, newSession],
        activeSessionId: id,
        outputBuffers: { ...state.outputBuffers, [id]: [] },
        loading: false,
      }));
      return id;
    } catch (e: unknown) {
      set({ error: String(e), loading: false });
      throw e;
    }
  },

  killSession: async (id) => {
    try {
      await invoke("pty_kill_session", { id });
      set((state) => ({
        sessions: state.sessions.map((s) => s.id === id ? { ...s, status: "exited" as PtySessionStatus } : s),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  removeSession: async (id) => {
    try {
      await invoke("pty_remove_session", { id });
      set((state) => {
        const newBuffers = { ...state.outputBuffers };
        delete newBuffers[id];
        const newAnalysis = { ...state.analysis };
        delete newAnalysis[id];
        return {
          sessions: state.sessions.filter((s) => s.id !== id),
          activeSessionId: state.activeSessionId === id ? null : state.activeSessionId,
          outputBuffers: newBuffers,
          analysis: newAnalysis,
        };
      });
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  setActiveSession: (id) => set({ activeSessionId: id }),

  writeToSession: async (id, data) => {
    try {
      await invoke("pty_write", { id, data });
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  resizeSession: async (id, rows, cols) => {
    try {
      await invoke("pty_resize", { id, rows, cols });
      set((state) => ({
        sessions: state.sessions.map((s) => s.id === id ? { ...s, rows, cols } : s),
      }));
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  refreshSessions: async () => {
    try {
      const sessions = await invoke<PtySessionInfo[]>("pty_list_sessions");
      set({ sessions });
    } catch (e: unknown) {
      set({ error: String(e) });
    }
  },

  appendOutput: (id, data) => {
    set((state) => {
      const buffer = state.outputBuffers[id] ?? [];
      const lines = data.split("\n");
      return {
        outputBuffers: {
          ...state.outputBuffers,
          [id]: [...buffer, ...lines].slice(-5000),
        },
      };
    });
  },

  clearOutput: (id) => {
    set((state) => ({
      outputBuffers: { ...state.outputBuffers, [id]: [] },
    }));
  },

  setAnalysis: (id, analysis) => {
    set((state) => ({
      analysis: { ...state.analysis, [id]: analysis },
    }));
  },

  analyzeOutput: async (id) => {
    try {
      const analysis = await invoke<TerminalAnalysis>("pty_analyze_output", {
        id,
      });
      set((state) => ({
        analysis: { ...state.analysis, [id]: analysis },
      }));
      return analysis;
    } catch (e: unknown) {
      set({ error: String(e) });
      throw e;
    }
  },

  getSuggestions: async (id) => {
    try {
      return await invoke<TerminalSuggestion[]>("pty_get_suggestions", { id });
    } catch (e: unknown) {
      set({ error: String(e) });
      return [];
    }
  },

  clearError: () => set({ error: null }),
}));
