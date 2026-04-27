import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { invoke } from "../../lib/invoke";
import type { ExecutionStatusResponse, ExecutionSummary } from "../../types";

interface WorkEngineState {
  executionId: string | null;
  status: ExecutionStatusResponse | null;
  nodeStatuses: Record<string, string>;
  executionHistory: ExecutionSummary[];
  loading: boolean;

  startExecution: (workflowId: string, input: unknown) => Promise<string>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  loadHistory: (workflowId: string) => Promise<void>;
  getStatus: (executionId: string) => Promise<void>;
  setupEventListeners: () => Promise<() => void>;
}

export const useWorkEngineStore = create<WorkEngineState>((set, get) => ({
  executionId: null,
  status: null,
  nodeStatuses: {},
  executionHistory: [],
  loading: false,

  startExecution: async (workflowId: string, input: unknown) => {
    set({ loading: true });
    try {
      const executionId = await invoke<string>("start_workflow_execution", {
        workflow_id: workflowId,
        input,
      });
      set({ executionId });
      return executionId;
    } finally {
      set({ loading: false });
    }
  },

  pause: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("pause_workflow_execution", { execution_id: executionId });
  },

  resume: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("resume_workflow_execution", { execution_id: executionId });
  },

  cancel: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("cancel_workflow_execution", { execution_id: executionId });
  },

  loadHistory: async (workflowId: string) => {
    const history = await invoke<ExecutionSummary[]>("list_workflow_executions", {
      workflow_id: workflowId,
    });
    set({ executionHistory: history });
  },

  getStatus: async (executionId: string) => {
    const status = await invoke<ExecutionStatusResponse>("get_workflow_execution_status", {
      execution_id: executionId,
    });
    set({ status });
  },

  setupEventListeners: async () => {
    const unlistenNode = await listen("workflow:node-status-changed", (event) => {
      const payload = event.payload as { execution_id: string; node_id: string; status: string };
      set((state) => ({
        nodeStatuses: { ...state.nodeStatuses, [payload.node_id]: payload.status },
      }));
    });

    const unlistenCompleted = await listen("workflow:execution-completed", (event) => {
      const payload = event.payload as { execution_id: string; status: string; total_time_ms: number };
      const { status } = get();
      if (status && status.execution_id === payload.execution_id) {
        set({
          status: {
            ...status,
            status: payload.status as ExecutionStatusResponse["status"],
            total_time_ms: payload.total_time_ms,
          },
        });
      }
    });

    return () => {
      unlistenNode();
      unlistenCompleted();
    };
  },
}));
