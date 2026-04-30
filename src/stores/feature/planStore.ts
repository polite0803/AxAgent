import { invoke, listen, type UnlistenFn } from "@/lib/invoke";
import type {
  Plan,
  PlanGenerateRequest,
  PlanGeneratedEvent,
  PlanExecuteRequest,
  PlanModifyStepRequest,
  PlanStepStatus,
  PlanStepUpdateEvent,
  PlanExecutionCompleteEvent,
} from "@/types";
import { create } from "zustand";

// ── Plan Event Types (frontend-only, derived from backend events) ─────

interface PlanStore {
  // ── State ──────────────────────────────────────────────────────────
  /** Active plan per conversation (only one active plan at a time per conversation) */
  activePlans: Record<string, Plan>;
  /** Completed/cancelled plans for history browsing */
  planHistory: Record<string, Plan[]>;
  /** Loading state per conversation */
  loading: Record<string, boolean>;
  /** Error state per conversation */
  errors: Record<string, string | null>;

  // ── Actions ────────────────────────────────────────────────────────
  /** Generate a plan for a conversation */
  generatePlan: (conversationId: string, content: string) => Promise<Plan>;
  /** Approve all steps in a plan and start execution */
  approvePlan: (conversationId: string, planId: string) => Promise<void>;
  /** Reject a plan entirely */
  rejectPlan: (conversationId: string, planId: string, reason?: string) => Promise<void>;
  /** Modify a single step (approve/reject/edit) */
  modifyStep: (
    conversationId: string,
    planId: string,
    stepId: string,
    modifications: { title?: string; description?: string; approved?: boolean },
  ) => Promise<void>;
  /** Execute an approved plan (or specific steps) */
  executePlan: (conversationId: string, planId: string, stepIds?: string[]) => Promise<void>;
  /** Resume a previously saved plan */
  resumePlan: (conversationId: string, planId: string) => Promise<void>;
  /** Cancel a plan that is in execution */
  cancelPlan: (conversationId: string, planId: string) => Promise<void>;
  /** Load plan history for a conversation */
  loadPlanHistory: (conversationId: string) => Promise<void>;
  /** Clear active plan for a conversation */
  clearActivePlan: (conversationId: string) => void;
  /** Set loading state */
  setLoading: (conversationId: string, loading: boolean) => void;
  /** Set error state */
  setError: (conversationId: string, error: string | null) => void;

  // ── Event Handlers (called from conversationStore) ──────────────────
  handlePlanGenerated: (event: PlanGeneratedEvent) => void;
  handlePlanStepUpdate: (event: PlanStepUpdateEvent) => void;
  handlePlanExecutionComplete: (event: PlanExecutionCompleteEvent) => void;
  updatePlanStatus: (conversationId: string, planId: string, status: Plan["status"]) => void;
}

// ── Store ─────────────────────────────────────────────────────────────

export const usePlanStore = create<PlanStore>((set, get) => ({
  activePlans: {},
  planHistory: {},
  loading: {},
  errors: {},

  // ── Actions ──────────────────────────────────────────────────────────

  generatePlan: async (conversationId, content) => {
    set((s) => ({
      loading: { ...s.loading, [conversationId]: true },
      errors: { ...s.errors, [conversationId]: null },
    }));

    try {
      const request: PlanGenerateRequest = { conversationId, content };
      const plan: Plan = await invoke("plan_generate", { request });
      set((s) => ({
        activePlans: { ...s.activePlans, [conversationId]: plan },
        loading: { ...s.loading, [conversationId]: false },
      }));
      return plan;
    } catch (e) {
      const errMsg = String(e);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
      throw e;
    }
  },

  approvePlan: async (conversationId, planId) => {
    set((s) => ({
      loading: { ...s.loading, [conversationId]: true },
      errors: { ...s.errors, [conversationId]: null },
    }));

    try {
      const request: PlanExecuteRequest = { conversationId, planId };
      await invoke("plan_execute", { request });
      // Plan status will be updated via planStepUpdate / planExecutionComplete events
    } catch (e) {
      const errMsg = String(e);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
    }
  },

  rejectPlan: async (conversationId, planId, reason) => {
    try {
      await invoke("plan_cancel", {
        request: { conversationId, planId, reason: reason || "User rejected the plan" },
      });
      get().clearActivePlan(conversationId);
    } catch (e) {
      console.error("[planStore] rejectPlan failed:", e);
    }
  },

  modifyStep: async (conversationId, planId, stepId, modifications) => {
    try {
      const request: PlanModifyStepRequest = { planId, stepId, ...modifications };
      await invoke("plan_modify_step", { request });

      // Optimistic update
      const plan = get().activePlans[conversationId];
      if (plan && plan.id === planId) {
        const updatedSteps = plan.steps.map((step) => {
          if (step.id === stepId) {
            return {
              ...step,
              ...modifications,
              status: modifications.approved
                ? ("approved" as PlanStepStatus)
                : ("rejected" as PlanStepStatus),
            };
          }
          return step;
        });
        set((s) => ({
          activePlans: {
            ...s.activePlans,
            [conversationId]: { ...plan, steps: updatedSteps },
          },
        }));
      }
    } catch (e) {
      console.error("[planStore] modifyStep failed:", e);
    }
  },

  executePlan: async (conversationId, planId, stepIds) => {
    set((s) => ({
      loading: { ...s.loading, [conversationId]: true },
    }));

    try {
      const request: PlanExecuteRequest = { conversationId, planId, stepIds };
      await invoke("plan_execute", { request });
    } catch (e) {
      const errMsg = String(e);
      set((s) => ({
        loading: { ...s.loading, [conversationId]: false },
        errors: { ...s.errors, [conversationId]: errMsg },
      }));
    }
  },

  resumePlan: async (conversationId, planId) => {
    try {
      const plan: Plan = await invoke("plan_get", { planId });
      if (plan) {
        set((s) => ({
          activePlans: { ...s.activePlans, [conversationId]: plan },
        }));
      }
    } catch (e) {
      console.error("[planStore] resumePlan failed:", e);
    }
  },

  cancelPlan: async (conversationId, planId) => {
    try {
      await invoke("plan_cancel", { request: { conversationId, planId } });
      get().clearActivePlan(conversationId);
    } catch (e) {
      console.error("[planStore] cancelPlan failed:", e);
    }
  },

  loadPlanHistory: async (conversationId) => {
    try {
      const plans: Plan[] = await invoke("plan_list", {
        request: { conversationId, includeCompleted: true },
      });
      set((s) => ({
        planHistory: { ...s.planHistory, [conversationId]: plans },
      }));
    } catch (e) {
      console.error("[planStore] loadPlanHistory failed:", e);
    }
  },

  clearActivePlan: (conversationId) => {
    set((s) => {
      const { [conversationId]: _removed, ...rest } = s.activePlans;
      return { activePlans: rest };
    });
  },

  setLoading: (conversationId, loading) => {
    set((s) => ({ loading: { ...s.loading, [conversationId]: loading } }));
  },

  setError: (conversationId, error) => {
    set((s) => ({ errors: { ...s.errors, [conversationId]: error } }));
  },

  // ── Event Handlers ───────────────────────────────────────────────────

  handlePlanGenerated: (event) => {
    const { conversationId, plan } = event;
    set((s) => ({
      activePlans: { ...s.activePlans, [conversationId]: plan },
      loading: { ...s.loading, [conversationId]: false },
    }));
  },

  handlePlanStepUpdate: (event) => {
    const { conversationId, planId, stepId, status, result } = event;
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    const updatedSteps = plan.steps.map((step) => {
      if (step.id === stepId) {
        return { ...step, status, result: result ?? step.result };
      }
      return step;
    });

    // Determine overall plan status based on step states
    let planStatus = plan.status;
    const hasRunning = updatedSteps.some((s) => s.status === "running");
    const hasError = updatedSteps.some((s) => s.status === "error");
    const allDone = updatedSteps.every(
      (s) => s.status === "completed" || s.status === "rejected",
    );

    if (hasRunning) planStatus = "executing";
    else if (allDone) planStatus = hasError ? "completed" : "completed";

    set((s) => ({
      activePlans: {
        ...s.activePlans,
        [conversationId]: { ...plan, steps: updatedSteps, status: planStatus },
      },
    }));
  },

  handlePlanExecutionComplete: (event) => {
    const { conversationId, planId, status } = event;
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    const updatedPlan = { ...plan, status: status as Plan["status"] };

    // Move to history
    set((s) => {
      const history = s.planHistory[conversationId] || [];
      return {
        activePlans: { ...s.activePlans, [conversationId]: updatedPlan },
        planHistory: {
          ...s.planHistory,
          [conversationId]: [updatedPlan, ...history],
        },
      };
    });
  },

  updatePlanStatus: (conversationId, planId, status) => {
    const plan = get().activePlans[conversationId];
    if (!plan || plan.id !== planId) { return; }

    set((s) => ({
      activePlans: {
        ...s.activePlans,
        [conversationId]: { ...plan, status },
      },
    }));
  },
}));

// ── Event Listener Setup ───────────────────────────────────────────────
// Registered once, persisted across component mounts

let _planUnlisten: UnlistenFn | null = null;
let _planListenersInitialized = false;

export function setupPlanEventListeners(): () => void {
  if (_planListenersInitialized) {
    return () => {}; // Already set up
  }
  _planListenersInitialized = true;

  const unlisteners: UnlistenFn[] = [];

  listen<PlanGeneratedEvent>("plan-generated", (event) => {
    usePlanStore.getState().handlePlanGenerated(event.payload);
  }).then((fn) => unlisteners.push(fn));

  listen<PlanStepUpdateEvent>("plan-step-update", (event) => {
    usePlanStore.getState().handlePlanStepUpdate(event.payload);
  }).then((fn) => unlisteners.push(fn));

  listen<PlanExecutionCompleteEvent>("plan-execution-complete", (event) => {
    usePlanStore.getState().handlePlanExecutionComplete(event.payload);
  }).then((fn) => unlisteners.push(fn));

  _planUnlisten = () => {
    unlisteners.forEach((fn) => fn());
  };

  return _planUnlisten;
}
