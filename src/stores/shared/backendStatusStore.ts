// SPDX-License-Identifier: AGPL-3.0-only

import { isTauri, listen, logIpcError } from "@/lib/invoke";
import { create } from "zustand";

export type BackendTaskStatus = "running" | "completed" | "failed";

export interface BackendTask {
  id: string;
  type: string;
  label: string;
  status: BackendTaskStatus;
  progress?: number;
  detail?: string;
  startedAt: number;
  completedAt?: number;
}

interface BackendStatusState {
  tasks: BackendTask[];
  agentRunning: Record<string, boolean>;

  upsertTask: (task: BackendTask) => void;
  removeTask: (id: string) => void;
  setAgentRunning: (conversationId: string, running: boolean) => void;
  clearCompleted: () => void;
}

export const useBackendStatusStore = create<BackendStatusState>((set) => ({
  tasks: [],
  agentRunning: {},

  upsertTask: (task) => {
    set((state) => {
      const idx = state.tasks.findIndex((t) => t.id === task.id);
      if (idx >= 0) {
        const tasks = [...state.tasks];
        tasks[idx] = task;
        return { tasks };
      }
      return { tasks: [task, ...state.tasks].slice(0, 50) };
    });
  },

  removeTask: (id) => {
    set((state) => ({
      tasks: state.tasks.filter((t) => t.id !== id),
    }));
  },

  setAgentRunning: (conversationId, running) => {
    set((state) => ({
      agentRunning: { ...state.agentRunning, [conversationId]: running },
    }));
  },

  clearCompleted: () => {
    set((state) => ({
      tasks: state.tasks.filter((t) => t.status !== "completed" && t.status !== "failed"),
    }));
  },
}));

let _initialized = false;

export function initBackendStatusListeners() {
  if (_initialized || !isTauri()) { return; }
  _initialized = true;

  listen<{ conversation_id: string }>("agent-started", (event) => {
    useBackendStatusStore.getState().setAgentRunning(event.payload.conversation_id, true);
  }).catch(logIpcError("listen:agent-started"));

  listen<{ conversation_id: string }>("agent-done", (event) => {
    useBackendStatusStore.getState().setAgentRunning(event.payload.conversation_id, false);
  }).catch(logIpcError("listen:agent-done"));

  listen<{ id: string; status: string; conversation_id: string }>("agent-status", (event) => {
    const { conversation_id, status } = event.payload;
    if (status === "cancelled" || status === "error") {
      useBackendStatusStore.getState().setAgentRunning(conversation_id, false);
    }
  }).catch(logIpcError("listen:agent-status"));

  listen<{ knowledge_base_id: string; status: string; progress?: number }>("knowledge-base-updated", (event) => {
    const { knowledge_base_id, status, progress } = event.payload;
    const store = useBackendStatusStore.getState();
    if (status === "indexing") {
      store.upsertTask({
        id: `kb-index-${knowledge_base_id}`,
        type: "knowledge-indexing",
        label: `Indexing knowledge base`,
        status: "running",
        progress,
        startedAt: Date.now(),
      });
    } else {
      store.upsertTask({
        id: `kb-index-${knowledge_base_id}`,
        type: "knowledge-indexing",
        label: `Indexing knowledge base`,
        status: status === "error" ? "failed" : "completed",
        startedAt: Date.now(),
        completedAt: Date.now(),
      });
    }
  }).catch(logIpcError("listen:knowledge-base-updated"));

  listen<{ namespace_id: string }>("memory-rebuild-complete", () => {
    useBackendStatusStore.getState().upsertTask({
      id: "memory-rebuild",
      type: "memory-rebuild",
      label: "Rebuilding memory",
      status: "completed",
      startedAt: Date.now(),
      completedAt: Date.now(),
    });
  }).catch(logIpcError("listen:memory-rebuild-complete"));

  listen<{ wiki_id: string }>("wiki-rebuild-complete", () => {
    useBackendStatusStore.getState().upsertTask({
      id: "wiki-rebuild",
      type: "wiki-rebuild",
      label: "Rebuilding wiki",
      status: "completed",
      startedAt: Date.now(),
      completedAt: Date.now(),
    });
  }).catch(logIpcError("listen:wiki-rebuild-complete"));

  listen<{ knowledge_base_id: string }>("knowledge-rebuild-complete", () => {
    useBackendStatusStore.getState().upsertTask({
      id: "kb-rebuild",
      type: "knowledge-rebuild",
      label: "Rebuilding knowledge base",
      status: "completed",
      startedAt: Date.now(),
      completedAt: Date.now(),
    });
  }).catch(logIpcError("listen:knowledge-rebuild-complete"));

  listen<{ execution_id: string; node_id: string; status: string }>("workflow:node-status-changed", (event) => {
    const { node_id, status } = event.payload;
    const store = useBackendStatusStore.getState();
    if (status === "running") {
      store.upsertTask({
        id: `wf-node-${node_id}`,
        type: "workflow-node",
        label: `Workflow: ${node_id}`,
        status: "running",
        startedAt: Date.now(),
      });
    } else if (status === "completed" || status === "skipped") {
      store.upsertTask({
        id: `wf-node-${node_id}`,
        type: "workflow-node",
        label: `Workflow: ${node_id}`,
        status: "completed",
        startedAt: Date.now(),
        completedAt: Date.now(),
      });
    } else if (status === "error" || status === "failed") {
      store.upsertTask({
        id: `wf-node-${node_id}`,
        type: "workflow-node",
        label: `Workflow: ${node_id}`,
        status: "failed",
        startedAt: Date.now(),
        completedAt: Date.now(),
      });
    }
  }).catch(logIpcError("listen:workflow:node-status-changed"));

  listen<{ execution_id: string; status: string }>("workflow:execution-completed", (event) => {
    const { execution_id, status } = event.payload;
    useBackendStatusStore.getState().upsertTask({
      id: `wf-exec-${execution_id}`,
      type: "workflow-execution",
      label: "Workflow execution",
      status: status === "error" || status === "failed" ? "failed" : "completed",
      startedAt: Date.now(),
      completedAt: Date.now(),
    });
  }).catch(logIpcError("listen:workflow:execution-completed"));
}
