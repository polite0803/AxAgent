import { invoke } from "@/lib/invoke";
import { create } from "zustand";

export interface ParallelExecution {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  tasks: ParallelTask[];
  created_at: string;
  completed_at?: string;
}

export interface ParallelTask {
  id: string;
  execution_id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed" | "cancelled";
  result?: string;
  error?: string;
  started_at?: string;
  completed_at?: string;
}

interface ParallelExecutionStore {
  executions: ParallelExecution[];
  loading: boolean;

  listExecutions: () => Promise<void>;
  getExecution: (id: string) => Promise<ParallelExecution | null>;
  createExecution: (name: string, tasks: { name: string }[]) => Promise<string>;
  startExecution: (id: string) => Promise<void>;
  cancelExecution: (id: string) => Promise<void>;
  deleteExecution: (id: string) => Promise<void>;
  getNextPendingTask: (executionId: string) => Promise<ParallelTask | null>;
  updateTaskResult: (executionId: string, taskId: string, result: string) => Promise<void>;
  updateTaskError: (executionId: string, taskId: string, error: string) => Promise<void>;
  getResult: (executionId: string) => Promise<unknown>;
}

export const useParallelExecutionStore = create<ParallelExecutionStore>((set, get) => ({
  executions: [],
  loading: false,

  listExecutions: async () => {
    set({ loading: true });
    try {
      const list = await invoke<ParallelExecution[]>("list_parallel_executions");
      set({ executions: list });
    } catch (e) {
      console.warn("Failed to list parallel executions:", e);
    } finally {
      set({ loading: false });
    }
  },

  getExecution: async (id) => {
    try {
      return await invoke<ParallelExecution>("get_parallel_execution", { executionId: id });
    } catch (e) {
      console.warn("Failed to get execution:", e);
      return null;
    }
  },

  createExecution: async (name, tasks) => {
    const id = await invoke<string>("create_parallel_execution", { name, tasks });
    await get().listExecutions();
    return id;
  },

  startExecution: async (id) => {
    await invoke("start_parallel_execution", { executionId: id });
    await get().listExecutions();
  },

  cancelExecution: async (id) => {
    await invoke("cancel_parallel_execution", { executionId: id });
    await get().listExecutions();
  },

  deleteExecution: async (id) => {
    await invoke("delete_parallel_execution", { executionId: id });
    set((s) => ({ executions: s.executions.filter((e) => e.id !== id) }));
  },

  getNextPendingTask: async (executionId) => {
    return await invoke<ParallelTask | null>("get_next_pending_task", { executionId });
  },

  updateTaskResult: async (executionId, taskId, result) => {
    await invoke("update_task_result", { executionId, taskId, result });
  },

  updateTaskError: async (executionId, taskId, error) => {
    await invoke("update_task_error", { executionId, taskId, error });
  },

  getResult: async (executionId) => {
    return await invoke("get_execution_result", { executionId });
  },
}));
