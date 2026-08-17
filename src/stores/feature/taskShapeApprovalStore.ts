// SPDX-License-Identifier: AGPL-3.0-only

import { invoke, listen, logIpcError } from "@/lib/invoke";
import type { UnlistenFn } from "@/lib/invoke";
import { create } from "zustand";

/** P3 后端 emit 的审批请求事件 payload */
export interface TaskShapeApprovalEvent {
  approvalId: string;
  conversationId: string;
  inputPreview: string;
  evidence: string[];
  mergeScore: number;
  splitScore: number;
}

interface TaskShapeApprovalStore {
  /** 当前挂起的审批请求（同时只有一个） */
  pendingApproval: TaskShapeApprovalEvent | null;
  /** 按钮加载态 */
  loading: boolean;

  /** 用户批准 */
  approve: (approvalId: string) => Promise<void>;
  /** 用户拒绝 */
  reject: (approvalId: string) => Promise<void>;
  /** 内部：设置挂起请求 */
  setPendingApproval: (approval: TaskShapeApprovalEvent | null) => void;
}

export const useTaskShapeApprovalStore = create<TaskShapeApprovalStore>(
  (set, _get) => ({
    pendingApproval: null,
    loading: false,

    setPendingApproval: (approval) => set({ pendingApproval: approval }),

    approve: async (approvalId) => {
      set({ loading: true });
      try {
        await invoke("respond_task_shape_approval", {
          request: { approvalId, approved: true },
        });
      } catch (e) {
        logIpcError("taskShapeApprovalStore.approve")(e);
      } finally {
        set({ loading: false, pendingApproval: null });
      }
    },

    reject: async (approvalId) => {
      set({ loading: true });
      try {
        await invoke("respond_task_shape_approval", {
          request: { approvalId, approved: false },
        });
      } catch (e) {
        logIpcError("taskShapeApprovalStore.reject")(e);
      } finally {
        set({ loading: false, pendingApproval: null });
      }
    },
  }),
);

// ── 事件监听 setup（引用计数模式，与 agentStore 一致）──

let _refCount = 0;

export function setupTaskShapeApprovalListeners(): () => void {
  _refCount++;
  if (_refCount > 1) {
    return () => {
      _refCount--;
    };
  }

  const unlisteners: Promise<UnlistenFn>[] = [];
  const store = useTaskShapeApprovalStore.getState();

  unlisteners.push(
    listen<TaskShapeApprovalEvent>("task-shape-approval-request", (event) => {
      store.setPendingApproval(event.payload);
    }),
  );

  return () => {
    _refCount--;
    if (_refCount <= 0) {
      _refCount = 0;
      for (const p of unlisteners) {
        p.then((u) => u());
      }
    }
  };
}
