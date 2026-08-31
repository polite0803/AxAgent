// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  isTauri: () => false,
}));

import { useApprovalStore } from "@/stores/feature/approvalStore";
import type { ApprovalRequest } from "@/types";

function makeApproval(overrides?: Partial<ApprovalRequest>): ApprovalRequest {
  return {
    id: "approval-1",
    executionId: "exec-1",
    nodeId: "node-1",
    workflowId: "wf-1",
    title: "Confirm Action",
    message: "Are you sure?",
    status: "pending",
    timeoutAction: "auto_reject",
    timeoutSecs: 0,
    expiresAt: 0,
    createdAt: 0,
    ...overrides,
  };
}

describe("approvalStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useApprovalStore.setState({
      pendingApprovals: [],
      loading: false,
      panelOpen: false,
      error: null,
    });
  });

  describe("setPanelOpen", () => {
    it("opens and closes the approval panel", () => {
      useApprovalStore.getState().setPanelOpen(true);
      expect(useApprovalStore.getState().panelOpen).toBe(true);

      useApprovalStore.getState().setPanelOpen(false);
      expect(useApprovalStore.getState().panelOpen).toBe(false);
    });
  });

  describe("fetchPendingApprovals", () => {
    it("fetches pending approvals", async () => {
      const approvals = [makeApproval(), makeApproval({ id: "approval-2", title: "Second" })];
      invokeMock.mockResolvedValueOnce(approvals);

      await useApprovalStore.getState().fetchPendingApprovals();

      expect(invokeMock).toHaveBeenCalledWith("list_pending_approvals");
      expect(useApprovalStore.getState().pendingApprovals).toEqual(approvals);
      expect(useApprovalStore.getState().loading).toBe(false);
      expect(useApprovalStore.getState().error).toBeNull();
    });

    it("sets loading to true while fetching", () => {
      invokeMock.mockResolvedValueOnce([]);
      const promise = useApprovalStore.getState().fetchPendingApprovals();
      expect(useApprovalStore.getState().loading).toBe(true);
      return promise;
    });

    it("sets error on fetch failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Network error"));

      await useApprovalStore.getState().fetchPendingApprovals();

      expect(useApprovalStore.getState().error).toBe("Error: Network error");
      expect(useApprovalStore.getState().loading).toBe(false);
    });
  });

  describe("resumeApproval", () => {
    it("approves an approval and refreshes list", async () => {
      const approvals = [makeApproval(), makeApproval({ id: "approval-2" })];
      useApprovalStore.setState({ pendingApprovals: approvals });

      invokeMock.mockResolvedValueOnce(true); // resume_approval
      invokeMock.mockResolvedValueOnce([]); // refresh list

      const result = await useApprovalStore.getState().resumeApproval("exec-1", "approval-1", "approved");

      expect(invokeMock).toHaveBeenCalledWith("resume_approval", {
        approvalId: "approval-1",
        decision: "approved",
        decidedBy: "frontend",
        note: null,
      });
      expect(result).toBe(true);
      expect(useApprovalStore.getState().pendingApprovals).toEqual([]);
    });

    it("rejects an approval", async () => {
      invokeMock.mockResolvedValueOnce(true);
      invokeMock.mockResolvedValueOnce([]);

      const result = await useApprovalStore.getState().resumeApproval(
        "exec-1",
        "approval-1",
        "rejected",
        "user-1",
        "Not needed",
      );

      expect(invokeMock).toHaveBeenCalledWith("resume_approval", {
        approvalId: "approval-1",
        decision: "rejected",
        decidedBy: "user-1",
        note: "Not needed",
      });
      expect(result).toBe(true);
    });

    it("returns false on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Failed"));

      const result = await useApprovalStore.getState().resumeApproval("exec-1", "approval-1", "approved");

      expect(result).toBe(false);
      expect(useApprovalStore.getState().error).toBe("Error: Failed");
    });
  });

  describe("cancelApproval", () => {
    it("cancels an approval and refreshes list", async () => {
      invokeMock.mockResolvedValueOnce(true);
      invokeMock.mockResolvedValueOnce([]);

      const result = await useApprovalStore.getState().cancelApproval("exec-1", "approval-1");

      expect(invokeMock).toHaveBeenCalledWith("cancel_approval", { approvalId: "approval-1" });
      expect(result).toBe(true);
    });

    it("returns false on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("Failed"));

      const result = await useApprovalStore.getState().cancelApproval("exec-1", "approval-1");

      expect(result).toBe(false);
      expect(useApprovalStore.getState().error).toBe("Error: Failed");
    });
  });
});
