// SPDX-License-Identifier: AGPL-3.0-only

import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@/lib/invoke", () => ({
  invoke: invokeMock,
  listen: vi.fn(() => Promise.resolve(() => {})),
  isTauri: () => false,
  logIpcError: vi.fn(() => vi.fn()),
}));

vi.mock("@/lib/toast", () => ({
  message: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
    warning: vi.fn(),
    loading: vi.fn(),
    open: vi.fn(),
    destroy: vi.fn(),
  },
}));

import { usePlanStore } from "@/stores/feature/planStore";
import type { Plan } from "@/types";

const CONV_ID = "conv-1";
const PLAN_ID = "plan-1";

function makePlan(overrides?: Partial<Plan>): Plan {
  return {
    id: PLAN_ID,
    conversationId: CONV_ID,
    userMessageId: "msg-1",
    title: "Test Plan",
    status: "draft",
    steps: [
      { id: "step-1", title: "Step 1", description: "First step", status: "pending", result: null },
      { id: "step-2", title: "Step 2", description: "Second step", status: "pending", result: null },
      { id: "step-3", title: "Step 3", description: "Third step", status: "pending", result: null },
    ],
    isActive: true,
    createdAt: 1735689600000,
    updatedAt: 1735689600000,
    ...overrides,
  };
}

describe("planStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    usePlanStore.setState({
      activePlans: {},
      planHistory: {},
      loading: {},
      errors: {},
    });
  });

  describe("generatePlan", () => {
    it("generates a plan and stores it in activePlans", async () => {
      const plan = makePlan();
      invokeMock.mockResolvedValueOnce(plan);

      const result = await usePlanStore.getState().generatePlan(CONV_ID, "Build a todo app");

      expect(invokeMock).toHaveBeenCalledWith("plan_generate", {
        request: { conversationId: CONV_ID, content: "Build a todo app" },
      });
      expect(result).toEqual(plan);
      expect(usePlanStore.getState().activePlans[CONV_ID]).toEqual(plan);
      expect(usePlanStore.getState().loading[CONV_ID]).toBe(false);
    });

    it("sets loading to true while generating", () => {
      invokeMock.mockResolvedValueOnce(makePlan());
      const promise = usePlanStore.getState().generatePlan(CONV_ID, "hello");
      expect(usePlanStore.getState().loading[CONV_ID]).toBe(true);
      return promise;
    });

    it("clears previous error on new generation", async () => {
      usePlanStore.setState({ errors: { [CONV_ID]: "old error" } });
      invokeMock.mockResolvedValueOnce(makePlan());

      await usePlanStore.getState().generatePlan(CONV_ID, "hello");

      expect(usePlanStore.getState().errors[CONV_ID]).toBeNull();
    });

    it("sets error state on failure", async () => {
      invokeMock.mockRejectedValueOnce(new Error("API error"));

      await expect(
        usePlanStore.getState().generatePlan(CONV_ID, "hello"),
      ).rejects.toThrow("API error");

      expect(usePlanStore.getState().loading[CONV_ID]).toBe(false);
      expect(usePlanStore.getState().errors[CONV_ID]).toBe("Error: API error");
    });
  });

  describe("approvePlan", () => {
    it("approves all pending steps and executes the plan", async () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      invokeMock.mockResolvedValue(undefined); // plan_modify_step x3
      invokeMock.mockResolvedValue(undefined); // plan_execute

      await usePlanStore.getState().approvePlan(CONV_ID, PLAN_ID);

      // Should approve all 3 pending steps
      expect(invokeMock).toHaveBeenCalledWith("plan_modify_step", {
        request: { planId: PLAN_ID, stepId: "step-1", approved: true },
      });
      expect(invokeMock).toHaveBeenCalledWith("plan_modify_step", {
        request: { planId: PLAN_ID, stepId: "step-2", approved: true },
      });
      expect(invokeMock).toHaveBeenCalledWith("plan_modify_step", {
        request: { planId: PLAN_ID, stepId: "step-3", approved: true },
      });

      // Should execute with all step IDs
      expect(invokeMock).toHaveBeenCalledWith("plan_execute", {
        request: {
          conversationId: CONV_ID,
          planId: PLAN_ID,
          stepIds: ["step-1", "step-2", "step-3"],
        },
      }, 0);

      const activePlan = usePlanStore.getState().activePlans[CONV_ID];
      expect(activePlan?.status).toBe("executing");
      expect(activePlan?.steps.every((s: { status: string }) => s.status === "approved")).toBe(true);
    });

    it("handles error during approve", async () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      // plan_modify_step 的失败会被逐个吞掉（仅告警），真正会冒泡的是 plan_execute
      invokeMock.mockImplementation(async (cmd: string) => {
        if (cmd === "plan_execute") {
          throw new Error("execute failed");
        }
        return undefined;
      });

      await usePlanStore.getState().approvePlan(CONV_ID, PLAN_ID);

      expect(usePlanStore.getState().errors[CONV_ID]).toBe("Error: execute failed");
    });
  });

  describe("rejectPlan", () => {
    it("cancels the plan and moves it to history", async () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      invokeMock.mockResolvedValueOnce(undefined);

      await usePlanStore.getState().rejectPlan(CONV_ID, PLAN_ID, "Not ready");

      expect(invokeMock).toHaveBeenCalledWith("plan_cancel", {
        request: {
          conversationId: CONV_ID,
          planId: PLAN_ID,
          reason: "Not ready",
        },
      });

      expect(usePlanStore.getState().activePlans[CONV_ID]).toBeUndefined();
      const history = usePlanStore.getState().planHistory[CONV_ID];
      expect(history).toHaveLength(1);
      expect(history[0].status).toBe("cancelled");
    });
  });

  describe("modifyStep", () => {
    it("modifies a step and optimistically updates state", async () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      invokeMock.mockResolvedValueOnce(undefined);

      await usePlanStore.getState().modifyStep(CONV_ID, PLAN_ID, "step-1", {
        approved: true,
        title: "Updated Step 1",
      });

      expect(invokeMock).toHaveBeenCalledWith("plan_modify_step", {
        request: {
          planId: PLAN_ID,
          stepId: "step-1",
          approved: true,
          title: "Updated Step 1",
        },
      });

      const updatedPlan = usePlanStore.getState().activePlans[CONV_ID];
      const step1 = updatedPlan?.steps.find((s: { id: string }) => s.id === "step-1");
      expect(step1?.status).toBe("approved");
      expect(step1?.title).toBe("Updated Step 1");
    });
  });

  describe("executePlan", () => {
    it("executes a plan with specific step IDs", async () => {
      invokeMock.mockResolvedValueOnce(undefined);

      await usePlanStore.getState().executePlan(CONV_ID, PLAN_ID, ["step-1", "step-2"]);

      expect(invokeMock).toHaveBeenCalledWith("plan_execute", {
        request: {
          conversationId: CONV_ID,
          planId: PLAN_ID,
          stepIds: ["step-1", "step-2"],
        },
      }, 0);

      expect(usePlanStore.getState().loading[CONV_ID]).toBe(true);
    });
  });

  describe("resumePlan", () => {
    it("resumes a plan from history", async () => {
      const plan = makePlan({ status: "reviewing" });
      invokeMock.mockResolvedValueOnce(plan);

      await usePlanStore.getState().resumePlan(CONV_ID, PLAN_ID);

      expect(invokeMock).toHaveBeenCalledWith("plan_activate", {
        request: { conversationId: CONV_ID, planId: PLAN_ID },
      });

      expect(usePlanStore.getState().activePlans[CONV_ID]).toEqual(plan);
      expect(usePlanStore.getState().loading[CONV_ID]).toBe(false);
    });
  });

  describe("cancelPlan", () => {
    it("cancels a plan and moves to history", async () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      invokeMock.mockResolvedValueOnce(undefined);

      await usePlanStore.getState().cancelPlan(CONV_ID, PLAN_ID);

      expect(invokeMock).toHaveBeenCalledWith("plan_cancel", {
        request: { conversationId: CONV_ID, planId: PLAN_ID },
      });
      expect(usePlanStore.getState().activePlans[CONV_ID]).toBeUndefined();
    });
  });

  describe("loadPlanHistory", () => {
    it("loads plan history for a conversation", async () => {
      const plans = [makePlan({ status: "completed" }), makePlan({ id: "plan-2", status: "cancelled" })];
      invokeMock.mockResolvedValueOnce(plans);

      await usePlanStore.getState().loadPlanHistory(CONV_ID);

      expect(invokeMock).toHaveBeenCalledWith("plan_list", {
        request: { conversationId: CONV_ID, includeCompleted: true },
      });
      expect(usePlanStore.getState().planHistory[CONV_ID]).toEqual(plans);
    });
  });

  describe("loadActivePlan", () => {
    it("loads the active plan from DB", async () => {
      const activePlan = makePlan({ status: "executing" });
      const inactivePlan = makePlan({ id: "plan-2", status: "completed", isActive: false });
      invokeMock.mockResolvedValueOnce([activePlan, inactivePlan]);

      await usePlanStore.getState().loadActivePlan(CONV_ID);

      expect(usePlanStore.getState().activePlans[CONV_ID]).toEqual(activePlan);
    });

    it("does not set active plan if none is active", async () => {
      invokeMock.mockResolvedValueOnce([]);

      await usePlanStore.getState().loadActivePlan(CONV_ID);

      expect(usePlanStore.getState().activePlans[CONV_ID]).toBeUndefined();
    });
  });

  describe("clearActivePlan", () => {
    it("removes the active plan for a conversation", () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      usePlanStore.getState().clearActivePlan(CONV_ID);

      expect(usePlanStore.getState().activePlans[CONV_ID]).toBeUndefined();
    });
  });

  describe("setLoading / setError", () => {
    it("setLoading updates loading state", () => {
      usePlanStore.getState().setLoading(CONV_ID, true);
      expect(usePlanStore.getState().loading[CONV_ID]).toBe(true);
    });

    it("setError updates error state", () => {
      usePlanStore.getState().setError(CONV_ID, "test error");
      expect(usePlanStore.getState().errors[CONV_ID]).toBe("test error");
    });
  });

  describe("updatePlanStatus", () => {
    it("updates the status of an active plan", () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      usePlanStore.getState().updatePlanStatus(CONV_ID, PLAN_ID, "executing");

      expect(usePlanStore.getState().activePlans[CONV_ID]?.status).toBe("executing");
    });

    it("does nothing if plan is not found", () => {
      usePlanStore.getState().updatePlanStatus(CONV_ID, "nonexistent", "executing");
      expect(usePlanStore.getState().activePlans[CONV_ID]).toBeUndefined();
    });
  });

  describe("handlePlanGenerated", () => {
    it("sets the generated plan as active and archives old plan", () => {
      const oldPlan = makePlan({ id: "old-plan", status: "draft" });
      const newPlan = makePlan({ id: PLAN_ID, status: "reviewing" });
      usePlanStore.setState({
        activePlans: { [CONV_ID]: oldPlan },
      });

      usePlanStore.getState().handlePlanGenerated({
        conversationId: CONV_ID,
        plan: newPlan,
      });

      const state = usePlanStore.getState();
      expect(state.activePlans[CONV_ID]).toEqual(newPlan);
      expect(state.planHistory[CONV_ID]).toHaveLength(1);
      expect(state.planHistory[CONV_ID][0]).toEqual(oldPlan);
      expect(state.loading[CONV_ID]).toBe(false);
    });
  });

  describe("handlePlanStepUpdate", () => {
    it("updates a step status and plan status", () => {
      const plan = makePlan();
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      usePlanStore.getState().handlePlanStepUpdate({
        conversationId: CONV_ID,
        planId: PLAN_ID,
        stepId: "step-1",
        status: "running",
        result: null,
      });

      const updatedPlan = usePlanStore.getState().activePlans[CONV_ID];
      const step1 = updatedPlan?.steps.find((s: { id: string }) => s.id === "step-1");
      expect(step1?.status).toBe("running");
      expect(updatedPlan?.status).toBe("executing");
    });

    it("sets plan status to completed when all steps are done", () => {
      const plan = makePlan({
        steps: [
          { id: "step-1", title: "Step 1", description: "", status: "completed", result: null },
          { id: "step-2", title: "Step 2", description: "", status: "completed", result: null },
          { id: "step-3", title: "Step 3", description: "", status: "pending", result: null },
        ],
      });
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      usePlanStore.getState().handlePlanStepUpdate({
        conversationId: CONV_ID,
        planId: PLAN_ID,
        stepId: "step-3",
        status: "completed",
        result: null,
      });

      expect(usePlanStore.getState().activePlans[CONV_ID]?.status).toBe("completed");
    });

    it("does nothing if plan is not found", () => {
      usePlanStore.getState().handlePlanStepUpdate({
        conversationId: CONV_ID,
        planId: "nonexistent",
        stepId: "step-1",
        status: "completed",
        result: null,
      });
    });
  });

  describe("handlePlanExecutionComplete", () => {
    it("moves the completed plan from active to history", () => {
      const plan = makePlan({ status: "executing" });
      usePlanStore.setState({ activePlans: { [CONV_ID]: plan } });

      usePlanStore.getState().handlePlanExecutionComplete({
        conversationId: CONV_ID,
        planId: PLAN_ID,
        status: "completed",
      });

      const state = usePlanStore.getState();
      expect(state.activePlans[CONV_ID]).toBeUndefined();
      expect(state.planHistory[CONV_ID]).toHaveLength(1);
      expect(state.planHistory[CONV_ID][0].status).toBe("completed");
    });

    it("does nothing if plan is not found", () => {
      usePlanStore.getState().handlePlanExecutionComplete({
        conversationId: CONV_ID,
        planId: "nonexistent",
        status: "completed",
      });
    });
  });
});
