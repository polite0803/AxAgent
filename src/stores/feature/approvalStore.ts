// SPDX-License-Identifier: AGPL-3.0-only

import { invoke } from "@/lib/invoke";
import type { ApprovalRequest } from "@/types";
import { create } from "zustand";

interface ApprovalStore {
  pendingApprovals: ApprovalRequest[];
  loading: boolean;
  panelOpen: boolean;
  error: string | null;

  setPanelOpen: (open: boolean) => void;
  fetchPendingApprovals: () => Promise<void>;
  resumeApproval: (
    executionId: string,
    approvalId: string,
    decision: string,
    decidedBy?: string,
    note?: string,
  ) => Promise<boolean>;
  cancelApproval: (executionId: string, approvalId: string, cancelledBy?: string) => Promise<boolean>;
}

export const useApprovalStore = create<ApprovalStore>((set, get) => ({
  pendingApprovals: [],
  loading: false,
  panelOpen: false,
  error: null,

  setPanelOpen: (open) => set({ panelOpen: open }),

  fetchPendingApprovals: async () => {
    set({ loading: true, error: null });
    try {
      const result = await invoke<ApprovalRequest[]>("list_pending_approvals");
      set({ pendingApprovals: result, loading: false });
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },

  resumeApproval: async (executionId, approvalId, decision, decidedBy, note) => {
    try {
      const result = await invoke<boolean>("resume_approval", {
        executionId,
        approvalId,
        decision,
        decidedBy: decidedBy ?? "frontend",
        note: note ?? null,
      });
      if (result) {
        // 刷新列表
        get().fetchPendingApprovals();
      }
      return result;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  cancelApproval: async (executionId, approvalId, cancelledBy) => {
    try {
      const result = await invoke<boolean>("cancel_approval", {
        executionId,
        approvalId,
        cancelledBy: cancelledBy ?? "frontend",
      });
      if (result) {
        get().fetchPendingApprovals();
      }
      return result;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },
}));
