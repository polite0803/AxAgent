// SPDX-License-Identifier: AGPL-3.0-only

import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { invoke } from "../../lib/invoke";
import type { ExecutionStatus, ExecutionStatusResponse, ExecutionSummary, NodeExecutionRecord } from "../../types";

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
  lastDebugError: string | null;

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
  toggleBreakpoint: (nodeId: string) => Promise<void>;
  setDryRun: (val: boolean) => void;
  loadHistory: (workflowId: string) => Promise<void>;
  getStatus: (executionId: string, replaceStatuses?: boolean) => Promise<void>;
  viewExecution: (executionId: string) => Promise<void>;
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
  lastDebugError: null,

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
    set({ loading: true, nodeStatuses: {}, nodeRecords: [], variables: {}, lastDebugError: null });
    try {
      const executionId = await invoke<string>("debug_run_workflow", {
        templateId: templateId,
        input: options?.input ?? null,
        breakpoints: options?.breakpoints ?? null,
        dry_run: options?.dryRun ?? get().dryRun,
        model_id: options?.modelId ?? null,
        provider_id: options?.providerId ?? null,
      });
      set({ executionId, isDebugRunning: true, lastDebugError: null });
      return executionId;
    } catch (e) {
      const msg = String(e);
      console.error("[debugRun] Failed to start debug:", msg);
      set({ lastDebugError: msg });
      throw e;
    } finally {
      set({ loading: false });
    }
  },

  pause: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    try {
      await invoke<boolean>("pause_workflow_execution", {
        execution_id: executionId,
      });
    } catch (e) {
      console.error("[workEngine] pause failed:", String(e));
      throw e;
    }
  },

  resume: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    try {
      await invoke<boolean>("resume_workflow_execution", {
        execution_id: executionId,
      });
    } catch (e) {
      console.error("[workEngine] resume failed:", String(e));
      throw e;
    }
  },

  cancel: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    try {
      await invoke<boolean>("cancel_workflow_execution", {
        execution_id: executionId,
      });
      set({ isDebugRunning: false });
    } catch (e) {
      console.error("[workEngine] cancel failed:", String(e));
      set({ isDebugRunning: false });
      throw e;
    }
  },

  setBreakpoints: async (nodeIds: string[]) => {
    const { executionId } = get();
    try {
      await invoke<boolean>("set_workflow_breakpoints", {
        node_ids: nodeIds,
        execution_id: executionId ?? null,
      });
      set({ breakpoints: nodeIds });
    } catch (e) {
      console.error("[workEngine] setBreakpoints failed:", String(e));
      throw e;
    }
  },

  resumeBreakpoint: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    try {
      await invoke<boolean>("resume_workflow_breakpoint", {
        execution_id: executionId,
      });
    } catch (e) {
      console.error("[workEngine] resumeBreakpoint failed:", String(e));
      throw e;
    }
  },

  stepBreakpoint: async () => {
    const { executionId } = get();
    if (!executionId) { return; }
    try {
      await invoke<boolean>("step_workflow_breakpoint", {
        execution_id: executionId,
      });
    } catch (e) {
      console.error("[workEngine] stepBreakpoint failed:", String(e));
      throw e;
    }
  },

  toggleBreakpoint: async (nodeId: string) => {
    const { breakpoints, executionId } = get();
    const prev = breakpoints;
    const next = breakpoints.includes(nodeId)
      ? breakpoints.filter((id) => id !== nodeId)
      : [...breakpoints, nodeId];
    set({ breakpoints: next });
    if (get().isDebugRunning) {
      try {
        await invoke<boolean>("set_workflow_breakpoints", {
          node_ids: next,
          execution_id: executionId ?? null,
        });
      } catch (e) {
        console.error("[workEngine] toggleBreakpoint remote sync failed:", String(e));
        set({ breakpoints: prev });
        throw e;
      }
    }
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

  getStatus: async (executionId: string, replaceStatuses?: boolean) => {
    const status = await invoke<ExecutionStatusResponse>(
      "get_workflow_execution_status",
      { execution_id: executionId },
    );
    const nodeStatusesFromRecords: Record<string, string> = {};
    for (const r of status.node_records ?? []) {
      nodeStatusesFromRecords[r.node_id] = r.status;
    }
    set((state) => ({
      status,
      nodeRecords: status.node_records ?? [],
      variables: status.variables ?? {},
      // replaceStatuses=true 时完全替换而非合并，用于查看历史执行时清除旧状态
      nodeStatuses: replaceStatuses
        ? nodeStatusesFromRecords
        : { ...state.nodeStatuses, ...nodeStatusesFromRecords },
    }));
  },

  viewExecution: async (executionId: string) => {
    set({ isDebugRunning: false, loading: true });
    try {
      await get().getStatus(executionId, true);
      set({ executionId, loading: false });
    } catch (e) {
      set({ loading: false });
      console.error("[workEngine] viewExecution failed:", String(e));
      throw e;
    }
  },

  resetDebug: () => {
    set({
      executionId: null,
      status: null,
      nodeStatuses: {},
      nodeRecords: [],
      variables: {},
      isDebugRunning: false,
      lastDebugError: null,
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
          execution_id?: string;
        };
        const { executionId } = get();
        if (payload.execution_id && executionId && payload.execution_id !== executionId) {
          return;
        }
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
          execution_id?: string;
          status: string;
          total_time_ms: number;
          error?: string;
        };
        const { executionId, getStatus } = get();
        if (payload.execution_id && executionId && payload.execution_id !== executionId) {
          return;
        }
        if (executionId) {
          await getStatus(executionId);
        }
        if (
          payload.status === "completed"
          || payload.status === "failed"
          || payload.status === "cancelled"
          || payload.status === "partially_completed"
        ) {
          set({ isDebugRunning: false });
        }
      },
    );

    // ── workflow:state-changed — 全量状态同步，取代 2s 轮询 ──
    const unlistenState = await listen(
      "workflow:state-changed",
      (event) => {
        const payload = event.payload as {
          execution_id?: string;
          workflow_id: string;
          status: string;
          current_node_id?: string;
          total_time_ms: number;
          node_count: number;
          node_records: Array<{
            node_id: string;
            node_type: string;
            node_name?: string;
            status: string;
            input?: unknown;
            output?: unknown;
            execution_time_ms?: number;
            error?: string;
            started_at: number;
            completed_at?: number;
            parent_execution_id?: string;
            sub_workflow_id?: string;
          }>;
          variables?: Record<string, unknown>;
        };
        const { executionId } = get();
        if (payload.execution_id && executionId && payload.execution_id !== executionId) {
          return;
        }

        // 从 node_records 提取 nodeStatuses
        const nodeStatusesFromRecords: Record<string, string> = {};
        for (const r of payload.node_records ?? []) {
          nodeStatusesFromRecords[r.node_id] = r.status;
        }

        set({
          status: {
            execution_id: payload.execution_id ?? "",
            workflow_id: payload.workflow_id,
            status: payload.status as ExecutionStatus,
            current_node_id: payload.current_node_id ?? null,
            total_time_ms: payload.total_time_ms,
            node_count: payload.node_count,
            parent_execution_id: null,
          } as ExecutionStatusResponse,
          nodeRecords: payload.node_records.map((r) => ({
            node_id: r.node_id,
            node_type: r.node_type,
            node_name: r.node_name ?? null,
            status: r.status,
            input: r.input ?? null,
            output: r.output ?? null,
            execution_time_ms: r.execution_time_ms ?? null,
            error: r.error ?? null,
            started_at: r.started_at,
            completed_at: r.completed_at ?? null,
            parent_execution_id: r.parent_execution_id ?? null,
            sub_workflow_id: r.sub_workflow_id ?? null,
          })),
          variables: (payload.variables ?? {}) as Record<string, unknown>,
          nodeStatuses: nodeStatusesFromRecords,
        });
      },
    );

    return () => {
      unlistenNode();
      unlistenCompleted();
      unlistenState();
    };
  },
}));
