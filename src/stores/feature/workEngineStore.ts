import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { invoke } from "../../lib/invoke";
import type { ExecutionStatusResponse, ExecutionSummary, NodeExecutionRecord } from "../../types";

interface WorkEngineState {
  executionId: string | null;
  status: ExecutionStatusResponse | null;
  nodeStatuses: Record<string, string>;
  nodeRecords: NodeExecutionRecord[];
  variables: Record<string, unknown>;
  executionHistory: ExecutionSummary[];
  breakpoints: string[];
  loading: boolean;
  dryRun: boolean;
  isDebugRunning: boolean;

  startExecution: (workflowId: string, input: unknown) => Promise<string>;
  debugRun: (
    templateId: string,
    options?: {
      input?: unknown;
      breakpoints?: string[];
      dryRun?: boolean;
      modelId?: string;
      providerId?: string;
    },
  ) => Promise<string>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  setBreakpoints: (nodeIds: string[]) => Promise<void>;
  resumeBreakpoint: () => Promise<void>;
  stepBreakpoint: () => Promise<void>;
  toggleBreakpoint: (nodeId: string) => void;
  setDryRun: (val: boolean) => void;
  loadHistory: (workflowId: string) => Promise<void>;
  getStatus: (executionId: string) => Promise<void>;
  resetDebug: () => void;
  setupEventListeners: () => Promise<() => void>;
}

export const useWorkEngineStore = create<WorkEngineState>((set, get) => ({
  executionId: null,
  status: null,
  nodeStatuses: {},
  nodeRecords: [],
  variables: {},
  executionHistory: [],
  breakpoints: [],
  loading: false,
  dryRun: false,
  isDebugRunning: false,

  startExecution: async (workflowId: string, input: unknown) => {
    set({ loading: true });
    try {
      const executionId = await invoke<string>("start_workflow_execution", {
        workflow_id: workflowId,
        input,
      });
      set({ executionId, isDebugRunning: true });
      return executionId;
    } finally {
      set({ loading: false });
    }
  },

  debugRun: async (
    templateId: string,
    options?: {
      input?: unknown;
      breakpoints?: string[];
      dryRun?: boolean;
      modelId?: string;
      providerId?: string;
    },
  ) => {
    set({ loading: true, nodeStatuses: {}, nodeRecords: [], variables: {} });
    try {
      const executionId = await invoke<string>("debug_run_workflow", {
        template_id: templateId,
        input: options?.input ?? null,
        breakpoints: options?.breakpoints ?? null,
        dry_run: options?.dryRun ?? get().dryRun,
        model_id: options?.modelId ?? null,
        provider_id: options?.providerId ?? null,
      });
      set({ executionId, isDebugRunning: true });
      return executionId;
    } finally {
      set({ loading: false });
    }
  },

  pause: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("pause_workflow_execution", {
      execution_id: executionId,
    });
  },

  resume: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("resume_workflow_execution", {
      execution_id: executionId,
    });
  },

  cancel: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    await invoke<boolean>("cancel_workflow_execution", {
      execution_id: executionId,
    });
    set({ isDebugRunning: false });
  },

  setBreakpoints: async (nodeIds: string[]) => {
    await invoke<boolean>("set_workflow_breakpoints", {
      node_ids: nodeIds,
    });
    set({ breakpoints: nodeIds });
  },

  resumeBreakpoint: async () => {
    await invoke<boolean>("resume_workflow_breakpoint");
  },

  stepBreakpoint: async () => {
    await invoke<boolean>("step_workflow_breakpoint");
  },

  toggleBreakpoint: (nodeId: string) => {
    const { breakpoints } = get();
    const next = breakpoints.includes(nodeId)
      ? breakpoints.filter((id) => id !== nodeId)
      : [...breakpoints, nodeId];
    set({ breakpoints: next });
  },

  setDryRun: (val: boolean) => {
    set({ dryRun: val });
  },

  loadHistory: async (workflowId: string) => {
    const history = await invoke<ExecutionSummary[]>(
      "list_workflow_executions",
      { workflow_id: workflowId },
    );
    set({ executionHistory: history });
  },

  getStatus: async (executionId: string) => {
    const status = await invoke<ExecutionStatusResponse>(
      "get_workflow_execution_status",
      { execution_id: executionId },
    );
    set({
      status,
      nodeRecords: status.node_records ?? [],
      variables: status.variables ?? {},
    });
  },

  resetDebug: () => {
    set({
      executionId: null,
      status: null,
      nodeStatuses: {},
      nodeRecords: [],
      variables: {},
      isDebugRunning: false,
    });
  },

  setupEventListeners: async () => {
    const unlistenNode = await listen(
      "workflow:node-status-changed",
      (event) => {
        const payload = event.payload as {
          node_id: string;
          status: string;
          total_nodes: number;
          completed_nodes: number;
        };
        set((state) => ({
          nodeStatuses: {
            ...state.nodeStatuses,
            [payload.node_id]: payload.status,
          },
        }));
      },
    );

    const unlistenCompleted = await listen(
      "workflow:execution-completed",
      async (event) => {
        const payload = event.payload as {
          workflow_id: string;
          status: string;
          total_time_ms: number;
          error?: string;
        };
        const { executionId, getStatus } = get();
        if (executionId) {
          await getStatus(executionId);
        }
        if (
          payload.status === "completed"
          || payload.status === "failed"
          || payload.status === "cancelled"
        ) {
          set({ isDebugRunning: false });
        }
      },
    );

    return () => {
      unlistenNode();
      unlistenCompleted();
    };
  },
}));
